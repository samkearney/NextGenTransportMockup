use serde::Deserialize;
use uuid::Uuid;

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AclDatabase {
    pub entries: Vec<AclEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AclEntry {
    pub controller_cids: Vec<Uuid>,
    pub device_cids: Vec<Uuid>,
    pub parameters: AclParameters,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AclParameters {
    pub read: Vec<String>,
    pub write: Vec<String>,
}
