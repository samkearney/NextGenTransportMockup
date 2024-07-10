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

use self::{request_handler::RequestHandler, state::run_state_loop};

mod request;
mod request_handler;
mod state;

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:5683";

    let client_cas = get_root_cert_store();
    let certificates = get_my_certs();

    let config = DtlsConfig {
        certificates,
        client_auth: ClientAuthType::RequireAndVerifyClientCert,
        client_cas,
        server_name: "arbiter.local".into(),
        ..Default::default()
    };

    let (tx, rx) = channel(1000);

    let listener = listen(addr, config).await.unwrap();
    let listener = Box::new(listener);
    let server = Server::from_listeners(vec![listener]);
    println!("Server up on {addr}");

    let state_handle = tokio::spawn(async move { run_state_loop(rx).await });

    server.run(RequestHandler::new(tx)).await.unwrap();

    state_handle.await.unwrap();
}

fn get_root_cert_store() -> RootCertStore {
    let mut store = RootCertStore::empty();
    for cert in rustls_pemfile::certs(&mut BufReader::new(
        File::open("../certs/root-cert.pem").unwrap(),
    )) {
        store
            .add(&RustlsCertificate(cert.unwrap().to_vec()))
            .unwrap();
    }
    store
}

fn get_my_certs() -> Vec<Certificate> {
    let private_key = std::fs::read_to_string("../certs/arbiter-key.pem").unwrap();
    let private_key = KeyPair::from_pem(&private_key).unwrap();
    let private_key = CryptoPrivateKey::from_key_pair(&private_key).unwrap();

    let certs: Vec<_> = rustls_pemfile::certs(&mut BufReader::new(
        File::open("../certs/arbiter-cert.pem").unwrap(),
    ))
    .map(|cert_result| RustlsCertificate(cert_result.unwrap().to_vec()))
    .collect();

    vec![Certificate {
        certificate: certs,
        private_key,
    }]
}
