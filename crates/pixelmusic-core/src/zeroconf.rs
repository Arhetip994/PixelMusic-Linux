use std::time::Duration;
use mdns_sd::{ServiceDaemon, ServiceEvent};
use serde::{Deserialize, Serialize};

const SPOTIFY_CONNECT_SERVICE: &str = "_spotify-connect._tcp.local.";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectDevice {
    pub name: String,
    pub address: String,
    pub port: u16,
    #[serde(rename = "deviceId")]
    pub device_id: String,
}

pub fn discover_spotify_receivers(timeout: Duration) -> Vec<ConnectDevice> {
    let daemon = match ServiceDaemon::new() {
        Ok(d) => d,
        Err(e) => {
            log::error!("Cannot create mdns daemon: {e}");
            return Vec::new();
        }
    };

    let receiver = match daemon.browse(SPOTIFY_CONNECT_SERVICE) {
        Ok(r) => r,
        Err(e) => {
            log::error!("Cannot browse mdns service: {e}");
            return Vec::new();
        }
    };

    let deadline = std::time::Instant::now() + timeout;
    let mut devices = Vec::new();

    while let Some(remaining) = deadline.checked_duration_since(std::time::Instant::now()) {
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let name = info.get_fullname().trim_end_matches('.').to_string();
                let port = info.get_port();
                let addr = info.get_addresses().iter().next().map(|a| a.to_string()).unwrap_or_default();
                let device_id = info.get_property_val_str("CPath").unwrap_or("").to_string();

                devices.push(ConnectDevice {
                    name,
                    address: addr,
                    port,
                    device_id,
                });
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    let _ = daemon.shutdown();
    devices
}
