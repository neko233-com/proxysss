# Google Cloud DNS

- **Provider id:** `google`
- **Auth:** Service account JWT → `https://oauth2.googleapis.com/token`
- **Scope:** `https://www.googleapis.com/auth/ndev.clouddns.readwrite`
- **Change records:** `POST .../managedZones/{zone}/changes` with `additions` / `deletions`
- **Required:** `service_account_json`; optional `project_id`, `managed_zone`

Docs: https://cloud.google.com/dns/docs/reference/rest/v1/changes/create
