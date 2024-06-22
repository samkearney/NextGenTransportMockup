use coap::UdpCoAPClient;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    let my_cid = Uuid::new_v4();
    let put_url = format!("coap://127.0.0.1/devices/{my_cid}");
    println!("Client request: {put_url}");

    let response = UdpCoAPClient::put(
        &put_url,
        serde_json::to_vec(&PutDevicePayload {
            label: "My Device".to_string(),
            manufacturer: "My Manufacturer".to_string(),
            model: "My Model".to_string(),
            ttl: 3600,
        })
        .expect("Failed to serialize payload"),
    )
    .await
    .unwrap();

    println!("Server reply: {:?}", response.get_status().clone(),);

    let get_url = "coap://127.0.0.1/devices";
    let response = UdpCoAPClient::get(&get_url).await.unwrap();
    let devices: Vec<ApiDevice> = serde_json::from_slice(&response.message.payload).unwrap();
    println!("Server reply: {:?}", devices);
}
