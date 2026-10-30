# Volcengine DNS

- **Provider id:** `volcengine`
- **Host:** `dns.volcengineapi.com`
- **Version:** `2018-08-01`
- **Auth:** Volcengine HMAC-SHA256 (`common/volcengine_sigv4.rs`)
- **Actions:** `ListZones`, `CreateRecord`, `DeleteRecord`
- **Default region:** `cn-beijing`

Docs: https://www.volcengine.com/docs/6758/155104

每次挑战创建独立 TXT，清理时仅删除本次返回的 RecordID。根域名与通配符可同时使用同一主机记录，不删除其他验证值或已有业务记录。
