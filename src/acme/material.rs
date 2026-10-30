//! Recoverable installation of a managed certificate and private key.
//! The temporary journal contains only the newly issued pair, never a backup.

use std::fs::{self, File, OpenOptions};
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::TlsConfig;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    certificate: String,
    private_key: String,
}

fn journal_path(tls: &TlsConfig) -> PathBuf {
    let mut hash = Sha256::new();
    hash.update(tls.cert_path.to_string_lossy().as_bytes());
    hash.update([0]);
    hash.update(tls.key_path.to_string_lossy().as_bytes());
    let digest = hash.finalize();
    let name: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    tls.acme
        .cache_dir
        .join(format!("certificate-{name}.journal"))
}

fn validate_pair(certificate: &str, private_key: &str) -> Result<()> {
    let certs = rustls_pemfile::certs(&mut Cursor::new(certificate.as_bytes()))
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("invalid managed certificate")?;
    if certs.is_empty() {
        return Err(anyhow!("managed certificate is empty"));
    }
    let key = rustls_pemfile::private_key(&mut Cursor::new(private_key.as_bytes()))
        .context("invalid managed private key")?
        .ok_or_else(|| anyhow!("managed private key is empty"))?;
    let signing_key = rustls::crypto::aws_lc_rs::default_provider()
        .key_provider
        .load_private_key(key)
        .map_err(|_| anyhow!("invalid managed private key"))?;
    rustls::sign::CertifiedKey::new(certs, signing_key)
        .keys_match()
        .context("managed certificate and private key do not match")
}

fn open_new(path: &Path, mode: u32) -> Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(mode);
    }
    #[cfg(not(unix))]
    let _ = mode;
    options
        .open(path)
        .with_context(|| format!("failed to create {}", path.display()))
}

fn sync_parent(path: &Path) -> Result<()> {
    #[cfg(unix)]
    File::open(path.parent().unwrap_or_else(|| Path::new(".")))?.sync_all()?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

pub(crate) fn replace_file(path: &Path, contents: &str, mode: u32) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    if fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Err(anyhow!(
            "managed certificate paths must not be symbolic links: {}",
            path.display()
        ));
    }
    let temp = parent.join(format!(".proxysss-cert-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> Result<()> {
        let mut file = open_new(&temp, mode)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path).with_context(|| format!("failed to replace {}", path.display()))?;
        sync_parent(path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Replay a completed journal before loading certificate files. A corrupt
/// journal fails closed; it is never interpreted as configuration or logged.
fn lock(tls: &TlsConfig) -> Result<File> {
    fs::create_dir_all(&tls.acme.cache_dir)?;
    let path = journal_path(tls).with_extension("lock");
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(path)
        .context("failed to open managed certificate lock")?;
    fs2::FileExt::try_lock_exclusive(&file)
        .context("managed certificate installation is already running")?;
    Ok(file)
}

pub fn recover(tls: &TlsConfig) -> Result<bool> {
    let _lock = lock(tls)?;
    recover_locked(tls)
}

fn recover_locked(tls: &TlsConfig) -> Result<bool> {
    let path = journal_path(tls);
    let data = match fs::read(&path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let temporary = path.with_extension("journal.tmp");
            match fs::remove_file(&temporary) {
                Ok(()) => sync_parent(&temporary)?,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(error).context("failed to remove incomplete certificate journal")
                }
            }
            return Ok(false);
        }
        Err(error) => return Err(error).context("failed to read managed certificate journal"),
    };
    let journal: Journal =
        serde_json::from_slice(&data).context("invalid managed certificate journal")?;
    validate_pair(&journal.certificate, &journal.private_key)?;
    replace_file(&tls.cert_path, &journal.certificate, 0o644)?;
    replace_file(&tls.key_path, &journal.private_key, 0o600)?;
    fs::remove_file(&path).context("failed to remove completed certificate journal")?;
    sync_parent(&path)?;
    Ok(true)
}

pub fn install(tls: &TlsConfig, certificate: String, private_key: String) -> Result<()> {
    validate_pair(&certificate, &private_key)?;
    let _lock = lock(tls)?;
    if recover_locked(tls)? {
        tracing::info!("recovered previous managed certificate installation");
    }
    fs::create_dir_all(&tls.acme.cache_dir)?;
    let journal = Journal {
        certificate,
        private_key,
    };
    let bytes = serde_json::to_vec(&journal).context("failed to encode certificate journal")?;
    let path = journal_path(tls);
    let temporary = path.with_extension("journal.tmp");
    let result = (|| -> Result<()> {
        let mut file = open_new(&temporary, 0o600)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &path).context("failed to publish certificate journal")?;
        sync_parent(&path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    recover_locked(tls)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_second_write_recovers_matching_new_pair_without_backup() -> Result<()> {
        let root = crate::test_support::TestDir::new("managed-certificate-journal");
        let mut tls = TlsConfig::default();
        tls.acme.cache_dir = root.join("state");
        tls.cert_path = root.join("cert.pem");
        tls.key_path = root.join("key.pem");
        let old_key = rcgen::KeyPair::generate()?;
        let old_cert =
            rcgen::CertificateParams::new(vec!["example.com".into()])?.self_signed(&old_key)?;
        install(&tls, old_cert.pem(), old_key.serialize_pem())?;
        let held = lock(&tls)?;
        assert!(recover(&tls).is_err());
        drop(held);
        let new_key = rcgen::KeyPair::generate()?;
        let new_cert =
            rcgen::CertificateParams::new(vec!["example.com".into()])?.self_signed(&new_key)?;
        fs::remove_file(&tls.key_path)?;
        fs::create_dir(&tls.key_path)?;
        assert!(install(&tls, new_cert.pem(), new_key.serialize_pem()).is_err());
        assert!(journal_path(&tls).exists());
        fs::remove_dir(&tls.key_path)?;
        assert!(recover(&tls)?);
        assert_eq!(fs::read_to_string(&tls.cert_path)?, new_cert.pem());
        assert_eq!(fs::read_to_string(&tls.key_path)?, new_key.serialize_pem());
        assert!(!journal_path(&tls).exists());
        assert!(!recover(&tls)?);
        let temporary = journal_path(&tls).with_extension("journal.tmp");
        fs::write(&temporary, b"incomplete journal")?;
        assert!(!recover(&tls)?);
        assert!(!temporary.exists());
        assert_eq!(fs::read_to_string(&tls.cert_path)?, new_cert.pem());
        assert_eq!(fs::read_to_string(&tls.key_path)?, new_key.serialize_pem());
        fs::write(journal_path(&tls), b"corrupted journal")?;
        assert!(recover(&tls).is_err());
        assert_eq!(fs::read_to_string(&tls.cert_path)?, new_cert.pem());
        assert_eq!(fs::read_to_string(&tls.key_path)?, new_key.serialize_pem());
        Ok(())
    }
}
