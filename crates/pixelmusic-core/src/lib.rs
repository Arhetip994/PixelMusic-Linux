pub mod guest;
pub mod host;
pub mod http;
pub mod protocol;
pub mod zeroconf;

use std::sync::{Arc, OnceLock, RwLock};
use std::time::Duration;

use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jint, jlong, jstring, JNI_FALSE, JNI_TRUE};
use tokio::runtime::Runtime;

use crate::guest::{DiscoveryManager, GuestClient};
use crate::host::HostSession;
use crate::protocol::JamPlayback;

static RUNTIME: OnceLock<Runtime> = OnceLock::new();
static HOST_SESSION: RwLock<Option<Arc<HostSession>>> = RwLock::new(None);
static DISCOVERY_MGR: OnceLock<DiscoveryManager> = OnceLock::new();
static GUEST_CLIENT: RwLock<Option<Arc<GuestClient>>> = RwLock::new(None);

pub fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    })
}

pub fn get_discovery() -> &'static DiscoveryManager {
    DISCOVERY_MGR.get_or_init(DiscoveryManager::new)
}

pub fn start_discovery() -> bool {
    get_discovery().start()
}

pub fn get_discovered_devices() -> Vec<crate::protocol::DiscoveredDevice> {
    get_discovery().get_devices()
}

pub fn connect_guest(ip: &str, port: u16, guest_name: &str, device_type: &str) -> anyhow::Result<Arc<GuestClient>> {
    let rt = get_runtime();
    let client = rt.block_on(async {
        GuestClient::connect(ip, port, guest_name, device_type).await
    })?;
    let mut guard = GUEST_CLIENT.write().unwrap();
    if let Some(old) = guard.take() {
        old.disconnect();
    }
    *guard = Some(client.clone());
    Ok(client)
}

pub fn get_guest_client() -> Option<Arc<GuestClient>> {
    GUEST_CLIENT.read().unwrap().clone()
}

pub fn disconnect_guest() {
    let mut guard = GUEST_CLIENT.write().unwrap();
    if let Some(client) = guard.take() {
        client.disconnect();
    }
}

pub fn is_guest_connected() -> bool {
    if let Some(client) = GUEST_CLIENT.read().unwrap().as_ref() {
        client.is_connected()
    } else {
        false
    }
}

fn jstring_to_string(env: &mut JNIEnv, j_str: &JString) -> String {
    if j_str.is_null() {
        return String::new();
    }
    env.get_string(j_str).map(|s| s.into()).unwrap_or_default()
}

fn string_to_jstring(env: &mut JNIEnv, s: &str) -> jstring {
    env.new_string(s).map(|js| js.into_raw()).unwrap_or(std::ptr::null_mut())
}

// ---------------- JNI EXPORTS ----------------

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_unshoo_pixelmusic_data_service_connect_PixelMusicNativeConnect_nativeInit(
    _env: JNIEnv,
    _class: JClass,
) {
    let _ = get_runtime();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_unshoo_pixelmusic_data_service_connect_PixelMusicNativeConnect_nativeStartHost(
    mut env: JNIEnv,
    _class: JClass,
    j_host_name: JString,
    j_port: jint,
) -> jboolean {
    let host_name = jstring_to_string(&mut env, &j_host_name);
    let port = if j_port > 0 { j_port as u16 } else { 0 };

    let rt = get_runtime();
    let res = rt.block_on(async {
        HostSession::start(host_name, port).await
    });

    match res {
        Ok(session) => {
            let mut guard = HOST_SESSION.write().unwrap();
            if let Some(old) = guard.take() {
                old.stop();
            }
            *guard = Some(session);
            JNI_TRUE
        }
        Err(e) => {
            log::error!("nativeStartHost error: {e}");
            JNI_FALSE
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_unshoo_pixelmusic_data_service_connect_PixelMusicNativeConnect_nativeStopHost(
    _env: JNIEnv,
    _class: JClass,
) {
    let mut guard = HOST_SESSION.write().unwrap();
    if let Some(session) = guard.take() {
        session.stop();
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_unshoo_pixelmusic_data_service_connect_PixelMusicNativeConnect_nativeUpdatePlayback(
    mut env: JNIEnv,
    _class: JClass,
    j_track_id: JString,
    j_title: JString,
    j_artist: JString,
    j_artwork_url: JString,
    j_duration_ms: jlong,
    j_position_ms: jlong,
    j_is_playing: jboolean,
) {
    let guard = HOST_SESSION.read().unwrap();
    if let Some(session) = guard.as_ref() {
        let track_id = jstring_to_string(&mut env, &j_track_id);
        let title = jstring_to_string(&mut env, &j_title);
        let artist = jstring_to_string(&mut env, &j_artist);
        let artwork_url = jstring_to_string(&mut env, &j_artwork_url);

        let pb = JamPlayback {
            track_id,
            title,
            artist,
            artwork_url,
            source: "YOUTUBE".to_string(),
            is_playing: j_is_playing != JNI_FALSE,
            position_ms: j_position_ms,
            duration_ms: j_duration_ms,
            server_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
            host_clock_ms: 0,
            youtube_id: None,
        };
        session.update_playback(pb);
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_unshoo_pixelmusic_data_service_connect_PixelMusicNativeConnect_nativePollCommands(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let guard = HOST_SESSION.read().unwrap();
    let commands = if let Some(session) = guard.as_ref() {
        session.poll_commands()
    } else {
        Vec::new()
    };

    let json_str = serde_json::to_string(&commands).unwrap_or_else(|_| "[]".to_string());
    string_to_jstring(&mut env, &json_str)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_unshoo_pixelmusic_data_service_connect_PixelMusicNativeConnect_nativeStartDiscovery(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = get_runtime();
        if get_discovery().start() {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    }));
    res.unwrap_or(JNI_FALSE)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_unshoo_pixelmusic_data_service_connect_PixelMusicNativeConnect_nativeStopDiscovery(
    _env: JNIEnv,
    _class: JClass,
) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        get_discovery().stop();
    }));
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_unshoo_pixelmusic_data_service_connect_PixelMusicNativeConnect_nativeGetDiscoveredDevices(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let devices = get_discovery().get_devices();
        serde_json::to_string(&devices).unwrap_or_else(|_| "[]".to_string())
    }));
    let json_str = res.unwrap_or_else(|_| "[]".to_string());
    string_to_jstring(&mut env, &json_str)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_unshoo_pixelmusic_data_service_connect_PixelMusicNativeConnect_nativeConnectToDevice(
    mut env: JNIEnv,
    _class: JClass,
    j_ip: JString,
    j_port: jint,
    j_guest_name: JString,
    j_device_type: JString,
) -> jboolean {
    let ip = jstring_to_string(&mut env, &j_ip);
    let port = j_port as u16;
    let guest_name = jstring_to_string(&mut env, &j_guest_name);
    let device_type = jstring_to_string(&mut env, &j_device_type);

    let rt = get_runtime();
    let res = rt.block_on(async {
        GuestClient::connect(&ip, port, &guest_name, &device_type).await
    });

    match res {
        Ok(client) => {
            let mut guard = GUEST_CLIENT.write().unwrap();
            if let Some(old) = guard.take() {
                old.disconnect();
            }
            *guard = Some(client);
            JNI_TRUE
        }
        Err(e) => {
            log::error!("nativeConnectToDevice error: {e}");
            JNI_FALSE
        }
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_unshoo_pixelmusic_data_service_connect_PixelMusicNativeConnect_nativeDisconnect(
    _env: JNIEnv,
    _class: JClass,
) {
    let mut guard = GUEST_CLIENT.write().unwrap();
    if let Some(client) = guard.take() {
        client.disconnect();
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_unshoo_pixelmusic_data_service_connect_PixelMusicNativeConnect_nativeSendRemoteCommand(
    mut env: JNIEnv,
    _class: JClass,
    j_action: JString,
    j_value: JString,
) -> jboolean {
    let action = jstring_to_string(&mut env, &j_action);
    let value = jstring_to_string(&mut env, &j_value);

    let guard = GUEST_CLIENT.read().unwrap();
    if let Some(client) = guard.as_ref() {
        let rt = get_runtime();
        let res = rt.block_on(async {
            client.send_command(&action, &value).await
        });
        if res.is_ok() {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    } else {
        JNI_FALSE
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_unshoo_pixelmusic_data_service_connect_PixelMusicNativeConnect_nativeGetRemotePlaybackState(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let guard = GUEST_CLIENT.read().unwrap();
    let pb = if let Some(client) = guard.as_ref() {
        client.get_playback()
    } else {
        JamPlayback::default()
    };

    let json_str = serde_json::to_string(&pb).unwrap_or_else(|_| "{}".to_string());
    string_to_jstring(&mut env, &json_str)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_unshoo_pixelmusic_data_service_connect_PixelMusicNativeConnect_nativeDiscoverSpotifyConnect(
    mut env: JNIEnv,
    _class: JClass,
    j_timeout_ms: jlong,
) -> jstring {
    let timeout = Duration::from_millis(if j_timeout_ms > 0 { j_timeout_ms as u64 } else { 3000 });
    let devices = zeroconf::discover_spotify_receivers(timeout);
    let json_str = serde_json::to_string(&devices).unwrap_or_else(|_| "[]".to_string());
    string_to_jstring(&mut env, &json_str)
}
