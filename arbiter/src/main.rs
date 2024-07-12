use std::{fs::File, io::BufReader};

use coap::Server;
use rcgen::KeyPair;
use rustls::{Certificate as RustlsCertificate, RootCertStore};
use tokio::sync::mpsc::channel;
use webrtc_dtls::{
    config::{ClientAuthType, Config as DtlsConfig},
    crypto::{Certificate, CryptoPrivateKey},
    listener::listen,
};

use self::{config::Config, request_handler::RequestHandler, state::run_state_loop};

mod acl;
mod config;
mod request;
mod request_handler;
mod state;

#[tokio::main]
async fn main() {
    let config = std::fs::read_to_string("config.json").expect("No config file provided");
    let config: Config = serde_json::from_str(&config).expect("Invalid config");

    env_logger::Builder::new()
        .filter_level(config.log_level)
        .init();

    let addr = "127.0.0.1:5683";

    let client_cas = get_root_cert_store(&config.root_ca_file);
    let (certificates, priv_key) = get_my_certs(&config.cert_file, &config.key_file);

    let dtls_config = DtlsConfig {
        certificates,
        client_auth: ClientAuthType::RequireAndVerifyClientCert,
        client_cas,
        server_name: "arbiter.local".into(),
        ..Default::default()
    };

    let (tx, rx) = channel(1000);

    let listener = listen(addr, dtls_config).await.unwrap();
    let listener = Box::new(listener);
    let server = Server::from_listeners(vec![listener]);
    println!("Server up on {addr}");

    let state_handle =
        tokio::spawn(async move { run_state_loop(rx, config.acl, priv_key, config.cid).await });

    server.run(RequestHandler::new(tx)).await.unwrap();

    state_handle.await.unwrap();
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

fn get_my_certs(cert_file: &str, key_file: &str) -> (Vec<Certificate>, KeyPair) {
    let private_key = std::fs::read_to_string(key_file).unwrap();
    let private_key = KeyPair::from_pem(&private_key).unwrap();
    let cert_private_key = CryptoPrivateKey::from_key_pair(&private_key).unwrap();

    let certs: Vec<_> = rustls_pemfile::certs(&mut BufReader::new(File::open(cert_file).unwrap()))
        .map(|cert_result| RustlsCertificate(cert_result.unwrap().to_vec()))
        .collect();

    (
        vec![Certificate {
            certificate: certs,
            private_key: cert_private_key,
        }],
        private_key,
    )
}
