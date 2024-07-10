use std::net::ToSocketAddrs;
use std::{fs::File, io::BufReader};

use coap::client::CoAPClient;
use coap::dtls::UdpDtlsConfig;
use coap::request::{Method, RequestBuilder};
use rustls::{Certificate, RootCertStore};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webrtc_dtls::config::Config as DtlsConfig;

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
    let mut my_store = RootCertStore::empty();

    let certs: Vec<_> = rustls_pemfile::certs(&mut BufReader::new(
        File::open("../certs/root-cert.pem").unwrap(),
    ))
    .map(|cert_result| Certificate(cert_result.unwrap().to_vec()))
    .collect();

    for cert in certs {
        my_store.add(&cert).unwrap();
    }

    let config = DtlsConfig {
        server_name: "arbiter.local".into(),
        roots_cas: my_store,
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
