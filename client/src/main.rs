use std::net::ToSocketAddrs;
use std::{fs::File, io::BufReader};

use coap::client::CoAPClient;
use coap::dtls::UdpDtlsConfig;
use coap::request::{Method, RequestBuilder};
use rcgen::KeyPair;
use rustls::{Certificate as RustlsCertificate, RootCertStore};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webrtc_dtls::config::Config as DtlsConfig;
use webrtc_dtls::crypto::{Certificate, CryptoPrivateKey};

#[derive(Serialize)]
struct PutDevicePayload {
    label: String,
    manufacturer: String,
    model: String,
    ttl: u64,
}

#[derive(Debug, Deserialize)]
pub struct ApiDevice {
    pub cid: Uuid,
    pub label: String,
    pub manufacturer: String,
    pub model: String,
    pub ttl: u64,
}

#[tokio::main]
async fn main() {
    let roots_cas = get_root_cert_store();
    let certificates = get_my_certs();

    let config = DtlsConfig {
        certificates,
        server_name: "arbiter.local".into(),
        roots_cas,
        ..Default::default()
    };
    let config = UdpDtlsConfig {
        config,
        dest_addr: ("127.0.0.1", 5683)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap(),
    };

    let my_cid = Uuid::new_v4();

    let client = CoAPClient::from_udp_dtls_config(config).await.unwrap();

    let request = RequestBuilder::new(&format!("/devices/{my_cid}"), Method::Put)
        .domain("127.0.0.1:5683".into())
        .data(Some(
            serde_json::to_vec(&PutDevicePayload {
                label: "My Device".to_string(),
                manufacturer: "My Manufacturer".to_string(),
                model: "My Model".to_string(),
                ttl: 3600,
            })
            .unwrap(),
        ))
        .build();

    println!("Client request: PUT coap://127.0.0.1/devices/{my_cid}");
    let response = client.send(request).await.unwrap();
    println!("Server reply: {:?}", response.get_status().clone(),);

    let request = RequestBuilder::new("/devices", Method::Get)
        .domain("127.0.0.1:5683".into())
        .build();

    println!("Client request: GET coap://127.0.0.1/devices");
    let response = client.send(request).await.unwrap();
    let devices: Vec<ApiDevice> = serde_json::from_slice(&response.message.payload).unwrap();
    println!("Server reply: {:?}", devices);
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
    let private_key = std::fs::read_to_string("../certs/client-key.pem").unwrap();
    let private_key = KeyPair::from_pem(&private_key).unwrap();
    let private_key = CryptoPrivateKey::from_key_pair(&private_key).unwrap();

    let certs: Vec<_> = rustls_pemfile::certs(&mut BufReader::new(
        File::open("../certs/client-cert.pem").unwrap(),
    ))
    .map(|cert_result| RustlsCertificate(cert_result.unwrap().to_vec()))
    .collect();

    vec![Certificate {
        certificate: certs,
        private_key,
    }]
}
