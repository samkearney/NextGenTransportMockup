use std::net::SocketAddr;

use coap_lite::{error::HandlingError, CoapRequest};
use serde::Serialize;
use tokio::sync::oneshot::Sender as OneshotSender;
use uuid::Uuid;

pub struct Request {
    ty: RequestType,
    notify: Option<OneshotSender<Response>>,
}

impl Request {
    pub fn synchronous(ty: RequestType, notify: OneshotSender<Response>) -> Self {
        Self {
            ty,
            notify: Some(notify),
        }
    }

    pub fn asynchronous(ty: RequestType) -> Self {
        Self { ty, notify: None }
    }

    pub fn get_type(&self) -> &RequestType {
        &self.ty
    }

    pub fn respond(self, response: Response) -> Result<(), Response> {
        if let Some(notify) = self.notify {
            notify.send(response)
        } else {
            Err(response)
        }
    }
}

pub enum RequestType {
    Register(ApiDevice),
    List,
    Shutdown,
}

#[derive(Debug, Serialize)]
pub struct ApiDevice {
    pub cid: Uuid,
    pub label: String,
    pub manufacturer: String,
    pub model: String,
    pub ttl: u64,
}

pub enum Response {
    Ok,
    ListResponse(ListResponse),
    Error(HandlingError),
}

pub struct ListResponse {
    pub devices: Vec<ApiDevice>,
}

impl Response {
    pub fn into_coap_response(self, message: &mut CoapRequest<SocketAddr>) {
        let resp = message
            .response
            .as_mut()
            .expect("into_coap_response() called with a request that has no response");

        match self {
            Response::Ok => {}
            Response::ListResponse(list) => {
                resp.message.payload = serde_json::to_vec(&list.devices).unwrap();
            }
            Response::Error(e) => {
                message.apply_from_error(e);
            }
        }
    }
}
