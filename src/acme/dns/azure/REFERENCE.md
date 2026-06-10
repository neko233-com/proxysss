# Azure DNS

- **Provider id:** `azure`
- **Auth:** OAuth2 client credentials → `https://management.azure.com/.default`
- **Upsert TXT:** `PUT .../dnsZones/{zone}/TXT/{relativeName}?api-version=2018-05-01`
- **Delete TXT:** `DELETE` same path
- **Required:** `tenant_id`, `client_id`, `client_secret`, `subscription_id`, `resource_group`

Docs: https://learn.microsoft.com/en-us/rest/api/dns/record-sets/create-or-update
