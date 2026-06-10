pub mod aliyun_cn;
pub mod aliyun_intl;
pub mod aws;
pub mod azure;
pub mod cloudflare;
pub mod common;
pub mod factory;
pub mod google;
pub mod manual;
pub mod tencent;
pub mod types;
pub mod util;
pub mod volcengine;

pub use factory::{
    is_builtin_dns_provider, list_builtin_dns_provider_ids, normalize_provider_id, DnsProvider,
};
pub use util::acme_challenge_fqdn;
