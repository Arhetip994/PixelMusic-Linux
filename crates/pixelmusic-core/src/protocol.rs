use serde::{Deserialize, Serialize};

pub const BEACON_PORT: u16 = 58245;
pub const BEACON_MAGIC: &str = "soundwave_jam";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Beacon {
    pub app: String,
    #[serde(rename = "roomId")]
    pub room_id: String,
    #[serde(rename = "hostName")]
    pub host_name: String,
    pub port: u16,
}

impl Beacon {
    pub fn new(room_id: String, host_name: String, port: u16) -> Self {
        Self {
            app: BEACON_MAGIC.to_string(),
            room_id,
            host_name,
            port,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let text = std::str::from_utf8(bytes).ok()?.trim();
        if !text.starts_with('{') || !text.contains(BEACON_MAGIC) {
            return None;
        }
        let beacon: Beacon = serde_json::from_str(text).ok()?;
        if beacon.app != BEACON_MAGIC || beacon.port == 0 {
            return None;
        }
        Some(beacon)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct JamPlayback {
    #[serde(rename = "trackId", default)]
    pub track_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub artist: String,
    #[serde(rename = "artworkUrl", default)]
    pub artwork_url: String,
    #[serde(default)]
    pub source: String,
    #[serde(rename = "isPlaying", default)]
    pub is_playing: bool,
    #[serde(rename = "positionMs", default)]
    pub position_ms: i64,
    #[serde(rename = "serverTimestamp", default)]
    pub server_timestamp: i64,
    #[serde(rename = "hostClockMs", default)]
    pub host_clock_ms: i64,
    #[serde(rename = "durationMs", default)]
    pub duration_ms: i64,
    #[serde(rename = "youtubeId", skip_serializing_if = "Option::is_none")]
    pub youtube_id: Option<String>,
}

impl Default for JamPlayback {
    fn default() -> Self {
        Self {
            track_id: String::new(),
            title: String::new(),
            artist: String::new(),
            artwork_url: String::new(),
            source: "YOUTUBE".to_string(),
            is_playing: false,
            position_ms: 0,
            server_timestamp: 0,
            host_clock_ms: 0,
            duration_ms: 0,
            youtube_id: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteCommand {
    pub action: String,
    #[serde(default)]
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub name: String,
    pub ip: String,
    pub port: u16,
    #[serde(rename = "roomId")]
    pub room_id: String,
    #[serde(rename = "isHost")]
    pub is_host: bool,
}
