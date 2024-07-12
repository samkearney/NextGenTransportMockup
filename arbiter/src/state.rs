use std::{
    collections::{hash_map::Entry, HashMap},
    time::{self, Instant},
};

use coap_lite::error::HandlingError;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use rcgen::KeyPair;
use serde::Serialize;
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

use crate::{
    acl::AclDatabase,
    request::{
        ApiDevice, ControlTokenRequest, ControlTokenResponse, ListResponse, Request, RequestType,
        Response,
    },
};

struct Device {
    label: String,
    manufacturer: String,
    model: String,
    port: u16,
    valid_until: Instant,
}

struct State {
    devices: HashMap<Uuid, Device>,
}

impl State {
    fn new() -> Self {
        State {
            devices: HashMap::new(),
        }
    }
}

pub async fn run_state_loop(
    mut channel: Receiver<Request>,
    acl: AclDatabase,
    private_key: KeyPair,
    my_cid: Uuid,
) {
    let mut state = State::new();
    let jwt_key = EncodingKey::from_ec_der(&private_key.serialize_der());

    while let Some(request) = channel.recv().await {
        let response = match request.get_type() {
            RequestType::Register(request) => {
                println!("Register request received: {:?}", request);

                match register_device(&mut state, request) {
                    Ok(()) => Response::Ok,
                    Err(e) => Response::Error(HandlingError::bad_request(e)),
                }
            }
            RequestType::List => Response::ListResponse(list_devices(&state)),
            RequestType::ControlToken(request) => {
                println!("Control token request received from {}", request.cid);
                match get_control_token(request, &acl, &jwt_key, &my_cid) {
                    Ok(token) => Response::ControlTokenResponse(token),
                    Err(e) => Response::Error(HandlingError::bad_request(e)),
                }
            }
            RequestType::Shutdown => Response::Ok,
        };

        let _ = request.respond(response);
    }
}

fn register_device(state: &mut State, device: &ApiDevice) -> anyhow::Result<()> {
    match state.devices.entry(device.cid) {
        Entry::Occupied(_) => Err(anyhow::anyhow!("A device with this CID already exists")),
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(Device {
                label: device.label.clone(),
                manufacturer: device.manufacturer.clone(),
                model: device.model.clone(),
                port: device.port,
                valid_until: Instant::now() + std::time::Duration::from_secs(device.ttl),
            });
            Ok(())
        }
    }
}

fn list_devices(state: &State) -> ListResponse {
    ListResponse {
        devices: state
            .devices
            .iter()
            .map(|(cid, device)| ApiDevice {
                cid: *cid,
                label: device.label.clone(),
                manufacturer: device.manufacturer.clone(),
                model: device.model.clone(),
                port: device.port,
                ttl: device.valid_until.duration_since(Instant::now()).as_secs(),
            })
            .collect(),
    }
}

#[derive(Serialize)]
struct JwtClaims {
    iss: String,
    sub: String,
    aud: String,
    exp: u64,
    params_read: Vec<String>,
    params_write: Vec<String>,
}

fn get_control_token(
    request: &ControlTokenRequest,
    acl: &AclDatabase,
    jwt_key: &EncodingKey,
    arb_cid: &Uuid,
) -> anyhow::Result<ControlTokenResponse> {
    // TODO: Validate with ACL

    let header = Header::new(Algorithm::ES256);
    let mut response = ControlTokenResponse {
        tokens: Default::default(),
    };

    for device in &request.devices {
        let claims = JwtClaims {
            iss: arb_cid.to_string(),
            sub: request.cid.to_string(),
            aud: device.to_string(),
            exp: (time::SystemTime::now() + time::Duration::from_secs(6000))
                .duration_since(time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            params_read: request.params_read.clone(),
            params_write: request.params_write.clone(),
        };

        let token = jsonwebtoken::encode(&header, &claims, jwt_key)?;
        response.tokens.insert(device.clone(), token);
        println!(
            "Generating token: {}",
            serde_json::to_string_pretty(&claims).unwrap()
        );
    }

    Ok(response)
}
