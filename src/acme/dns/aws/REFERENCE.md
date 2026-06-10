# Amazon Route 53

- **Provider id:** `aws`
- **Host:** `route53.amazonaws.com`
- **Region for signing:** `us-east-1` (Route 53 global endpoint convention)
- **Auth:** AWS SigV4 (`common/aws_sigv4.rs`)
- **Upsert/delete:** `ChangeResourceRecordSets` with XML body
- **Zone lookup:** `ListHostedZonesByName` unless `hosted_zone_id` provided

Docs: https://docs.aws.amazon.com/Route53/latest/APIReference/API_ChangeResourceRecordSets.html
