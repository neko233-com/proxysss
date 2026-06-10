# Cloudflare DNS-01

- **Provider id:** `cloudflare`
- **API base:** `https://api.cloudflare.com/client/v4`
- **Auth:** Bearer `api_token` (preferred) or `X-Auth-Email` + `X-Auth-Key`
- **Create TXT:** `POST /zones/{zone_id}/dns_records`
- **Delete TXT:** `DELETE /zones/{zone_id}/dns_records/{record_id}`
- **Zone lookup:** `GET /zones?name={zone}`

Docs: https://developers.cloudflare.com/api/resources/dns/subresources/records/methods/create/
