use serde::Deserialize;
use uuid::Uuid;

#[derive(Default, Deserialize)]
pub struct AclDatabase {
    pub entries: Vec<AclEntry>,
}

#[derive(Deserialize)]
pub struct AclEntry {
    pub controller_cids: Vec<Uuid>,
    pub device_cids: Vec<Uuid>,
    pub parameters: AclParameters,
}

#[derive(Deserialize)]
pub struct AclParameters {
    pub read: Vec<String>,
    pub write: Vec<String>,
}
