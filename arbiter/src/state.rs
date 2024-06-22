use std::{
    collections::{hash_map::Entry, HashMap},
    time::Instant,
};

use coap_lite::error::HandlingError;
use tokio::sync::mpsc::Receiver;
use uuid::Uuid;

use crate::request::{ApiDevice, ListResponse, Request, RequestType, Response};

struct Device {
    label: String,
    manufacturer: String,
    model: String,
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

pub async fn run_state_loop(mut channel: Receiver<Request>) {
    let mut state = State::new();

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
                ttl: device.valid_until.duration_since(Instant::now()).as_secs(),
            })
            .collect(),
    }
}
