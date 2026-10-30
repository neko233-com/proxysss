//! Cloud certificate deployment control plane. API replies never contain secrets
//! in logs; individual providers are selected by configuration at the call site.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::acme::dns::common::VolcengineSigV4;

const CERT_HOST: &str = "certificate-service.volcengineapi.com";
const DCDN_HOST: &str = "open.volcengineapi.com";

#[derive(Clone, Debug)]
struct DcdnBinding {
    domain: String,
    certificate_id: String,
    status: String,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CleanupState {
    owned_current: Option<String>,
    pending: BTreeSet<String>,
    #[serde(default)]
    in_flight_domains: BTreeSet<String>,
    #[serde(default)]
    successful_key: Option<Vec<u8>>,
}

fn cleanup_state_path(target: &CertificateSyncTarget) -> Result<PathBuf> {
    use sha2::{Digest, Sha256};
    let parent = target
        .cert_path
        .parent()
        .ok_or_else(|| anyhow!("invalid certificate path"))?;
    let mut hash = Sha256::new();
    hash.update(target.cert_path.to_string_lossy().as_bytes());
    hash.update([0]);
    hash.update(target.project.as_bytes());
    hash.update([0]);
    hash.update(target.name.as_bytes());
    let digest = hash.finalize();
    let suffix: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(parent.join(format!(".proxysss-cloud-cleanup-{suffix}.json")))
}

fn valid_certificate_id(id: &str) -> bool {
    id.starts_with("cert-")
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn read_cleanup_state(path: &std::path::Path) -> Result<CleanupState> {
    use std::io::Read as _;
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(CleanupState::default())
        }
        Err(error) => return Err(error).context("cannot read cloud cleanup state"),
    };
    if std::fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err(anyhow!("cloud cleanup state must not be a symbolic link"));
    }
    let mut bytes = Vec::new();
    file.take(64 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 64 * 1024 {
        return Err(anyhow!("cloud cleanup state is too large"));
    }
    let state: CleanupState =
        serde_json::from_slice(&bytes).context("invalid cloud cleanup state")?;
    if state
        .successful_key
        .as_ref()
        .is_some_and(|key| key.len() != 32)
        || state.in_flight_domains.len() > 100
        || state
            .in_flight_domains
            .iter()
            .any(|name| !valid_sync_domain(name))
        || state.pending.len() > 100
        || state.pending.iter().any(|id| !valid_certificate_id(id))
        || state
            .owned_current
            .as_deref()
            .is_some_and(|id| !valid_certificate_id(id))
    {
        return Err(anyhow!("invalid cloud cleanup state"));
    }
    Ok(state)
}

fn save_cleanup_state(path: &std::path::Path, state: &CleanupState) -> Result<()> {
    crate::acme::material::replace_file(path, &serde_json::to_string(state)?, 0o600)
        .context("cannot save cloud cleanup state")
}

fn certificate_covers_domain(names: &[String], domain: &str) -> bool {
    names.iter().any(|name| {
        name.eq_ignore_ascii_case(domain)
            || name.strip_prefix("*.").is_some_and(|suffix| {
                domain.split_once('.').is_some_and(|(left, rest)| {
                    !left.is_empty() && rest.eq_ignore_ascii_case(suffix)
                })
            })
    })
}

fn valid_sync_domain(name: &str) -> bool {
    !name.contains('*')
        && !name.contains(':')
        && !name.ends_with('.')
        && matches!(url::Host::parse(name), Ok(url::Host::Domain(_)))
}

pub struct SyncReceipt {
    pub certificate_id: String,
    pub certificate_names: Vec<String>,
    pub urls: Vec<String>,
}

impl SyncReceipt {
    pub fn notification(&self, target: &str) -> String {
        format!(
            "同步目标：{target}\n证书 ID：{}\n证书域名：{}\n状态：云端已部署，当前出口 TLS 已核验",
            self.certificate_id,
            self.certificate_names.join(", ")
        )
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CertificateSyncConfig {
    #[serde(default)]
    pub targets: Vec<CertificateSyncTarget>,
}

impl CertificateSyncConfig {
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CertificateSyncTarget {
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    pub provider: CertificateSyncProvider,
    pub cert_path: PathBuf,
    pub key_path: PathBuf,
    pub domains: Vec<String>,
    pub access_key_env: String,
    pub secret_key_env: String,
    #[serde(default = "default_project")]
    pub project: String,
    /// Opt in to deleting previously recorded, proxysss-owned certificates after verified replacement.
    #[serde(default)]
    pub cleanup_previous: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CertificateSyncProvider {
    VolcengineDcdn,
    Aliyun,
    Tencent,
    Aws,
    Azure,
}

fn default_project() -> String {
    "default".into()
}

pub fn validate(config: &CertificateSyncConfig, errors: &mut Vec<String>) {
    let mut names = std::collections::HashSet::new();
    if config.targets.len() > 16 {
        errors.push("certificate_sync.targets supports at most 16 targets".into());
    }
    for (index, target) in config.targets.iter().enumerate() {
        if target.enabled && target.provider != CertificateSyncProvider::VolcengineDcdn {
            errors.push(format!(
                "certificate_sync.targets.{index}: this provider is reserved and not implemented"
            ));
        }
        if target.name.trim().is_empty() || !names.insert(target.name.as_str()) {
            errors.push(format!(
                "certificate_sync.targets.{index}.name must be nonempty and unique"
            ));
        }
        if target.cert_path.as_os_str().is_empty()
            || target.key_path.as_os_str().is_empty()
            || target.cert_path == target.key_path
        {
            errors.push(format!(
                "certificate_sync.targets.{index} needs distinct certificate and key paths"
            ));
        }
        if target.domains.is_empty()
            || target.domains.len() > 100
            || target.domains.iter().any(|name| !valid_sync_domain(name))
        {
            errors.push(format!(
                "certificate_sync.targets.{index}.domains must contain 1-100 DNS hostnames"
            ));
        }
        for (field, value) in [
            ("access_key_env", &target.access_key_env),
            ("secret_key_env", &target.secret_key_env),
        ] {
            if value.is_empty()
                || !value.chars().enumerate().all(|(i, c)| {
                    c == '_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
                })
            {
                errors.push(format!(
                    "certificate_sync.targets.{index}.{field} must be an environment variable name"
                ));
            }
        }
        if target.project.trim().is_empty() {
            errors.push(format!(
                "certificate_sync.targets.{index}.project must not be empty"
            ));
        }
    }
}

pub struct VolcengineDcdn {
    http: reqwest::Client,
    certificate_signer: VolcengineSigV4,
    dcdn_signer: VolcengineSigV4,
    certificate_url: String,
    dcdn_url: String,
}

impl VolcengineDcdn {
    pub fn new(access_key: String, secret_key: String) -> Result<Self> {
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .build()
            .context("failed to construct cloud certificate client")?;
        Ok(Self {
            http,
            certificate_signer: VolcengineSigV4::for_service(
                access_key.clone(),
                secret_key.clone(),
                "cn-beijing".into(),
                "certificate_service".into(),
            ),
            dcdn_signer: VolcengineSigV4::for_service(
                access_key,
                secret_key,
                "cn-north-1".into(),
                "dcdn".into(),
            ),
            certificate_url: format!("https://{CERT_HOST}"),
            dcdn_url: format!("https://{DCDN_HOST}"),
        })
    }

    async fn request(
        &self,
        signer: &VolcengineSigV4,
        base: &str,
        host: &str,
        action: &str,
        version: &str,
        payload: &Value,
    ) -> Result<(reqwest::StatusCode, Value)> {
        let query = format!("Action={action}&Version={version}");
        let body = serde_json::to_vec(payload).context("encode cloud request")?;
        let signed = signer.sign_post(host, &query, &body)?;
        let mut response = self
            .http
            .post(format!("{base}/?{query}"))
            .header("Host", host)
            .header("Authorization", signed.authorization)
            .header("X-Date", signed.x_date)
            .header(
                "X-Content-Sha256",
                signed.content_sha256.unwrap_or_default(),
            )
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(|_| anyhow!("cloud {action} request failed"))?;
        let status = response.status();
        let mut data = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| anyhow!("cloud {action} response failed"))?
        {
            if data.len().saturating_add(chunk.len()) > 128 * 1024 {
                return Err(anyhow!("cloud {action} response too large"));
            }
            data.extend_from_slice(&chunk);
        }
        let value: Value = serde_json::from_slice(&data)
            .map_err(|_| anyhow!("cloud {action} returned invalid JSON"))?;
        if !value.get("ResponseMetadata").is_some_and(Value::is_object) {
            return Err(anyhow!("cloud {action} response lacks metadata"));
        }
        Ok((status, value))
    }

    pub async fn import(
        &self,
        project: &str,
        certificate: &str,
        private_key: &str,
    ) -> Result<String> {
        let payload = json!({
            "Tag": "proxysss",
            "ProjectName": project,
            "Repeatable": false,
            "NoVerifyAndFixChain": false,
            "CertificateInfo": {"CertificateChain": certificate, "PrivateKey": private_key}
        });
        let (status, body) = self
            .request(
                &self.certificate_signer,
                &self.certificate_url,
                CERT_HOST,
                "ImportCertificate",
                "2024-10-01",
                &payload,
            )
            .await?;
        import_result(status, &body)
    }

    pub async fn bind(&self, certificate_id: &str, domains: &[String]) -> Result<()> {
        let payload = json!({"CertSource":"volc", "CertId":certificate_id, "DomainNames":domains});
        let (status, body) = self
            .request(
                &self.dcdn_signer,
                &self.dcdn_url,
                DCDN_HOST,
                "CreateCertBind",
                "2021-04-01",
                &payload,
            )
            .await?;
        check_success("CreateCertBind", status, &body)
    }

    async fn managed_certificate(
        &self,
        project: &str,
        certificate_id: &str,
    ) -> Result<Option<bool>> {
        let (status, body) = self.request(
            &self.certificate_signer, &self.certificate_url, CERT_HOST,
            "CertificateGetInstanceList", "2024-10-01",
            &json!({"InstanceIds": [certificate_id], "ProjectName": project, "PageNumber": 1, "PageSize": 10}),
        ).await?;
        check_success("CertificateGetInstanceList", status, &body)?;
        let instances = body
            .pointer("/Result/Instances")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("cloud certificate list is missing instances"))?;
        if instances.len() > 1 {
            return Err(anyhow!("cloud certificate identity is ambiguous"));
        }
        let Some(instance) = instances.first() else {
            return Ok(None);
        };
        if instance.get("InstanceId").and_then(Value::as_str) != Some(certificate_id) {
            return Err(anyhow!("cloud certificate identity does not match"));
        }
        Ok(Some(
            instance.get("Tag").and_then(Value::as_str) == Some("proxysss")
                && instance.get("InstanceType").and_then(Value::as_str) == Some("Imported")
                && instance.get("ProjectName").and_then(Value::as_str) == Some(project),
        ))
    }

    async fn delete_certificate(&self, certificate_id: &str) -> Result<()> {
        let (status, body) = self
            .request(
                &self.certificate_signer,
                &self.certificate_url,
                CERT_HOST,
                "CertificateDeleteInstance",
                "2024-10-01",
                &json!({"InstanceId": certificate_id}),
            )
            .await?;
        if body
            .pointer("/ResponseMetadata/Error/Code")
            .and_then(Value::as_str)
            == Some("NotFound.CertificateInstance")
        {
            return Ok(());
        }
        check_success("CertificateDeleteInstance", status, &body)
    }

    pub async fn binding_status(
        &self,
        project: &str,
        certificate_id: &str,
        domain: &str,
    ) -> Result<Option<String>> {
        Ok(self
            .list_bindings(project, domain)
            .await?
            .into_iter()
            .find(|entry| {
                entry.domain.eq_ignore_ascii_case(domain) && entry.certificate_id == certificate_id
            })
            .map(|entry| entry.status))
    }

    async fn list_bindings(&self, project: &str, search: &str) -> Result<Vec<DcdnBinding>> {
        let payload = if project.is_empty() {
            json!({"SearchKey":search})
        } else {
            json!({"SearchKey":search,"ProjectName":[project]})
        };
        let (status, body) = self
            .request(
                &self.dcdn_signer,
                &self.dcdn_url,
                DCDN_HOST,
                "ListCertBind",
                "2021-04-01",
                &payload,
            )
            .await?;
        check_success("ListCertBind", status, &body)?;
        let bindings = body
            .pointer("/Result/BindList")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("cloud ListCertBind returned no binding list"))?;
        bindings
            .iter()
            .map(|entry| {
                Ok(DcdnBinding {
                    domain: entry
                        .get("DomainName")
                        .and_then(Value::as_str)
                        .ok_or_else(|| anyhow!("cloud binding lacks domain"))?
                        .to_ascii_lowercase(),
                    certificate_id: entry
                        .get("CertId")
                        .and_then(Value::as_str)
                        .ok_or_else(|| anyhow!("cloud binding lacks certificate id"))?
                        .to_owned(),
                    status: entry
                        .get("DeployStatus")
                        .and_then(Value::as_str)
                        .ok_or_else(|| anyhow!("cloud binding lacks deployment status"))?
                        .to_owned(),
                })
            })
            .collect()
    }
}

async fn discover_bound_domains(
    client: &VolcengineDcdn,
    target: &CertificateSyncTarget,
    certificate_names: &[String],
) -> Result<(Vec<String>, BTreeMap<String, String>)> {
    let mut domains = BTreeSet::new();
    let mut old_ids = BTreeSet::new();
    let mut observed = BTreeMap::new();
    for configured in &target.domains {
        let name = configured.to_ascii_lowercase();
        domains.insert(name.clone());
        for binding in client.list_bindings(&target.project, &name).await? {
            if binding.domain == name {
                if binding.certificate_id.is_empty() {
                    return Err(anyhow!("cloud binding has an empty certificate id"));
                }
                if observed
                    .insert(name.clone(), binding.certificate_id.clone())
                    .is_some_and(|previous| previous != binding.certificate_id)
                {
                    return Err(anyhow!("cloud domain has conflicting certificate bindings"));
                }
                old_ids.insert(binding.certificate_id);
            }
        }
    }
    for old_id in old_ids {
        for binding in client.list_bindings(&target.project, &old_id).await? {
            if binding.certificate_id != old_id {
                continue;
            }
            if !valid_sync_domain(&binding.domain)
                || !certificate_covers_domain(certificate_names, &binding.domain)
            {
                return Err(anyhow!(
                    "existing certificate binding is not covered by the new certificate"
                ));
            }
            if observed
                .insert(binding.domain.clone(), old_id.clone())
                .is_some_and(|previous| previous != old_id)
            {
                return Err(anyhow!("cloud domain has conflicting certificate bindings"));
            }
            domains.insert(binding.domain);
            if domains.len() > 100 {
                return Err(anyhow!("cloud certificate has more than 100 bound domains"));
            }
        }
    }
    Ok((domains.into_iter().collect(), observed))
}

fn load_material(target: &CertificateSyncTarget) -> Result<(String, String, Vec<u8>, Vec<String>)> {
    use sha2::{Digest, Sha256};
    use std::io::Cursor;
    use x509_parser::extensions::GeneralName;
    let certificate =
        std::fs::read_to_string(&target.cert_path).context("failed to read sync certificate")?;
    let key =
        std::fs::read_to_string(&target.key_path).context("failed to read sync private key")?;
    let certs = rustls_pemfile::certs(&mut Cursor::new(certificate.as_bytes()))
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("invalid sync certificate")?;
    let leaf = certs
        .first()
        .ok_or_else(|| anyhow!("sync certificate is empty"))?;
    let fingerprint = Sha256::digest(leaf.as_ref()).to_vec();
    let (_, parsed) = x509_parser::parse_x509_certificate(leaf.as_ref())
        .map_err(|_| anyhow!("invalid sync leaf certificate"))?;
    let now = time::OffsetDateTime::now_utc().unix_timestamp();
    if parsed.validity().not_before.timestamp() > now
        || parsed.validity().not_after.timestamp() <= now
    {
        return Err(anyhow!("sync certificate is not currently valid"));
    }
    let sans = parsed
        .subject_alternative_name()
        .map_err(|_| anyhow!("invalid certificate names"))?
        .ok_or_else(|| anyhow!("sync certificate has no subject alternative names"))?;
    let certificate_names = sans
        .value
        .general_names
        .iter()
        .filter_map(|name| match name {
            GeneralName::DNSName(name) => Some((*name).to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for domain in &target.domains {
        if !certificate_covers_domain(&certificate_names, domain) {
            return Err(anyhow!(
                "sync certificate does not cover configured domain {domain}"
            ));
        }
    }
    let private = rustls_pemfile::private_key(&mut Cursor::new(key.as_bytes()))
        .context("invalid sync private key")?
        .ok_or_else(|| anyhow!("sync private key is empty"))?;
    let signing_key = rustls::crypto::aws_lc_rs::default_provider()
        .key_provider
        .load_private_key(private)
        .map_err(|_| anyhow!("invalid sync private key"))?;
    rustls::sign::CertifiedKey::new(certs, signing_key)
        .keys_match()
        .context("sync certificate and private key do not match")?;
    Ok((certificate, key, fingerprint, certificate_names))
}

pub async fn sync_target(target: &CertificateSyncTarget) -> Result<SyncReceipt> {
    let _lock = acquire_lock(target)?;
    let sync_key = change_key(target)?;
    let (certificate, key, fingerprint, certificate_names) = load_material(target)?;
    let access = std::env::var(&target.access_key_env)
        .map_err(|_| anyhow!("cloud access key environment variable is unavailable"))?;
    let secret = std::env::var(&target.secret_key_env)
        .map_err(|_| anyhow!("cloud secret key environment variable is unavailable"))?;
    if access.is_empty() || secret.is_empty() {
        return Err(anyhow!("cloud credential environment variable is empty"));
    }
    match target.provider {
        CertificateSyncProvider::VolcengineDcdn => {
            let client = VolcengineDcdn::new(access, secret)?;
            let path = cleanup_state_path(target)?;
            let mut state = read_cleanup_state(&path)?;
            state.successful_key = None;
            save_cleanup_state(&path, &state)?;
            let id = client.import(&target.project, &certificate, &key).await?;
            let mut discovery_target = target.clone();
            for name in &state.in_flight_domains {
                if !discovery_target.domains.contains(name) {
                    discovery_target.domains.push(name.clone());
                }
            }
            if discovery_target.domains.len() > 100
                || discovery_target
                    .domains
                    .iter()
                    .any(|name| !certificate_covers_domain(&certificate_names, name))
            {
                return Err(anyhow!(
                    "pending cloud domains exceed limits or are not covered by the new certificate"
                ));
            }
            let (domains, observed) =
                discover_bound_domains(&client, &discovery_target, &certificate_names).await?;
            state.in_flight_domains = domains.iter().cloned().collect();
            if target.cleanup_previous {
                if let Some(previous) = state.owned_current.as_ref() {
                    if previous != &id && observed.values().any(|value| value == previous) {
                        state.pending.insert(previous.clone());
                    }
                }
            }
            save_cleanup_state(&path, &state)?;
            let mut unbound = Vec::new();
            for domain in &domains {
                let current: BTreeSet<_> = client
                    .list_bindings(&target.project, domain)
                    .await?
                    .into_iter()
                    .filter(|entry| &entry.domain == domain)
                    .map(|entry| entry.certificate_id)
                    .collect();
                if current.len() > 1 || current.iter().next() != observed.get(domain) {
                    return Err(anyhow!(
                        "cloud binding changed during certificate discovery"
                    ));
                }
                if current.iter().next() != Some(&id) {
                    unbound.push(domain.clone());
                }
            }
            if !unbound.is_empty() {
                client.bind(&id, &unbound).await?;
            }
            for domain in &domains {
                if client
                    .binding_status(&target.project, &id, domain)
                    .await?
                    .as_deref()
                    != Some("Deployed")
                {
                    return Err(anyhow!("cloud deployment is pending for {domain}"));
                }
            }
            let verifier = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(15))
                .tls_info(true)
                .build()
                .context("failed to construct cloud TLS verifier")?;
            for domain in &domains {
                let response = verifier
                    .head(format!("https://{domain}/"))
                    .send()
                    .await
                    .map_err(|_| anyhow!("cloud TLS verification failed for {domain}"))?;
                let peer = response
                    .extensions()
                    .get::<reqwest::tls::TlsInfo>()
                    .and_then(|info| info.peer_certificate())
                    .ok_or_else(|| {
                        anyhow!("cloud TLS verification has no peer certificate for {domain}")
                    })?;
                use sha2::{Digest, Sha256};
                if Sha256::digest(peer).as_slice() != fingerprint {
                    return Err(anyhow!(
                        "cloud TLS certificate has not propagated for {domain}"
                    ));
                }
            }
            let current = std::fs::read_to_string(&target.cert_path)
                .context("failed to re-read sync certificate")?;
            if current != certificate {
                return Err(anyhow!(
                    "source certificate changed during cloud synchronization"
                ));
            }
            if target.cleanup_previous {
                if !valid_certificate_id(&id)
                    || client.managed_certificate(&target.project, &id).await? != Some(true)
                {
                    return Err(anyhow!("new cloud certificate ownership cannot be verified; old certificates retained"));
                }
                state.owned_current = Some(id.clone());
                save_cleanup_state(&path, &state)?;
                for previous in state.pending.clone() {
                    if previous != id {
                        match client
                            .managed_certificate(&target.project, &previous)
                            .await?
                        {
                            None => {}
                            Some(false) => {
                                return Err(anyhow!(
                                    "old certificate ownership cannot be verified; retained"
                                ))
                            }
                            Some(true) => {
                                if client
                                    .list_bindings("", &previous)
                                    .await?
                                    .iter()
                                    .any(|binding| binding.certificate_id == previous)
                                {
                                    return Err(anyhow!(
                                        "old certificate is still bound to a DCDN domain; retained"
                                    ));
                                }
                                // Certificate Center rejects deletion while other cloud resources remain linked.
                                client.delete_certificate(&previous).await?;
                            }
                        }
                    }
                    state.pending.remove(&previous);
                    save_cleanup_state(&path, &state)?;
                }
            }
            state.in_flight_domains.clear();
            if change_key(target)? != sync_key {
                return Err(anyhow!(
                    "certificate synchronization configuration or material changed"
                ));
            }
            state.successful_key = Some(sync_key);
            save_cleanup_state(&path, &state)?;
            Ok(SyncReceipt {
                certificate_id: id,
                certificate_names,
                urls: domains
                    .iter()
                    .map(|domain| format!("https://{domain}/"))
                    .collect(),
            })
        }
        CertificateSyncProvider::Aliyun
        | CertificateSyncProvider::Tencent
        | CertificateSyncProvider::Aws
        | CertificateSyncProvider::Azure => {
            Err(anyhow!("cloud provider is reserved and not implemented"))
        }
    }
}

fn acquire_lock(target: &CertificateSyncTarget) -> Result<std::fs::File> {
    let name = target
        .cert_path
        .file_name()
        .ok_or_else(|| anyhow!("invalid sync certificate path"))?;
    let mut lock_name = name.to_os_string();
    lock_name.push(".proxysss-sync.lock");
    let path = target.cert_path.with_file_name(lock_name);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .context("failed to open certificate synchronization lock")?;
    fs2::FileExt::try_lock_exclusive(&file)
        .context("certificate synchronization already running or lock unavailable")?;
    Ok(file)
}

/// A control-plane change key; no credential values are included or logged.
pub fn change_key(target: &CertificateSyncTarget) -> Result<Vec<u8>> {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(serde_json::to_vec(target)?);
    hash.update(std::fs::read(&target.cert_path).context("failed to read sync certificate")?);
    hash.update(std::fs::read(&target.key_path).context("failed to read sync private key")?);
    Ok(hash.finalize().to_vec())
}

/// Only fully verified deployments with no unfinished cleanup survive a restart.
pub fn already_synchronized(target: &CertificateSyncTarget, key: &[u8]) -> Result<bool> {
    let state = read_cleanup_state(&cleanup_state_path(target)?)?;
    Ok(state.successful_key.as_deref() == Some(key)
        && state.in_flight_domains.is_empty()
        && state.pending.is_empty())
}

pub async fn sync_config(
    config: &CertificateSyncConfig,
    notifications: &crate::config::NotificationConfig,
    selected: &[String],
) -> Result<()> {
    for name in selected {
        if !config
            .targets
            .iter()
            .any(|target| target.enabled && &target.name == name)
        {
            return Err(anyhow!(
                "enabled certificate synchronization target not found: {name}"
            ));
        }
    }
    let mut failed = false;
    for target in config
        .targets
        .iter()
        .filter(|target| target.enabled && (selected.is_empty() || selected.contains(&target.name)))
    {
        match sync_target(target).await {
            Ok(receipt) => {
                println!(
                    "certificate sync {}: deployed and TLS verified; certificate id: {}",
                    target.name, receipt.certificate_id
                );
                crate::notifications::dispatch_with_links(
                    notifications,
                    crate::notifications::NotificationEvent::CloudSyncSucceeded,
                    &receipt.notification(&target.name),
                    &receipt.urls,
                )
                .await;
            }
            Err(error) => {
                eprintln!("certificate sync {}: {error}", target.name);
                crate::notifications::dispatch(
                    notifications,
                    crate::notifications::NotificationEvent::CloudSyncFailed,
                    &format!(
                        "proxysss 云证书同步未完成：{}。请查看网关日志。",
                        target.name
                    ),
                )
                .await;
                failed = true;
            }
        }
    }
    if failed {
        return Err(anyhow!(
            "one or more certificate sync targets failed or are pending"
        ));
    }
    Ok(())
}

fn check_success(action: &str, status: reqwest::StatusCode, body: &Value) -> Result<()> {
    if !status.is_success()
        || body
            .pointer("/ResponseMetadata/Error")
            .is_some_and(|error| !error.is_null())
    {
        return Err(anyhow!("cloud {action} failed (HTTP {})", status.as_u16()));
    }
    Ok(())
}

fn import_result(status: reqwest::StatusCode, body: &Value) -> Result<String> {
    // Repeatable=false intentionally permits a previously imported certificate to
    // be reused after retries/restarts. Never print the provider response body.
    if let Some(id) = body
        .pointer("/Result/RepeatId")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        return Ok(id.to_owned());
    }
    check_success("ImportCertificate", status, body)?;
    body.pointer("/Result/InstanceId")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("cloud ImportCertificate returned no certificate id"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_is_idempotent_and_errors_are_not_reported_as_success() {
        let ok = json!({"ResponseMetadata":{}, "Result":{"InstanceId":"cert-new"}});
        assert_eq!(
            import_result(reqwest::StatusCode::OK, &ok).unwrap(),
            "cert-new"
        );
        let duplicate = json!({"ResponseMetadata":{"Error":{"Code":"Duplicate"}}, "Result":{"RepeatId":"cert-existing"}});
        assert_eq!(
            import_result(reqwest::StatusCode::BAD_REQUEST, &duplicate).unwrap(),
            "cert-existing"
        );
        let failure = json!({"ResponseMetadata":{"Error":{"Code":"Denied"}}, "Result":{}});
        assert!(import_result(reqwest::StatusCode::OK, &failure).is_err());
        assert!(import_result(reqwest::StatusCode::BAD_REQUEST, &failure).is_err());
    }

    #[test]
    fn configuration_rejects_invalid_targets_and_secret_values() -> Result<()> {
        let mut target: CertificateSyncTarget = serde_yaml::from_str(
            r#"
name: example
enabled: true
provider: volcengine_dcdn
cert_path: example.crt
key_path: example.key
domains: [cdn.example.com]
access_key_env: CLOUD_ACCESS
secret_key_env: CLOUD_SECRET
"#,
        )?;
        let mut config = CertificateSyncConfig {
            targets: vec![target.clone()],
        };
        let mut errors = Vec::new();
        validate(&config, &mut errors);
        assert!(errors.is_empty());
        target.secret_key_env = "secret-value".into();
        target.domains = vec!["*.example.com".into()];
        config.targets.push(target);
        validate(&config, &mut errors);
        assert!(errors.iter().any(|error| error.contains("name must")));
        assert!(errors.iter().any(|error| error.contains("secret_key_env")));
        assert!(errors.iter().any(|error| error.contains("domains")));
        Ok(())
    }

    #[test]
    fn reserved_cloud_strategies_cannot_be_enabled_as_if_implemented() -> Result<()> {
        let mut target: CertificateSyncTarget = serde_yaml::from_str(
            "name: future\nenabled: false\nprovider: aliyun\ncert_path: example.crt\nkey_path: example.key\ndomains: [cdn.example.com]\naccess_key_env: CLOUD_ACCESS\nsecret_key_env: CLOUD_SECRET\n",
        )?;
        for provider in [
            CertificateSyncProvider::Aliyun,
            CertificateSyncProvider::Tencent,
            CertificateSyncProvider::Aws,
            CertificateSyncProvider::Azure,
        ] {
            target.provider = provider;
            target.enabled = false;
            let mut errors = Vec::new();
            validate(
                &CertificateSyncConfig {
                    targets: vec![target.clone()],
                },
                &mut errors,
            );
            assert!(errors.is_empty());
            target.enabled = true;
            validate(
                &CertificateSyncConfig {
                    targets: vec![target.clone()],
                },
                &mut errors,
            );
            assert!(errors.iter().any(|error| error.contains("not implemented")));
        }
        Ok(())
    }

    #[test]
    fn cleanup_state_is_durable_bounded_and_scoped_to_target() -> Result<()> {
        let root = crate::test_support::TestDir::new("cloud-cleanup-state");
        let yaml = "name: example\nenabled: true\nprovider: volcengine_dcdn\ncert_path: example.crt\nkey_path: example.key\ndomains: [www.example.com]\naccess_key_env: TEST_ACCESS\nsecret_key_env: TEST_SECRET\ncleanup_previous: true\n";
        let mut target: CertificateSyncTarget = serde_yaml::from_str(yaml)?;
        target.cert_path = root.join("example.crt");
        target.key_path = root.join("example.key");
        std::fs::write(&target.cert_path, b"certificate-a")?;
        std::fs::write(&target.key_path, b"key-a")?;
        let first_key = change_key(&target)?;
        std::fs::write(&target.key_path, b"key-b")?;
        assert_ne!(change_key(&target)?, first_key);
        std::fs::write(&target.cert_path, b"certificate-b")?;
        assert_ne!(change_key(&target)?, first_key);
        let path = cleanup_state_path(&target)?;
        let state = CleanupState {
            owned_current: Some("cert-old".into()),
            pending: ["cert-older".into()].into_iter().collect(),
            in_flight_domains: ["cdn.example.com".into()].into_iter().collect(),
            successful_key: Some(vec![7; 32]),
        };
        save_cleanup_state(&path, &state)?;
        let restored = read_cleanup_state(&path)?;
        assert_eq!(restored.owned_current.as_deref(), Some("cert-old"));
        assert!(restored.pending.contains("cert-older"));
        assert!(restored.in_flight_domains.contains("cdn.example.com"));
        assert!(!already_synchronized(&target, &[7; 32])?);
        let completed = CleanupState {
            successful_key: Some(vec![7; 32]),
            ..Default::default()
        };
        save_cleanup_state(&path, &completed)?;
        assert!(already_synchronized(&target, &[7; 32])?);
        assert!(!already_synchronized(&target, &[8; 32])?);
        target.name = "another-target".into();
        assert_ne!(cleanup_state_path(&target)?, path);
        std::fs::write(&path, b"{invalid")?;
        assert!(read_cleanup_state(&path).is_err());
        std::fs::write(&path, vec![b'x'; 64 * 1024 + 1])?;
        assert!(read_cleanup_state(&path).is_err());
        Ok(())
    }

    #[tokio::test]
    async fn selected_sync_rejects_unknown_targets_before_any_cloud_request() {
        assert!(sync_config(
            &CertificateSyncConfig::default(),
            &crate::config::NotificationConfig::default(),
            &["unknown".into()]
        )
        .await
        .is_err());
    }

    #[tokio::test]
    async fn dcdn_discovers_only_domains_using_the_same_certificate() -> Result<()> {
        use bytes::Bytes;
        use http_body_util::{BodyExt, Full};
        use hyper::{service::service_fn, Request, Response};
        use hyper_util::rt::TokioIo;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let server = tokio::spawn(async move {
            for _ in 0..4 {
                let (stream, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let service = service_fn(
                        |request: Request<hyper::body::Incoming>| async move {
                            let body = request.into_body().collect().await.unwrap().to_bytes();
                            let payload: Value = serde_json::from_slice(&body).unwrap();
                            let bindings = if payload["SearchKey"] == "one.example.com" {
                                json!([{ "DomainName": "one.example.com", "CertId": "old", "DeployStatus": "Deployed" }])
                            } else {
                                assert_eq!(payload["SearchKey"], "old");
                                json!([
                                    { "DomainName": "one.example.com", "CertId": "old", "DeployStatus": "Deployed" },
                                    { "DomainName": "two.example.com", "CertId": "old", "DeployStatus": "Deployed" },
                                    { "DomainName": "other.example.com", "CertId": "unrelated", "DeployStatus": "Deployed" }
                                ])
                            };
                            Ok::<_, std::convert::Infallible>(Response::new(Full::new(Bytes::from(
                            json!({"ResponseMetadata": {}, "Result": {"BindList": bindings}}).to_string(),
                        ))))
                        },
                    );
                    let _ = hyper::server::conn::http1::Builder::new()
                        .keep_alive(false)
                        .serve_connection(TokioIo::new(stream), service)
                        .await;
                });
            }
        });
        let mut client = VolcengineDcdn::new("test-ak".into(), "test-sk".into())?;
        client.dcdn_url = format!("http://{address}");
        let target: CertificateSyncTarget = serde_yaml::from_str(
            "name: test\nenabled: true\nprovider: volcengine_dcdn\ncert_path: a\nkey_path: b\ndomains: [one.example.com]\naccess_key_env: TEST_ACCESS\nsecret_key_env: TEST_SECRET\n",
        )?;
        assert_eq!(
            discover_bound_domains(&client, &target, &["*.example.com".into()])
                .await?
                .0,
            vec!["one.example.com", "two.example.com"]
        );
        assert!(
            discover_bound_domains(&client, &target, &["one.example.com".into()])
                .await
                .is_err()
        );
        server.await?;
        assert!(!certificate_covers_domain(
            &["*.example.com".into()],
            "deep.two.example.com"
        ));
        Ok(())
    }

    #[test]
    fn source_certificate_must_cover_domains_and_match_private_key() -> Result<()> {
        let root = crate::test_support::TestDir::new("certificate-sync-material");
        let key = rcgen::KeyPair::generate()?;
        let cert =
            rcgen::CertificateParams::new(vec!["example.com".into(), "*.example.com".into()])?
                .self_signed(&key)?;
        let cert_path = root.join("cert.pem");
        let key_path = root.join("key.pem");
        std::fs::write(&cert_path, cert.pem())?;
        std::fs::write(&key_path, key.serialize_pem())?;
        let mut target = CertificateSyncTarget {
            name: "test".into(),
            enabled: true,
            provider: CertificateSyncProvider::VolcengineDcdn,
            cert_path,
            key_path,
            domains: vec!["www.example.com".into()],
            access_key_env: "TEST_ACCESS".into(),
            secret_key_env: "TEST_SECRET".into(),
            project: "default".into(),
            cleanup_previous: false,
        };
        let (_, _, _, names) = load_material(&target)?;
        assert!(names.iter().any(|name| name == "*.example.com"));
        let lock = acquire_lock(&target)?;
        assert!(acquire_lock(&target).is_err());
        drop(lock);
        drop(acquire_lock(&target)?);
        target.domains = vec!["a.b.example.com".into()];
        assert!(load_material(&target).is_err());
        target.domains = vec!["www.example.com".into()];
        std::fs::write(
            &target.key_path,
            rcgen::KeyPair::generate()?.serialize_pem(),
        )?;
        assert!(load_material(&target).is_err());
        Ok(())
    }

    #[tokio::test]
    async fn volcengine_import_bind_and_status_use_separate_signed_actions() -> Result<()> {
        use bytes::Bytes;
        use http_body_util::{BodyExt, Full};
        use hyper::{service::service_fn, Request, Response};
        use hyper_util::rt::TokioIo;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let server = tokio::spawn(async move {
            for _ in 0..3 {
                let (stream, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let service = service_fn(
                        |request: Request<hyper::body::Incoming>| async move {
                            let query = request.uri().query().unwrap_or("").to_owned();
                            let authorization = request
                                .headers()
                                .get("authorization")
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_owned();
                            let body = request.into_body().collect().await.unwrap().to_bytes();
                            let payload: Value = serde_json::from_slice(&body).unwrap();
                            let result = if query.contains("Action=ImportCertificate") {
                                assert!(authorization.contains("/certificate_service/request"));
                                assert_eq!(payload["Repeatable"], false);
                                assert_eq!(payload["CertificateInfo"]["PrivateKey"], "test-key");
                                json!({"ResponseMetadata":{},"Result":{"RepeatId":"cert-existing"}})
                            } else if query.contains("Action=CreateCertBind") {
                                assert!(authorization.contains("/dcdn/request"));
                                assert_eq!(payload["CertId"], "cert-existing");
                                json!({"ResponseMetadata":{},"Result":{}})
                            } else {
                                assert!(query.contains("Action=ListCertBind"));
                                json!({"ResponseMetadata":{},"Result":{"BindList":[{"DomainName":"www.example.com","CertId":"cert-existing","DeployStatus":"Deployed"}]}})
                            };
                            Ok::<_, std::convert::Infallible>(Response::new(Full::new(
                                Bytes::from(result.to_string()),
                            )))
                        },
                    );
                    let _ = hyper::server::conn::http1::Builder::new()
                        .keep_alive(false)
                        .serve_connection(TokioIo::new(stream), service)
                        .await;
                });
            }
        });
        let mut client = VolcengineDcdn::new("test-ak".into(), "test-sk".into())?;
        client.certificate_url = format!("http://{address}");
        client.dcdn_url = format!("http://{address}");
        let id = client.import("default", "test-cert", "test-key").await?;
        client.bind(&id, &["www.example.com".into()]).await?;
        assert_eq!(
            client
                .binding_status("default", &id, "www.example.com")
                .await?
                .as_deref(),
            Some("Deployed")
        );
        server.await?;
        Ok(())
    }
}
