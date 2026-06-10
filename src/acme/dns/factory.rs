use std::collections::BTreeMap;

use anyhow::{anyhow, Result};

use super::aliyun_cn::AliyunCnDns;
use super::aliyun_intl::AliyunIntlDns;
use super::aws::AwsRoute53Dns;
use super::azure::AzureDns;
use super::cloudflare::CloudflareDns;
use super::google::GoogleCloudDns;
use super::manual::ManualDns;
use super::tencent::TencentDns;
use super::types::DnsRecordHandle;
use super::volcengine::VolcengineDns;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DnsProviderDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub credential_keys: &'static [&'static str],
}

pub const BUILTIN_DNS_PROVIDERS: &[DnsProviderDescriptor] = &[
    DnsProviderDescriptor {
        id: "cloudflare",
        display_name: "Cloudflare",
        credential_keys: &["api_token", "CF_Token", "api_key+email"],
    },
    DnsProviderDescriptor {
        id: "aliyun_cn",
        display_name: "Alibaba Cloud DNS (China)",
        credential_keys: &["access_key_id", "access_key_secret"],
    },
    DnsProviderDescriptor {
        id: "aliyun_intl",
        display_name: "Alibaba Cloud DNS (International)",
        credential_keys: &["access_key_id", "access_key_secret"],
    },
    DnsProviderDescriptor {
        id: "tencent",
        display_name: "Tencent Cloud DNSPod",
        credential_keys: &["api_token+api_secret", "login_token"],
    },
    DnsProviderDescriptor {
        id: "volcengine",
        display_name: "Volcengine DNS",
        credential_keys: &["access_key_id", "secret_access_key", "region"],
    },
    DnsProviderDescriptor {
        id: "aws",
        display_name: "Amazon Route 53",
        credential_keys: &["access_key_id", "secret_access_key", "hosted_zone_id?"],
    },
    DnsProviderDescriptor {
        id: "azure",
        display_name: "Azure DNS",
        credential_keys: &[
            "tenant_id",
            "client_id",
            "client_secret",
            "subscription_id",
            "resource_group",
        ],
    },
    DnsProviderDescriptor {
        id: "google",
        display_name: "Google Cloud DNS",
        credential_keys: &["project_id", "managed_zone?", "service_account_json"],
    },
    DnsProviderDescriptor {
        id: "manual",
        display_name: "Manual DNS-01 (no API key)",
        credential_keys: &["timeout_secs?", "poll_interval_secs?"],
    },
];

pub fn normalize_provider_id(provider: &str) -> String {
    match provider.trim().to_ascii_lowercase().as_str() {
        "cloudflare" | "dns_cf" | "cf" => "cloudflare".to_string(),
        "aliyun_cn" | "aliyun" | "ali" | "dns_ali" | "alicloud" | "alibaba_cloud_cn" => {
            "aliyun_cn".to_string()
        }
        "aliyun_intl" | "aliyun_global" | "alicloud_intl" | "dns_aliyun" | "alibaba_cloud_intl" => {
            "aliyun_intl".to_string()
        }
        "tencent" | "dnspod" | "dns_dp" | "dns_tencent" => "tencent".to_string(),
        "volcengine" | "volc" | "byteplus" | "dns_volcengine" => "volcengine".to_string(),
        "aws" | "route53" | "dns_aws" | "amazon" => "aws".to_string(),
        "azure" | "azuredns" | "dns_azure" | "dns_azuredns" => "azure".to_string(),
        "google" | "gcloud" | "gcp" | "google_cloud" | "dns_gcloud" | "dns_google" => {
            "google".to_string()
        }
        "manual" | "dns_manual" | "hand" => "manual".to_string(),
        other => other.to_string(),
    }
}

pub fn is_builtin_dns_provider(provider: &str) -> bool {
    let normalized = normalize_provider_id(provider);
    BUILTIN_DNS_PROVIDERS
        .iter()
        .any(|descriptor| descriptor.id == normalized)
}

pub fn list_builtin_dns_provider_ids() -> Vec<&'static str> {
    BUILTIN_DNS_PROVIDERS
        .iter()
        .map(|descriptor| descriptor.id)
        .collect()
}

pub enum DnsProvider {
    Cloudflare(CloudflareDns),
    AliyunCn(AliyunCnDns),
    AliyunIntl(AliyunIntlDns),
    Tencent(TencentDns),
    Volcengine(VolcengineDns),
    Aws(AwsRoute53Dns),
    Azure(AzureDns),
    Google(GoogleCloudDns),
    Manual(ManualDns),
}

impl DnsProvider {
    pub fn create(provider: &str, credentials: BTreeMap<String, String>) -> Result<Self> {
        match normalize_provider_id(provider).as_str() {
            "cloudflare" => Ok(Self::Cloudflare(CloudflareDns::new(&credentials)?)),
            "aliyun_cn" => Ok(Self::AliyunCn(AliyunCnDns::new(&credentials)?)),
            "aliyun_intl" => Ok(Self::AliyunIntl(AliyunIntlDns::new(&credentials)?)),
            "tencent" => Ok(Self::Tencent(TencentDns::new(&credentials)?)),
            "volcengine" => Ok(Self::Volcengine(VolcengineDns::new(&credentials)?)),
            "aws" => Ok(Self::Aws(AwsRoute53Dns::new(&credentials)?)),
            "azure" => Ok(Self::Azure(AzureDns::new(&credentials)?)),
            "google" => Ok(Self::Google(GoogleCloudDns::new(&credentials)?)),
            "manual" => Ok(Self::Manual(ManualDns::new(&credentials)?)),
            unknown => Err(anyhow!(
                "unsupported built-in DNS provider {unknown}; supported: {}",
                list_builtin_dns_provider_ids().join(", ")
            )),
        }
    }

    pub fn id(&self) -> &'static str {
        match self {
            Self::Cloudflare(_) => "cloudflare",
            Self::AliyunCn(_) => "aliyun_cn",
            Self::AliyunIntl(_) => "aliyun_intl",
            Self::Tencent(_) => "tencent",
            Self::Volcengine(_) => "volcengine",
            Self::Aws(_) => "aws",
            Self::Azure(_) => "azure",
            Self::Google(_) => "google",
            Self::Manual(_) => "manual",
        }
    }

    pub async fn upsert_txt_record(&self, fqdn: &str, value: &str) -> Result<DnsRecordHandle> {
        match self {
            Self::Cloudflare(provider) => provider.upsert_txt_record(fqdn, value).await,
            Self::AliyunCn(provider) => provider.upsert_txt_record(fqdn, value).await,
            Self::AliyunIntl(provider) => provider.upsert_txt_record(fqdn, value).await,
            Self::Tencent(provider) => provider.upsert_txt_record(fqdn, value).await,
            Self::Volcengine(provider) => provider.upsert_txt_record(fqdn, value).await,
            Self::Aws(provider) => provider.upsert_txt_record(fqdn, value).await,
            Self::Azure(provider) => provider.upsert_txt_record(fqdn, value).await,
            Self::Google(provider) => provider.upsert_txt_record(fqdn, value).await,
            Self::Manual(provider) => provider.upsert_txt_record(fqdn, value).await,
        }
    }

    pub async fn delete_txt_record(&self, handle: &DnsRecordHandle) -> Result<()> {
        match self {
            Self::Cloudflare(provider) => provider.delete_txt_record(handle).await,
            Self::AliyunCn(provider) => provider.delete_txt_record(handle).await,
            Self::AliyunIntl(provider) => provider.delete_txt_record(handle).await,
            Self::Tencent(provider) => provider.delete_txt_record(handle).await,
            Self::Volcengine(provider) => provider.delete_txt_record(handle).await,
            Self::Aws(provider) => provider.delete_txt_record(handle).await,
            Self::Azure(provider) => provider.delete_txt_record(handle).await,
            Self::Google(provider) => provider.delete_txt_record(handle).await,
            Self::Manual(provider) => provider.delete_txt_record(handle).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_provider_id_maps_acme_sh_aliases() {
        assert_eq!(normalize_provider_id("dns_cf"), "cloudflare");
        assert_eq!(normalize_provider_id("dns_ali"), "aliyun_cn");
        assert_eq!(normalize_provider_id("dns_aliyun"), "aliyun_intl");
        assert_eq!(normalize_provider_id("dns_dp"), "tencent");
        assert_eq!(normalize_provider_id("dns_aws"), "aws");
        assert_eq!(normalize_provider_id("dns_azuredns"), "azure");
        assert_eq!(normalize_provider_id("dns_gcloud"), "google");
        assert_eq!(normalize_provider_id("dns_volcengine"), "volcengine");
    }

    #[test]
    fn aliyun_cn_and_intl_are_distinct_providers() {
        assert_ne!(
            normalize_provider_id("aliyun_cn"),
            normalize_provider_id("aliyun_intl")
        );
    }

    #[test]
    fn lists_all_nine_builtin_providers() {
        assert_eq!(list_builtin_dns_provider_ids().len(), 9);
        assert!(is_builtin_dns_provider("manual"));
    }
}
