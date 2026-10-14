# Aoi for developers

Aoi is a Tauri app (Rust and Svelte) for Windows 11 and macOS. On Windows it captures Blue
Archive's window with Windows Graphics Capture; it reads the item and equipment lists, the
student screen, the shops and the credits from each frame, and shows what it has read in the
system's web view (WebView2 on Windows). It sends no input to the game.

It builds for macOS too, where there is no window to capture: the capture lives in
`src-tauri/src/watch/window.rs` and its crates are Windows-only dependencies, so the macOS
app starts with the status `no_source`.

On either system it can read an Android phone's screen over USB or Wi-Fi
(`src-tauri/src/android.rs`): USB debugging over `nusb` (the ADB interface found in the
device's configuration, which Windows lists for no device that is not composite), or wireless
debugging paired by a QR code and connected over TLS with `libadb`, its services found over mDNS, and the screen recorded by scrcpy's server
(`src-tauri/vendor/scrcpy`, pushed to the phone and started over ADB) as H.264 from the
phone's hardware encoder, at most 5 frames a second, and decoded by `openh264` (`src-tauri/src/video.rs`).
The `openh264` crate builds Cisco's decoder from source; Cisco's H.264 patent licence covers
only its own prebuilt binaries. The key and the phone paired are kept in the app's data folder
(`adbkey.pem`, `android.json`). A phone's frame is cut to its safe area and scaled to the
2222-wide window the references were drawn from (`screen::normalize`); the readers measure
in a 16:9 window's height (`screen::unit`), each part kept to the top, the middle or the
bottom of a wider screen (`screen::Anchor`).

Any phone can also send its screen from a streaming app over RTMP (`src-tauri/src/broadcast.rs`,
with `rml_rtmp`): when the user asks, Aoi listens on port 1935 under a fresh key for streams from
the local network, one at a time, till none comes for 10 minutes. Its FLV video tags are turned
into Annex B for `openh264`; a screen boxed in black is cut out and one sent upright turned back
(`screen::trim`, `screen::turn`). The window shows the URL and a QR code of Moblin's settings link
(`moblin://?<JSON>`: H.264, 2560x1440, 50 Mbps, 15 fps). `cargo run --release --example watch --
60 wait` waits as the app does, and `cargo run --release --example send_stream -- <url> <png>...`
stands in for a phone.

- [Data and file formats](data.md): where the item data comes from, what is counted and
  when, and the YAML Aoi writes.
- The player's guides are in Japanese, under [docs/guide](../guide).
- Design rules of the window: [DESIGN.md](../../DESIGN.md).

## Development

Needs Node.js 24 and Rust 1.96 (pinned by `rust-toolchain.toml`).

```powershell
npm ci
npm run tauri dev       # the app, reloading on changes
npm run verify          # every check: format, lint, unit, browser and recognition tests
git config core.hooksPath .githooks   # run verify before pushing main or a tag
```

Recognition is tested against labeled screenshots in `src-tauri/tests/fixtures`; the
references it reads text with are built from them ([Reading text](data.md#reading-text)). A
student screenshot is added there with only the areas Aoi reads kept, a tenth of its
size: `cargo run --release --example mask_fixture -- <screenshot.png> <fixture.png>`.
Level samples (`levels.json`), taken on any tab of the student screen, keep the level
alone: add `level` after the two paths; `credit` keeps the top bar's credits, `shop` what
a shop screen is read from.

To measure against the running game, run `cargo run --release --example watch -- 60` in
`src-tauri`. It watches for 60 seconds and prints the frame rate and dropped frames.

## Releases

Pushing a `v*` tag builds the exe on GitHub Actions and publishes it with its SHA-256
(`bluearchive-aoi-x86_64-pc-windows-msvc.exe` and `.exe.sha256`). The app's
アップデートを確認 asks the GitHub API for the latest release; 更新して再起動 downloads its
exe, checks it against the SHA-256, puts it in place of the running one (the old one kept
as `.old` until the next start) and starts it.

A second job then adds the universal macOS disk image and its SHA-256
(`bluearchive-aoi-universal-apple-darwin.dmg` and `.dmg.sha256`), bundled as
`src-tauri/tauri.macos.conf.json` says: signed ad hoc, not notarized. On macOS the update
check looks for the disk image, and 入手ページを開く opens the release page.

To check a download by hand:

```powershell
Get-FileHash .\bluearchive-aoi-x86_64-pc-windows-msvc.exe -Algorithm SHA256
```
