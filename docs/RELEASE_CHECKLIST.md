# Release checklist

Two tracks share one pipeline. A **beta** goes to people who opted in (Settings → updates, beta channel) and
may still have rough edges; a **stable** release goes to everyone and must not. Work through the beta list
for every beta, and the stable list on top of it before promoting a build to stable.

How a release is cut today:

- `version` in the root `Cargo.toml` is the app's version (`1.0.0-beta` now).
- Pushing a tag `v*` runs `.github/workflows/release.yml`: it builds and tests on Windows, Linux and macOS
  (`cargo test --locked --release --workspace`), then creates the GitHub Release with the files and checksums.
- A tag with a hyphen (`v1.0.0-beta.2`) is published as a **pre-release**; the in-app updater offers
  pre-releases only on the beta channel. A tag without one (`v1.0.0`) is a stable release, offered to everyone.

---

## Beta release

### 1. Code

- [ ] All work for this beta is merged into `main` through a pull request (not pushed straight from a branch).
- [ ] `cargo test --workspace --locked` passes locally.
- [ ] `cargo clippy --workspace --locked` has no new warnings.
- [ ] `qmllint` shows no new errors in changed QML files.
- [ ] **QML load check:** start the built app once with `QT_FORCE_STDERR_LOGGING=1` and confirm stderr has no
      `Type … unavailable`, `Router:` or `Invalid property assignment` lines. The linter does not catch every
      load error (a `Behavior` on a `readonly` property broke Home and Search once and lint stayed clean).
      Every sidebar page is pre-built at start-up, so one launch covers them.
- [ ] No test or debug files in the build (`_bench_*.qml`, temporary logging).

### 2. Version and notes

- [ ] Bump `version` in `Cargo.toml` to the next beta (`1.0.0-beta.N`) and commit it.
- [ ] Write short release notes: what is new, what was fixed, known issues. The updater shows them.

### 3. Smoke test (Windows, the installed build)

Install with `target\dist\PixelMusic-<version>-Setup.exe` over the previous version.

- [ ] Settings, sign-ins and the queue survived the upgrade.
- [ ] Sidebar: every section opens on the first click (Home, Search, Smart Mix, Library, Local files, Downloads).
- [ ] Home loads; skeletons show while it does; pull to refresh and clicking the "Home" title both refresh it.
- [ ] Tapping a song plays it (Home lists, Quick picks, search, artist page, playlist).
- [ ] Play / Shuffle on a Home shelf and on its "See all" page.
- [ ] Drag a song from a list into the queue.
- [ ] Queue: drag to reorder (also the last song to the top), shuffle on and off in a YouTube radio.
- [ ] Artist radio (artist page and "Similar to" on Home) starts playing.
- [ ] Full player in light and dark theme, with a colourful and a black-and-white cover: every button readable.
- [ ] Mini player, lyrics, volume, seek.
- [ ] Sidebar and queue panel open and close smoothly.
- [ ] Settings → Appearance: a palette, pitch black (dark mode), light/dark switch.
- [ ] Listening stats: every range (Today, Week, Month days/weeks, Year, All time).
- [ ] Smart Mix: recommendations prepare by themselves on the first visit.
- [ ] Close and reopen: the app comes back where it was.

### 4. Publish

- [ ] Tag the version commit `v1.0.0-beta.N` and push the tag; wait for `release.yml` to finish green.
- [ ] On the GitHub Release: it is marked **pre-release**, all files and the checksums are attached, notes are in.
- [ ] On a machine on the beta channel, the in-app updater finds, downloads and installs the new beta.

### 5. After release

- [ ] Watch issues and incoming `crash.log` reports for a few days.
- [ ] Note anything that must be fixed before stable.

---

## Stable release

Everything in the beta list, for the build being promoted, plus the following.

### 1. Readiness

- [ ] The release candidate has been out as a beta for at least a week with no open crash or data-loss bugs.
- [ ] Every issue tagged for this release is closed or consciously moved to the next one.
- [ ] Version without a suffix in `Cargo.toml` (`1.0.0`); release notes cover everything since the last stable.

### 2. Test matrix

Test on clean machines or VMs that have never had Qt, Visual Studio or Rust installed.

- [ ] Windows 10 22H2 and Windows 11 (latest), fresh install and upgrade from the previous stable.
- [ ] A low-end laptop on battery, Windows power mode set to battery saving: scrolling, sidebar, playback.
- [ ] A laptop with hybrid graphics: Settings → Graphics processor → Discrete takes effect after a restart.
- [ ] Display scaling 100 %, 125 %, 150 % and 200 %, and a second monitor with a different scale.
- [ ] Russian and Portuguese: no untranslated or cut-off text on the main screens.
- [ ] First run signed out, no network, slow network, and a large library (thousands of songs).
- [ ] Uninstall removes the app and its Start menu / taskbar entries; settings are kept or removed as promised.
- [ ] Linux and macOS packages from the same release start and play (if they ship with this release).

### 3. Trust and installation

- [ ] **Code signing:** the exe, the installer and the uninstaller are signed with an Authenticode certificate,
      so Windows SmartScreen does not warn people away (an unsigned installer shows "Windows protected your PC").
- [ ] The signature and timestamp verify (`signtool verify /pa /v`) on the published files.
- [ ] Checksums on the release page match the files.

### 4. Legal

- [ ] GPL-3.0: the release links to the exact source (the tag), and `LICENSE`, `NOTICE` and `LICENSES\` ship
      with the app. **Open gap:** `build_windows.ps1` does not copy `NOTICE` or `LICENSES\` into
      `target\dist\pixelmusic\` yet (only the old Inno installer shows `LICENSE`); add them before the first stable.
- [ ] Third-party notices are complete: Qt (LGPL), librespot, fonts (OFL), Material Symbols (Apache 2.0) and any
      dependency added since the last stable.
- [ ] How the app is described and where it is distributed fit the risk of an unofficial YouTube Music client
      (some stores will reject it).

### 5. Publish

- [ ] Tag `v1.0.0` from the tested commit (the one the last beta came from, plus only the version bump).
- [ ] `release.yml` green; the GitHub Release is **not** a pre-release and is marked latest.
- [ ] The stable channel's in-app updater offers it; a beta-channel install also moves to it.
- [ ] Release notes and download links on any website or README updated.

### 6. After release

- [ ] Watch crash reports and issues closely for the first week.
- [ ] Keep a hotfix path ready: a `v1.0.1` from the release tag, not from `main`, if `main` has moved on.
- [ ] Merge the release branch back so `main` has the version bump and any hotfix.

---

## Known open items (as of `a99d277`)

Resolve or consciously accept these before the stable release:

- Sidebar and queue panel open/close: the motion is smooth, but the page re-lays out once (about 110 ms on the
  test machine) at the start or the end of it. Check how it feels on a slow PC.
- Not yet checked by hand: drag-to-queue from search and artist pages, pull to refresh, Smart Mix automatic
  recommendations, pitch black across all pages, the Windows graphics-card setting.
- "From your library" has no Play / Shuffle (left out on purpose: it lists playlists and artists, not songs).
- Smart Mix built from YouTube Music (new releases, quick picks, top artists, genre radio) is planned but not
  designed yet.
