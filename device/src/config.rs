use log::LevelFilter;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub cid: Uuid,
    pub label: String,
    pub manufacturer: String,
    pub model: String,
    #[serde(default = "default_root_ca")]
    pub root_ca_file: String,
    #[serde(default = "default_cert_file")]
    pub cert_file: String,
    #[serde(default = "default_key_file")]
    pub key_file: String,
    #[serde(default = "default_arbiter_public_key_file")]
    pub arbiter_public_key_file: String,
    #[serde(default = "default_log_filter")]
    pub log_level: LevelFilter,
}

fn default_root_ca() -> String {
    "../certs/root-cert.pem".to_string()
}

fn default_cert_file() -> String {
    "../certs/device-cert.pem".to_string()
}

fn default_key_file() -> String {
    "../certs/device-key.pem".to_string()
}

fn default_arbiter_public_key_file() -> String {
    "../certs/arbiter-key.pub.pem".to_string()
}

fn default_log_filter() -> LevelFilter {
    LevelFilter::Off
}
