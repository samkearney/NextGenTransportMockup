use std::net::SocketAddr;

use coap::request::{CoapRequest, Method};
use coap_lite::error::HandlingError;
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot::channel as oneshot_channel;

use crate::request::{ApiDevice, ControlTokenRequest, Request, RequestType};

pub struct RequestHandler {
    tx: Sender<Request>,
}

impl RequestHandler {
    pub fn new(tx: Sender<Request>) -> Self {
        RequestHandler { tx }
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
            // We are not handling any Acknowledgment or Reset messages
            if !request.response.is_some() {
                return request;
            };

            match request.get_method() {
                &Method::Get => println!("handling: GET /{}", request.get_path()),
                &Method::Post => println!("handling: POST /{}", request.get_path(),),
                &Method::Put => println!("handling: PUT /{}", request.get_path()),
                _ => println!("Ignoring request with unknown method"),
            };

            let path = request.get_path_as_vec().unwrap();

            let req = match (
                request.get_method(),
                path.iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .as_slice(),
            ) {
                (&Method::Get, &["devices"]) => RequestType::List,
                (&Method::Put, &["devices", id]) => {
                    let payload = match serde_json::from_slice::<PutDevicePayload>(
                        &request.message.payload,
                    ) {
                        Ok(payload) => payload,
                        Err(e) => {
                            request.apply_from_error(HandlingError::bad_request(format!(
                                "Couldn't parse payload of PUT /devices/{id}: {e}"
                            )));
                            return request;
                        }
                    };

                    RequestType::Register(ApiDevice {
                        cid: id.parse().unwrap(),
                        label: payload.label,
                        manufacturer: payload.manufacturer,
                        model: payload.model,
                        port: payload.port,
                        ttl: payload.ttl,
                    })
                }
                (&Method::Get, &["controlToken"]) => {
                    let payload = match serde_json::from_slice::<ControlTokenRequest>(
                        &request.message.payload,
                    ) {
                        Ok(payload) => payload,
                        Err(e) => {
                            request.apply_from_error(HandlingError::bad_request(format!(
                                "Couldn't parse payload of GET /controlToken: {e}"
                            )));
                            return request;
                        }
                    };

                    RequestType::ControlToken(payload)
                }
                (_, _) => {
                    request.apply_from_error(HandlingError::not_found());
                    return request;
                }
            };

            let (resp_tx, resp_rx) = oneshot_channel();
            self.tx
                .send(Request::synchronous(req, resp_tx))
                .await
                .unwrap();
            let resp = resp_rx.await.unwrap();

            resp.into_coap_response(&mut request);

            request
        })
    }
}

impl Drop for RequestHandler {
    fn drop(&mut self) {
        let _ = self.tx.send(Request::asynchronous(RequestType::Shutdown));
    }
}

#[derive(Deserialize)]
struct PutDevicePayload {
    label: String,
    manufacturer: String,
    model: String,
    port: u16,
    ttl: u64,
}
