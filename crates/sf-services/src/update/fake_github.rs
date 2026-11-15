//! The updater end to end against a local fake of GitHub's releases API (`PIXELMUSIC_UPDATE_API` does the
//! same for a running app): which release each channel offers, the asset for the install kind, and the
//! download checked against SHA256SUMS.txt. This build is `CARGO_PKG_VERSION` (1.0.0-beta); the fake has a
//! stable 1.0.0 and a pre-release 1.1.0-beta.1.
use std::collections::HashMap;
use std::sync::Arc;

use mf_platform::{SettingKey, SettingValue, SettingsStore};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::broadcast;

use super::*;

type Routes = Arc<HashMap<String, (u16, Vec<u8>)>>;

fn sha256_hex(bytes: &[u8]) -> String {
    use sf_api::sha2::Digest as _;
    hex(&sf_api::sha2::Sha256::digest(bytes))
}

/// A release with a portable zip, a Setup.exe and SHA256SUMS.txt (whose zip line is `zip_sum`, if given).
fn release(base: &str, routes: &mut HashMap<String, (u16, Vec<u8>)>, version: &str, prerelease: bool, zip_sum: Option<&str>) -> Value {
    let zip_name = format!("PixelMusic-{version}-windows-x64-portable.zip");
    let setup_name = format!("PixelMusic-{version}-Setup.exe");
    let zip = format!("portable build {version}").into_bytes();
    let setup = format!("setup build {version}").into_bytes();
    let sums = format!(
        "{}  {zip_name}\n{}  {setup_name}\n",
        zip_sum.map_or_else(|| sha256_hex(&zip), str::to_owned),
        sha256_hex(&setup)
    );
    let mut assets = Vec::new();
    for (name, bytes) in [(zip_name, zip), (setup_name, setup), (SUMS_NAME.to_owned(), sums.into_bytes())] {
        let path = format!("/download/v{version}/{name}");
        assets.push(json!({ "name": name, "size": bytes.len(), "browser_download_url": format!("{base}{path}") }));
        routes.insert(path, (200, bytes));
    }
    json!({
        "tag_name": format!("v{version}"), "name": format!("PixelMusic {version}"), "body": format!("Notes for {version}"),
        "draft": false, "prerelease": prerelease, "html_url": format!("{base}/releases/tag/v{version}"),
        "published_at": "2026-11-15T12:00:00Z", "assets": assets,
    })
}

struct Fake {
    service: Arc<UpdateService>,
    settings: Arc<SettingsStore>,
    dir: PathBuf,
}

/// A fake GitHub: `latest` is the stable answer, `list` the beta one (newest first, as GitHub sorts).
async fn fake(kind: InstallKind, build: impl FnOnce(&str, &mut HashMap<String, (u16, Vec<u8>)>) -> (Value, Value)) -> Fake {
    // the port is known only after binding, so the routes are filled through a shared map
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let mut routes = HashMap::new();
    let (latest, list) = build(&base, &mut routes);
    routes.insert("/repos/fake/releases/latest".into(), (if latest.is_null() { 404 } else { 200 }, latest.to_string().into_bytes()));
    routes.insert("/repos/fake/releases?per_page=20".into(), (200, list.to_string().into_bytes()));
    // serve on the same port the URLs name
    let listener = TcpListener::bind(base.trim_start_matches("http://")).await.unwrap();
    let routes: Routes = Arc::new(routes);
    tokio::spawn(async move {
        loop {
            let Ok((socket, _)) = listener.accept().await else { return };
            let routes = routes.clone();
            tokio::spawn(answer(socket, routes));
        }
    });
    let settings = SettingsStore::in_memory();
    let service = Arc::new(UpdateService {
        rt: Handle::current(),
        settings: settings.clone(),
        http: sf_api::Http::default(),
        app_requests: broadcast::channel(4).0,
        kind,
        api: format!("{base}/repos/fake"),
        snapshot: watch::Sender::new(UpdateSnapshot { current_version: current_version(), eta: -1, ..UpdateSnapshot::default() }),
        inner: Mutex::new(Inner::default()),
    });
    let dir = std::env::temp_dir().join(format!("mf-update-fake-{}-{}", std::process::id(), base.rsplit(':').next().unwrap()));
    let _ = std::fs::remove_dir_all(&dir);
    Fake { service, settings, dir }
}

async fn answer(mut socket: tokio::net::TcpStream, routes: Routes) {
    let mut request = Vec::new();
    let mut buf = [0u8; 4096];
    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
        match socket.read(&mut buf).await {
            Ok(0) | Err(_) => return,
            Ok(n) => request.extend_from_slice(&buf[..n]),
        }
    }
    let head = String::from_utf8_lossy(&request);
    let path = head.split_whitespace().nth(1).unwrap_or("/").to_owned();
    let (status, body) = routes.get(&path).cloned().unwrap_or((404, b"{\"message\":\"Not Found\"}".to_vec()));
    let reply = format!("HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
    let _ = socket.write_all(reply.as_bytes()).await;
    let _ = socket.write_all(&body).await;
}

impl Fake {
    fn channel(&self, channel: &str) {
        self.settings.set(SettingKey::UpdateChannel, SettingValue::Str(channel.into()));
    }

    /// A check as "Check now" runs it; the snapshot afterwards.
    async fn check(&self) -> UpdateSnapshot {
        self.service.apply(Event::CheckStarted { manual: true });
        match self.service.fetch_offer().await {
            Ok((offer, skipped)) => {
                self.service.apply(Event::CheckFinished { offer, skipped, at: now_unix() });
            }
            Err(error) => {
                self.service.apply(Event::Failed { stage: Stage::Check, text: error.text, retryable: error.retryable });
            }
        }
        self.service.snapshot()
    }

    /// Downloads and verifies the offer into a temporary folder; the snapshot afterwards.
    async fn download(&self) -> UpdateSnapshot {
        let offer = self.service.snapshot().offer.expect("an offer");
        self.service.apply(Event::DownloadStarted { resumed_from: 0, total: offer.size });
        if let Err(error) = self.service.download_and_verify(&offer, &self.dir).await {
            self.service.apply(Event::Failed { stage: Stage::Download, text: error.text, retryable: error.retryable });
        }
        self.service.snapshot()
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn both(bad_stable_sum: bool) -> impl FnOnce(&str, &mut HashMap<String, (u16, Vec<u8>)>) -> (Value, Value) {
    move |base, routes| {
        let bad = bad_stable_sum.then_some("0000000000000000000000000000000000000000000000000000000000000000");
        let stable = release(base, routes, "1.0.0", false, bad);
        let beta = release(base, routes, "1.1.0-beta.1", true, None);
        let old = release(base, routes, "0.9.0", false, None);
        (stable.clone(), json!([beta, stable, old]))
    }
}

#[tokio::test]
async fn stable_channel_offers_the_newest_stable_release_and_installs_it() {
    let fake = fake(InstallKind::WindowsPortable, both(false)).await;
    fake.channel("stable");
    let checked = fake.check().await;
    assert_eq!(checked.state, UpdateState::Available, "{checked:?}");
    let offer = checked.offer.clone().unwrap();
    assert_eq!(offer.version, "1.0.0");
    assert!(!offer.prerelease);
    assert_eq!(offer.asset_name, "PixelMusic-1.0.0-windows-x64-portable.zip");
    assert_eq!(offer.notes, "Notes for 1.0.0");

    let done = fake.download().await;
    assert_eq!(done.state, UpdateState::ReadyToInstall, "{done:?}");
    assert_eq!(std::fs::read(fake.dir.join(&offer.asset_name)).unwrap(), b"portable build 1.0.0");
}

#[tokio::test]
async fn beta_channel_offers_the_newest_pre_release_and_installs_it() {
    let fake = fake(InstallKind::WindowsPortable, both(false)).await;
    fake.channel("beta");
    let checked = fake.check().await;
    assert_eq!(checked.state, UpdateState::Available, "{checked:?}");
    let offer = checked.offer.clone().unwrap();
    assert_eq!(offer.version, "1.1.0-beta.1");
    assert!(offer.prerelease);
    assert_eq!(offer.asset_name, "PixelMusic-1.1.0-beta.1-windows-x64-portable.zip");

    let done = fake.download().await;
    assert_eq!(done.state, UpdateState::ReadyToInstall, "{done:?}");
    assert_eq!(std::fs::read(fake.dir.join(&offer.asset_name)).unwrap(), b"portable build 1.1.0-beta.1");
}

#[tokio::test]
async fn an_installed_copy_gets_the_setup_exe() {
    let fake = fake(InstallKind::WindowsInstaller, both(false)).await;
    fake.channel("beta");
    let offer = fake.check().await.offer.unwrap();
    assert_eq!(offer.asset_name, "PixelMusic-1.1.0-beta.1-Setup.exe");
    assert_eq!(fake.download().await.state, UpdateState::ReadyToInstall);
}

#[tokio::test]
async fn a_damaged_download_is_refused() {
    let fake = fake(InstallKind::WindowsPortable, both(true)).await;
    fake.channel("stable");
    fake.check().await;
    let done = fake.download().await;
    assert_eq!(done.state, UpdateState::Error, "{done:?}");
    assert!(done.error_text.contains("checksum"), "{}", done.error_text);
    assert!(!fake.dir.join("PixelMusic-1.0.0-windows-x64-portable.zip").exists());
}

#[tokio::test]
async fn nothing_newer_is_up_to_date_on_both_channels() {
    let only_old = |base: &str, routes: &mut HashMap<String, (u16, Vec<u8>)>| {
        let old = release(base, routes, "0.9.0", false, None);
        let old_beta = release(base, routes, "1.0.0-alpha.3", true, None);
        (old.clone(), json!([old_beta, old]))
    };
    let fake = fake(InstallKind::WindowsPortable, only_old).await;
    for channel in ["stable", "beta"] {
        fake.channel(channel);
        let checked = fake.check().await;
        assert_eq!(checked.state, UpdateState::UpToDate, "{channel}: {checked:?}");
        assert!(checked.offer.is_none());
    }
}

#[tokio::test]
async fn a_repo_without_releases_is_up_to_date() {
    // what the real repo answers today: no releases yet (`latest` is a 404, the list is empty)
    let fake = fake(InstallKind::WindowsPortable, |_: &str, _: &mut HashMap<String, (u16, Vec<u8>)>| (Value::Null, json!([]))).await;
    for channel in ["stable", "beta"] {
        fake.channel(channel);
        assert_eq!(fake.check().await.state, UpdateState::UpToDate, "{channel}");
    }
}

#[tokio::test]
async fn a_skipped_version_is_marked() {
    let fake = fake(InstallKind::WindowsPortable, both(false)).await;
    fake.channel("beta");
    fake.settings.set(SettingKey::SkippedVersion, SettingValue::Str("1.1.0-beta.1".into()));
    let checked = fake.check().await;
    assert!(checked.skipped, "{checked:?}");
}
