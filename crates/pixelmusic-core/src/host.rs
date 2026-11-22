use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;
use serde_json::json;
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use crate::http::{self, respond_json};
use crate::protocol::{Beacon, BEACON_PORT, JamPlayback, RemoteCommand};

pub struct HostSession {
    pub room_id: String,
    pub host_name: String,
    pub port: u16,
    playback: Arc<RwLock<JamPlayback>>,
    version: Arc<watch::Sender<u64>>,
    commands_tx: mpsc::Sender<RemoteCommand>,
    commands_rx: Arc<Mutex<mpsc::Receiver<RemoteCommand>>>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
}

impl HostSession {
    pub async fn start(host_name: String, requested_port: u16) -> anyhow::Result<Arc<Self>> {
        let bind_port = if requested_port == 0 { 58246 } else { requested_port };
        let listener = match TcpListener::bind(("0.0.0.0", bind_port)).await {
            Ok(l) => l,
            Err(_) => TcpListener::bind(("0.0.0.0", 0)).await?,
        };
        let port = listener.local_addr()?.port();
        let room_id = format!("{:06X}", rand::random::<u32>() & 0xFFFFFF);

        let playback = Arc::new(RwLock::new(JamPlayback::default()));
        let (version_tx, _) = watch::channel(1u64);
        let version_tx = Arc::new(version_tx);
        let (commands_tx, commands_rx) = mpsc::channel(100);

        let session = Arc::new(Self {
            room_id: room_id.clone(),
            host_name: host_name.clone(),
            port,
            playback: playback.clone(),
            version: version_tx.clone(),
            commands_tx,
            commands_rx: Arc::new(Mutex::new(commands_rx)),
            tasks: Mutex::new(Vec::new()),
        });

        // 1. UDP Beacon Broadcaster
        let beacon_handle = {
            let room_id = room_id.clone();
            let host_name = host_name.clone();
            tokio::spawn(async move {
                let beacon = Beacon::new(room_id, host_name, port);
                let payload = beacon.to_bytes();
                let socket = match UdpSocket::bind("0.0.0.0:0").await {
                    Ok(s) => {
                        let _ = s.set_broadcast(true);
                        s
                    }
                    Err(e) => {
                        log::error!("UDP beacon bind failed: {e}");
                        return;
                    }
                };

                let targets = [
                    SocketAddr::from(([255, 255, 255, 255], BEACON_PORT)),
                    SocketAddr::from(([10, 0, 2, 2], BEACON_PORT)), // Android emulator host gateway
                    SocketAddr::from(([127, 0, 0, 1], BEACON_PORT)),
                ];
                loop {
                    for target in &targets {
                        let _ = socket.send_to(&payload, *target).await;
                    }
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            })
        };

        // 2. TCP HTTP Server for Remote Control & State Sync
        let server_session = session.clone();
        let server_handle = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((mut stream, _)) => {
                        let sess = server_session.clone();
                        tokio::spawn(async move {
                            if let Some(req) = http::read_request(&mut stream, Duration::from_secs(5)).await {
                                let path = req.path.as_str();
                                match path {
                                    "/join" => {
                                        let pb = sess.playback.read().unwrap().clone();
                                        let ver = *sess.version.borrow();
                                        let reply = json!({
                                            "room": {
                                                "roomId": sess.room_id,
                                                "hostName": sess.host_name,
                                                "port": sess.port
                                            },
                                            "playback": pb,
                                            "version": ver,
                                            "you": { "token": "ok" }
                                        });
                                        let _ = respond_json(&mut stream, "200 OK", &reply.to_string()).await;
                                    }
                                    "/sync" => {
                                        let since: u64 = req.param("since").and_then(|s| s.parse().ok()).unwrap_or(0);
                                        let cur_ver = *sess.version.borrow();
                                        if since >= cur_ver {
                                            // Wait for state change or timeout (3s)
                                            let mut rx = sess.version.subscribe();
                                            let _ = tokio::time::timeout(Duration::from_secs(3), async {
                                                let _ = rx.changed().await;
                                            }).await;
                                        }
                                        let pb = sess.playback.read().unwrap().clone();
                                        let ver = *sess.version.borrow();
                                        let reply = json!({
                                            "playback": pb,
                                            "version": ver
                                        });
                                        let _ = respond_json(&mut stream, "200 OK", &reply.to_string()).await;
                                    }
                                    "/control" => {
                                        let action = req.param("action")
                                            .or_else(|| req.param("type"))
                                            .unwrap_or("")
                                            .to_string();
                                        let value = req.param("value")
                                            .or_else(|| req.param("ms"))
                                            .unwrap_or("")
                                            .to_string();

                                        if !action.is_empty() {
                                            let cmd = RemoteCommand { action, value };
                                            let _ = sess.commands_tx.send(cmd).await;
                                        }

                                        let _ = respond_json(&mut stream, "200 OK", r#"{"status":"ok"}"#).await;
                                    }
                                    "/time" => {
                                        let now = std::time::SystemTime::now()
                                            .duration_since(std::time::UNIX_EPOCH)
                                            .map(|d| d.as_millis() as i64)
                                            .unwrap_or(0);
                                        let reply = json!({ "time": now });
                                        let _ = respond_json(&mut stream, "200 OK", &reply.to_string()).await;
                                    }
                                    _ => {
                                        let _ = respond_json(&mut stream, "404 Not Found", r#"{"error":"not found"}"#).await;
                                    }
                                }
                            }
                        });
                    }
                    Err(e) => {
                        log::error!("TCP accept error: {e}");
                        break;
                    }
                }
            }
        });

        session.tasks.lock().unwrap().extend([beacon_handle, server_handle]);
        Ok(session)
    }

    pub fn update_playback(&self, new_pb: JamPlayback) {
        {
            let mut cur = self.playback.write().unwrap();
            *cur = new_pb;
        }
        self.version.send_modify(|v| *v += 1);
    }

    pub fn poll_commands(&self) -> Vec<RemoteCommand> {
        let mut rx = self.commands_rx.lock().unwrap();
        let mut list = Vec::new();
        while let Ok(cmd) = rx.try_recv() {
            list.push(cmd);
        }
        list
    }

    pub fn stop(&self) {
        let mut tasks = self.tasks.lock().unwrap();
        for t in tasks.drain(..) {
            t.abort();
        }
    }
}
