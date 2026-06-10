#[derive(Debug, Clone)]
pub struct DnsRecordHandle {
    #[allow(dead_code)]
    pub provider: String,
    pub record_id: String,
    pub zone: String,
    pub name: String,
}
