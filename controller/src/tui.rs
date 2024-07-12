use std::fmt::Display;
use std::net::ToSocketAddrs;
use std::{collections::HashMap, io};

use coap::{
    client::CoAPClient,
    dtls::{DtlsConnection, UdpDtlsConfig},
    request::{Method, RequestBuilder},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use webrtc_dtls::config::Config as DtlsConfig;

const REQUEST_DESTINATION: &str = "127.0.0.1:5683";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Device {
    pub cid: Uuid,
    pub label: String,
    pub manufacturer: String,
    pub model: String,
    pub port: u16,
    pub ttl: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ControlTokenRequest {
    pub cid: Uuid,
    pub devices: Vec<Uuid>,
    pub params_read: Vec<String>,
    pub params_write: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ControlTokenResponse {
    pub tokens: HashMap<Uuid, String>,
}

#[derive(Clone, Copy, PartialEq)]
enum RequestType {
    Get,
    Put,
}

impl Into<Method> for RequestType {
    fn into(self) -> Method {
        match self {
            Self::Get => Method::Get,
            Self::Put => Method::Put,
        }
    }
}

impl From<&str> for RequestType {
    fn from(value: &str) -> Self {
        match value {
            "g" => Self::Get,
            "s" => Self::Put,
            _ => panic!("RequestType From<&str>"),
        }
    }
}

impl Display for RequestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Get => "GET",
                Self::Put => "PUT",
            }
        )
    }
}

#[derive(Serialize)]
struct GetParamPayload {
    token: String,
}

#[derive(Serialize)]
struct SetParamPayload {
    token: String,
    value: String,
}

pub fn run_tui(config: DtlsConfig, my_cid: Uuid, runtime: tokio::runtime::Runtime) {
    println!("NextGen Transport Controller");
    println!("Available commands:");
    println!("  c: Connect to local Arbiter on port 5683 via DTLS");
    println!("  d: Discover devices via local Arbiter");
    println!("  g: Get param value from device");
    println!("      syntax: g [device_index] [parameter]");
    println!("  s: Set param value on device");
    println!("      syntax: s [device_index] [parameter] [value]");
    println!("  p: Print current devices");
    println!("  q: Quit");

    let gs_regex = regex::Regex::new(r"^([gs]) (\d+) (\w+) ([^\s]+)?$").unwrap();

    let mut client: Option<CoAPClient<DtlsConnection>> = None;
    let mut current_devices: Vec<Device> = vec![];

    let stdin = io::stdin();
    for line in stdin.lines() {
        let line = line.unwrap();
        match line.chars().next().unwrap() {
            'q' => break,
            'c' => {
                println!("Connecting to Arbiter...");
                match connect_to_arbiter(config.clone(), &runtime) {
                    Ok(c) => {
                        println!("Connected to Arbiter.");
                        client = Some(c)
                    }
                    Err(e) => {
                        println!("Failed to connect to Arbiter: {:?}", e);
                    }
                };
            }
            'd' => {
                if let Some(ref client) = client {
                    match discover_devices(client, &runtime) {
                        Ok(devices) => {
                            println!("Discovered {} devices", devices.len());
                            let devices: Vec<Device> =
                                devices.into_iter().map(|device| device.into()).collect();
                            print_devices(&devices);
                            current_devices = devices;
                        }
                        Err(e) => {
                            println!("Failed to discover devices: {:?}", e);
                        }
                    }
                } else {
                    println!("Not connected to Arbiter");
                }
            }
            'g' | 's' => {
                let Some(captures) = gs_regex.captures(&line) else {
                    println!("Invalid syntax");
                    continue;
                };

                let request_type: RequestType = captures.get(1).unwrap().as_str().into();
                let device_index = captures.get(2).unwrap().as_str();
                let Ok(device_index) = device_index.parse::<usize>() else {
                    println!("Invalid device index");
                    continue;
                };

                if device_index > current_devices.len() {
                    println!("Invalid device index");
                    continue;
                }

                let parameter = captures.get(3).unwrap().as_str();

                let Some(ref client) = client else {
                    println!("Not connected to Arbiter");
                    continue;
                };

                let device = &current_devices[device_index];

                let token = request_control_token(
                    client,
                    &runtime,
                    &my_cid,
                    device,
                    if request_type == RequestType::Get {
                        vec![parameter.to_string()]
                    } else {
                        vec![]
                    },
                    if request_type == RequestType::Put {
                        vec![parameter.to_string()]
                    } else {
                        vec![]
                    },
                );

                let token = match token {
                    Ok(token) => token,
                    Err(err) => {
                        println!("Failed to get control token: {err}");
                        continue;
                    }
                };

                println!("Got control token for device. Sending {request_type} /{parameter}...",);

                match send_request(
                    config.clone(),
                    &runtime,
                    request_type,
                    device.port,
                    token.tokens.get(&device.cid).unwrap().clone(),
                    parameter,
                    if request_type == RequestType::Put {
                        Some(captures.get(4).unwrap().as_str().to_string())
                    } else {
                        None
                    },
                ) {
                    Ok(Some(result)) => {
                        println!("Got GET result: {result}");
                    }
                    Ok(None) => {
                        println!("SET successfully");
                    }
                    Err(e) => {
                        println!("Failed to execute {request_type} request: {e}");
                    }
                }
            }
            'p' => {
                if current_devices.is_empty() {
                    println!("No devices discovered");
                } else {
                    print_devices(&current_devices)
                }
            }
            _ => {}
        }
    }
}

fn connect_to_arbiter(
    config: DtlsConfig,
    runtime: &tokio::runtime::Runtime,
) -> anyhow::Result<CoAPClient<DtlsConnection>> {
    let config = UdpDtlsConfig {
        config,
        dest_addr: ("127.0.0.1", 5683)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap(),
    };
    Ok(runtime.block_on(async move { CoAPClient::from_udp_dtls_config(config).await })?)
}

fn discover_devices(
    client: &CoAPClient<DtlsConnection>,
    runtime: &tokio::runtime::Runtime,
) -> anyhow::Result<Vec<Device>> {
    let request = RequestBuilder::new("/devices", Method::Get)
        .domain(REQUEST_DESTINATION.to_string())
        .build();

    let response = runtime.block_on(async move { client.send(request).await })?;
    Ok(serde_json::from_slice(&response.message.payload)?)
}

fn print_devices(devices: &Vec<Device>) {
    for (index, device) in devices.iter().enumerate() {
        println!(
            "{}: {} ({}) {} {}",
            index, device.label, device.cid, device.manufacturer, device.model
        );
    }
}

fn request_control_token(
    client: &CoAPClient<DtlsConnection>,
    runtime: &tokio::runtime::Runtime,
    my_cid: &Uuid,
    device: &Device,
    params_read: Vec<String>,
    params_write: Vec<String>,
) -> anyhow::Result<ControlTokenResponse> {
    let payload = ControlTokenRequest {
        cid: my_cid.clone(),
        devices: vec![device.cid],
        params_read,
        params_write,
    };

    let request = RequestBuilder::new("/controlToken", Method::Get)
        .domain(REQUEST_DESTINATION.to_string())
        .data(Some(serde_json::to_vec(&payload)?))
        .build();

    let response = runtime.block_on(async move { client.send(request).await })?;
    Ok(serde_json::from_slice(&response.message.payload)?)
}

fn send_request(
    mut config: DtlsConfig,
    runtime: &tokio::runtime::Runtime,
    request_type: RequestType,
    port: u16,
    token: String,
    parameter: &str,
    value: Option<String>,
) -> anyhow::Result<Option<String>> {
    config.server_name = "device.local".to_string();
    let config = UdpDtlsConfig {
        config,
        dest_addr: ("127.0.0.1", port)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap(),
    };
    let client = runtime.block_on(async move { CoAPClient::from_udp_dtls_config(config).await })?;

    let payload = match request_type {
        RequestType::Get => serde_json::to_vec(&GetParamPayload { token }).unwrap(),
        RequestType::Put => serde_json::to_vec(&SetParamPayload {
            token,
            value: value.unwrap(),
        })
        .unwrap(),
    };

    let request = RequestBuilder::new(&format!("/{parameter}"), request_type.into())
        .domain(format!("127.0.0.1:{port}"))
        .data(Some(payload))
        .build();

    let response = runtime.block_on(async move { client.send(request).await })?;

    match request_type {
        RequestType::Get => Ok(Some(String::from_utf8(response.message.payload)?)),
        RequestType::Put => Ok(None),
    }
}
