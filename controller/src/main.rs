use std::{fs::File, io::BufReader};

use rcgen::KeyPair;
use rustls::{Certificate as RustlsCertificate, RootCertStore};
use webrtc_dtls::config::Config as DtlsConfig;
use webrtc_dtls::crypto::{Certificate, CryptoPrivateKey};

use self::config::Config;

mod config;
mod tui;

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    let config = std::fs::read_to_string("config.json").expect("No config file provided");
    let config: Config = serde_json::from_str(&config).expect("Invalid config");

    env_logger::Builder::new()
        .filter_level(config.log_level)
        .init();

    let roots_cas = get_root_cert_store(&config.root_ca_file);
    let certificates = get_my_certs(&config.cert_file, &config.key_file);
    let my_cid = config.cid;

    let config = DtlsConfig {
        certificates,
        server_name: "arbiter.local".into(),
        roots_cas,
        ..Default::default()
    };

    // It is recommended to use a normal thread for stdin reads
    // https://docs.rs/tokio/latest/tokio/io/struct.Stdin.html
    tui::run_tui(config, my_cid, runtime);
}

fn get_root_cert_store(cert_file: &str) -> RootCertStore {
    let mut store = RootCertStore::empty();
    for cert in rustls_pemfile::certs(&mut BufReader::new(File::open(cert_file).unwrap())) {
        store
            .add(&RustlsCertificate(cert.unwrap().to_vec()))
            .unwrap();
    }
    store
}

fn get_my_certs(cert_file: &str, key_file: &str) -> Vec<Certificate> {
    let private_key = std::fs::read_to_string(key_file).unwrap();
    let private_key = KeyPair::from_pem(&private_key).unwrap();
    let private_key = CryptoPrivateKey::from_key_pair(&private_key).unwrap();

    let certs: Vec<_> = rustls_pemfile::certs(&mut BufReader::new(File::open(cert_file).unwrap()))
        .map(|cert_result| RustlsCertificate(cert_result.unwrap().to_vec()))
        .collect();

    vec![Certificate {
        certificate: certs,
        private_key,
    }]
}
