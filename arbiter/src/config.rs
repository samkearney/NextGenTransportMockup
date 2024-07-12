use log::LevelFilter;
use serde::Deserialize;
use uuid::Uuid;

use crate::acl::AclDatabase;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub cid: Uuid,
    #[serde(default = "default_root_ca")]
    pub root_ca_file: String,
    #[serde(default = "default_cert_file")]
    pub cert_file: String,
    #[serde(default = "default_key_file")]
    pub key_file: String,
    #[serde(default = "default_log_filter")]
    pub log_level: LevelFilter,
    #[serde(default)]
    pub acl: AclDatabase,
}

fn default_root_ca() -> String {
    "../certs/root-cert.pem".to_string()
}

fn default_cert_file() -> String {
    "../certs/arbiter-cert.pem".to_string()
}

fn default_key_file() -> String {
    "../certs/arbiter-key.pem".to_string()
}

fn default_log_filter() -> LevelFilter {
    LevelFilter::Off
}
