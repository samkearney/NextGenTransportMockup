use std::net::{SocketAddr, ToSocketAddrs};
use std::{fs::File, io::BufReader};

use coap::client::CoAPClient;
use coap::dtls::UdpDtlsConfig;
use coap::request::{CoapRequest, Method, RequestBuilder};
use coap::Server;
use coap_lite::error::HandlingError;
use coap_lite::ResponseType;
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation};
use rcgen::KeyPair;
use rustls::{Certificate as RustlsCertificate, RootCertStore};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webrtc_dtls::config::{ClientAuthType, Config as DtlsConfig};
use webrtc_dtls::crypto::{Certificate, CryptoPrivateKey};
use webrtc_dtls::listener::listen;
use webrtc_util::conn::Listener;

use self::config::Config;

mod config;

#[derive(Serialize)]
struct PutDevicePayload {
    label: String,
    manufacturer: String,
    model: String,
    port: u16,
    ttl: u64,
}

#[derive(Deserialize)]
struct GetParamPayload {
    token: String,
}

#[derive(Deserialize)]
struct SetParamPayload {
    token: String,
    value: String,
}

#[derive(Serialize, Deserialize)]
struct JwtClaims {
    iss: String,
    sub: String,
    aud: String,
    exp: u64,
    params_read: Vec<String>,
    params_write: Vec<String>,
}

struct RequestHandler {
    jwt_decoder: DecodingKey,
    my_cid: Uuid,
}

impl RequestHandler {
    pub fn new(jwt_decoder: DecodingKey, my_cid: Uuid) -> Self {
        Self {
            jwt_decoder,
            my_cid,
        }
    }
}

impl coap::server::RequestHandler for RequestHandler {
    fn handle_request<'life0, 'async_trait>(
        &'life0 self,
        mut request: Box<CoapRequest<SocketAddr>>,
    ) -> core::pin::Pin<
        Box<
            dyn core::future::Future<Output = Box<CoapRequest<SocketAddr>>>
                + core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async {
            let method = request.get_method();
            match method {
                Method::Get => {
                    let parameter = request.get_path();
                    println!("Handling GET /{}", parameter);

                    let payload =
                        match serde_json::from_slice::<GetParamPayload>(&request.message.payload) {
                            Ok(payload) => payload,
                            Err(e) => {
                                request.apply_from_error(HandlingError::bad_request(format!(
                                    "Couldn't parse payload of GET /: {e}"
                                )));
                                return request;
                            }
                        };

                    let jwt_data = match decode_jwt(
                        &payload.token,
                        &self.jwt_decoder,
                        &self.my_cid.to_string(),
                    ) {
                        Ok(data) => data,
                        Err(e) => {
                            println!("Error decoding control token: {e}");
                            request.apply_from_error(HandlingError::bad_request(format!(
                                "Couldn't decode JWT: {e}"
                            )));
                            return request;
                        }
                    };

                    println!(
                        "Received token: {}",
                        serde_json::to_string_pretty(&jwt_data.claims).unwrap()
                    );

                    if !jwt_data.claims.params_read.contains(&parameter) {
                        println!("Validation error: Token does not have permission to access parameter {parameter}");
                        request.apply_from_error(HandlingError::with_code(
                            ResponseType::Forbidden,
                            "No permission for parameter",
                        ));
                    } else {
                        println!("Get request validated successfully.");
                        if let Some(ref mut message) = request.response {
                            message.message.payload = b"42".to_vec();
                        }
                    }
                }
                Method::Put => {
                    let parameter = request.get_path();
                    println!("Handling PUT /{}", parameter);

                    let payload =
                        match serde_json::from_slice::<SetParamPayload>(&request.message.payload) {
                            Ok(payload) => payload,
                            Err(e) => {
                                request.apply_from_error(HandlingError::bad_request(format!(
                                    "Couldn't parse payload of SET /: {e}"
                                )));
                                return request;
                            }
                        };

                    let jwt_data = match decode_jwt(
                        &payload.token,
                        &self.jwt_decoder,
                        &self.my_cid.to_string(),
                    ) {
                        Ok(data) => data,
                        Err(e) => {
                            request.apply_from_error(HandlingError::bad_request(format!(
                                "Couldn't decode JWT: {e}"
                            )));
                            return request;
                        }
                    };

                    println!(
                        "Received token: {}",
                        serde_json::to_string_pretty(&jwt_data.claims).unwrap()
                    );

                    if !jwt_data.claims.params_write.contains(&parameter) {
                        println!("Validation error: Token does not have permission to write parameter {parameter}");
                        request.apply_from_error(HandlingError::with_code(
                            ResponseType::Forbidden,
                            "No permission for parameter",
                        ));
                    } else {
                        println!("Put request validated successfully.");
                        println!("Setting {parameter} to {}", payload.value);
                        if let Some(ref mut message) = request.response {
                            message.message.payload.clear();
                        }
                    }
                }
                _ => println!("Received unhandled method {:?}", method),
            }

            return request;
        })
    }
}

#[tokio::main]
async fn main() {
    let config = std::fs::read_to_string("config.json").expect("No config file provided");
    let config: Config = serde_json::from_str(&config).expect("Invalid config");

    env_logger::Builder::new()
        .filter_level(config.log_level)
        .init();

    let roots_cas = get_root_cert_store(&config.root_ca_file);
    let certificates = get_my_certs(&config.cert_file, &config.key_file);
    let jwt_decoder = get_jwt_decoder(&config.arbiter_public_key_file);

    let server_config = DtlsConfig {
        certificates: certificates.clone(),
        client_auth: ClientAuthType::RequireAndVerifyClientCert,
        client_cas: roots_cas.clone(),
        ..Default::default()
    };

    let listener = listen("127.0.0.1:0", server_config).await.unwrap();
    let port = listener.addr().await.unwrap().port();
    let listener = Box::new(listener);
    let server = Server::from_listeners(vec![listener]);
    println!("Server up on port {port}");

    register_with_arbiter(&config, port, certificates, roots_cas).await;

    server
        .run(RequestHandler::new(jwt_decoder, config.cid))
        .await
        .unwrap();
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

fn get_jwt_decoder(public_key_file: &str) -> DecodingKey {
    let public_key = std::fs::read(public_key_file).unwrap();
    DecodingKey::from_ec_pem(&public_key).unwrap()
}

fn decode_jwt(
    token: &str,
    decoder: &DecodingKey,
    my_cid: &str,
) -> anyhow::Result<TokenData<JwtClaims>> {
    let mut validation = Validation::new(Algorithm::ES256);
    validation.set_audience(&[my_cid]);

    Ok(jsonwebtoken::decode::<JwtClaims>(
        token,
        decoder,
        &validation,
    )?)
}

async fn register_with_arbiter(
    config: &Config,
    port: u16,
    certificates: Vec<Certificate>,
    roots_cas: RootCertStore,
) {
    let dtls_config = DtlsConfig {
        certificates,
        server_name: "arbiter.local".into(),
        roots_cas,
        ..Default::default()
    };
    let client_config = UdpDtlsConfig {
        config: dtls_config,
        dest_addr: ("127.0.0.1", 5683)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap(),
    };

    // Register with the Arbiter
    let request = RequestBuilder::new(&format!("/devices/{}", config.cid), Method::Put)
        .domain("127.0.0.1:5683".into())
        .data(Some(
            serde_json::to_vec(&PutDevicePayload {
                label: config.label.clone(),
                manufacturer: config.manufacturer.clone(),
                model: config.model.clone(),
                port,
                ttl: 3600,
            })
            .unwrap(),
        ))
        .build();

    let client = CoAPClient::from_udp_dtls_config(client_config)
        .await
        .unwrap();

    println!("Registering device {} with arbiter...", config.cid);
    let response = client.send(request).await.unwrap();
    println!("Server reply: {:?}", response.get_status().clone());
}
