---
name: acme-dns-providers
description: >-
  Built-in proxysss ACME DNS-01 provider strategies. Use when adding, fixing,
  or configuring wildcard SSL via http.tls.acme.challenge=dns01 and
  http.tls.acme.dns.provider for cloudflare, aliyun_cn, aliyun_intl, tencent,
  volcengine, aws, azure, or google.
---

# ACME DNS-01 Provider Strategies

proxysss implements DNS-01 inside the binary. **No acme.sh, no external ACME client.**

## Product boundary

| Scenario | Config | External deps |
| --- | --- | --- |
| Single-host / public HTTP reachable | `http.tls.auto_https` or `acme_managed` + `challenge: http01` / `tls_alpn01` | None |
| Wildcard / DNS-only validation | `acme_managed` + `challenge: dns01` + `dns.provider` | Cloud DNS API credentials only |

One cloud vendor = one strategy subdirectory under `src/acme/dns/{provider}/`.

**Alibaba Cloud China (`aliyun_cn`) and International (`aliyun_intl`) are separate strategies** — different API endpoints, never merge.

## Code layout

```
src/acme/dns/
  factory.rs          # registry + DnsProvider enum
  common/             # shared signing/oauth helpers
  cloudflare/
  aliyun_cn/
  aliyun_intl/
  tencent/
  volcengine/
  aws/
  azure/
  google/
```

Each provider folder should contain:

- `mod.rs` — re-export
- `client.rs` — API calls only for that vendor
- `REFERENCE.md` — official API doc links + credential keys (keep updated)

## Provider IDs and credentials

| provider | credentials (YAML keys) | Official API |
| --- | --- | --- |
| `cloudflare` | `api_token` or `api_key`+`email` | https://developers.cloudflare.com/api/resources/dns/subresources/records/methods/create/ |
| `aliyun_cn` | `access_key_id`, `access_key_secret` | https://www.alibabacloud.com/help/en/dns/ |
| `aliyun_intl` | `access_key_id`, `access_key_secret` | https://www.alibabacloud.com/help/en/dns/ (intl endpoint) |
| `tencent` | `api_token`, `api_secret` or `login_token` | https://cloud.tencent.com/document/product/1427 |
| `volcengine` | `access_key_id`, `secret_access_key`, optional `region` | https://www.volcengine.com/docs/6758/155104 |
| `aws` | `access_key_id`, `secret_access_key`, optional `hosted_zone_id` | https://docs.aws.amazon.com/Route53/latest/APIReference/API_ChangeResourceRecordSets.html |
| `azure` | `tenant_id`, `client_id`, `client_secret`, `subscription_id`, `resource_group` | https://learn.microsoft.com/en-us/rest/api/dns/record-sets/create-or-update |
| `google` | `service_account_json`, optional `project_id`, optional `managed_zone` | https://cloud.google.com/dns/docs/reference/rest/v1/changes/create |

Legacy acme.sh names (`dns_cf`, `dns_ali`, `dns_aws`, …) normalize in `factory.rs`.

## Config template

```yaml
http:
  tls:
    mode: acme_managed
    acme:
      email: admin@example.com
      challenge: dns01
      domains: [example.com, "*.example.com"]
      directory_production: true
      dns:
        provider: cloudflare
        credentials:
          api_token: "***"
```

## Implementation rules

1. TXT name is always `_acme-challenge.{apex}` — use `util::acme_challenge_fqdn`.
2. Upsert = delete existing matching TXT + create new (avoid stale validation).
3. Always clean up TXT records after ACME order completes (`gateway.rs`).
4. Do not add business logic to providers; they only publish/delete DNS TXT.
5. When adding a provider: new subdirectory, register in `BUILTIN_DNS_PROVIDERS` + `DnsProvider` enum, add alias mapping, add config test, update docs/skills.

## Verification without live credentials

- Unit tests: factory alias normalization, fqdn parsing, signing helpers.
- Config validation tests: `config.rs` accepts valid credential shapes.
- Do not require live API keys in CI.

Read the provider's `REFERENCE.md` before editing its `client.rs`.
