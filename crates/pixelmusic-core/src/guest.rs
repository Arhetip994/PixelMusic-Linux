use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;

use crate::http::send_request;
use crate::protocol::{Beacon, BEACON_PORT, DiscoveredDevice, JamPlayback};

pub struct DiscoveryManager {
    devices: Arc<RwLock<HashMap<String, (DiscoveredDevice, Instant)>>>,
    task: Mutex<Option<JoinHandle<()>>>,
}

impl DiscoveryManager {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            task: Mutex::new(None),
        }
    }

    pub fn start(&self) -> bool {
        let mut task_guard = self.task.lock().unwrap();
        if task_guard.is_some() {
            return true;
        }

        let devices = self.devices.clone();
        let handle = crate::get_runtime().spawn(async move {
            let socket = match UdpSocket::bind(("0.0.0.0", BEACON_PORT)).await {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("Cannot bind to UDP {BEACON_PORT}: {e}, retrying on ephemeral port...");
                    match UdpSocket::bind("0.0.0.0:0").await {
                        Ok(s) => s,
                        Err(e) => {
                            log::error!("Cannot bind UDP discovery socket: {e}");
                            return;
                        }
                    }
                }
            };

            let mut buf = [0u8; 2048];
            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((n, sender)) => {
                        if let Some(beacon) = Beacon::from_bytes(&buf[..n]) {
                            let ip = sender.ip().to_string();
                            let key = format!("{}:{}", ip, beacon.port);
                            let dev = DiscoveredDevice {
                                name: beacon.host_name,
                                ip,
                                port: beacon.port,
                                room_id: beacon.room_id,
                                is_host: true,
                            };
                            let mut map = devices.write().unwrap();
                            map.insert(key, (dev, Instant::now()));
                        }
                    }
                    Err(e) => {
                        log::error!("UDP recv error: {e}");
                        tokio::time::sleep(Duration::from_millis(500)).await;
                    }
                }
            }
        });

        *task_guard = Some(handle);
        true
    }

    pub fn get_devices(&self) -> Vec<DiscoveredDevice> {
        let mut map = self.devices.write().unwrap();
        let now = Instant::now();
        // Retain only devices seen within the last 15 seconds
        map.retain(|_, (_, seen)| now.duration_since(*seen) < Duration::from_secs(15));
        map.values().map(|(d, _)| d.clone()).collect()
    }

    pub fn stop(&self) {
        let mut task_guard = self.task.lock().unwrap();
        if let Some(t) = task_guard.take() {
            t.abort();
        }
    }
}

pub struct GuestClient {
    target_addr: SocketAddr,
    #[allow(dead_code)]
    guest_name: String,
    #[allow(dead_code)]
    device_type: String,
    playback: Arc<RwLock<JamPlayback>>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl GuestClient {
    pub async fn connect(ip: &str, port: u16, guest_name: &str, device_type: &str) -> anyhow::Result<Arc<Self>> {
        let target_addr: SocketAddr = format!("{ip}:{port}").parse()?;
        let query = format!("/join?name={}&device={}", urlencoding::encode(guest_name), urlencoding::encode(device_type));
        
        let (status, body) = send_request(target_addr, "POST", &query, "", Duration::from_secs(4)).await?;
        if status != 200 {
            anyhow::bail!("Host returned HTTP {status}");
        }

        let playback_init: JamPlayback = serde_json::from_str(&body)
            .ok()
            .and_then(|v: serde_json::Value| v.get("playback").cloned())
            .and_then(|pb| serde_json::from_value(pb).ok())
            .unwrap_or_default();

        let playback = Arc::new(RwLock::new(playback_init));
        let client = Arc::new(Self {
            target_addr,
            guest_name: guest_name.to_string(),
            device_type: device_type.to_string(),
            playback: playback.clone(),
            tasks: Mutex::new(Vec::new()),
        });

        // Background loop to poll sync
        let poll_client = client.clone();
        let sync_task = crate::get_runtime().spawn(async move {
            let mut since = 0u64;
            loop {
                let path = format!("/sync?since={since}");
                if let Ok((status, body)) = send_request(poll_client.target_addr, "GET", &path, "", Duration::from_secs(5)).await {
                    if status == 200 {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                            if let Some(v_num) = v.get("version").and_then(|val| val.as_u64()) {
                                since = v_num;
                            }
                            if let Some(pb_val) = v.get("playback") {
                                if let Ok(new_pb) = serde_json::from_value::<JamPlayback>(pb_val.clone()) {
                                    *poll_client.playback.write().unwrap() = new_pb;
                                }
                            }
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });

        client.tasks.lock().unwrap().push(sync_task);
        Ok(client)
    }

    pub async fn send_command(&self, action: &str, value: &str) -> anyhow::Result<()> {
        let path = format!(
            "/control?action={}&value={}",
            urlencoding::encode(action),
            urlencoding::encode(value)
        );
        let (status, _) = send_request(self.target_addr, "POST", &path, "", Duration::from_secs(3)).await?;
        if status != 200 {
            anyhow::bail!("Command failed with HTTP {status}");
        }
        Ok(())
    }

    pub fn get_playback(&self) -> JamPlayback {
        self.playback.read().unwrap().clone()
    }

    pub fn disconnect(&self) {
        let mut tasks = self.tasks.lock().unwrap();
        for t in tasks.drain(..) {
            t.abort();
        }
    }
}
