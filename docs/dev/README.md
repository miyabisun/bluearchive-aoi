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
phone's hardware encoder, at most 5 frames a second and 1920 pixels on the longer side, and decoded by `openh264` (`src-tauri/src/video.rs`).
A recording that sends nothing for 5 seconds (a still screen's frames come too) is taken to hang:
the link stops streaming and the phone is connected anew.
The `openh264` crate builds Cisco's decoder from source; Cisco's H.264 patent licence covers
only its own prebuilt binaries. The key and the phone paired are kept in the app's data folder
(`adbkey.pem`, `android.json`). Every frame, a phone's cut to its safe area, is scaled down to
the widest of the widths the texts are read at no wider than it (`screen::WIDTHS`: 1920, 1760,
1600 and 1500; `screen::normalize`); the readers measure in a 16:9 window's height
(`screen::unit`), each part kept to the top, the middle or the bottom of a wider screen
(`screen::Anchor`). Each width has references of its own, drawn from the tuning fixtures
scaled to it (`data/glyphs-*.png`, `data/digits.json`; `examples/build_glyphs.rs`,
`examples/build_digits.rs`), and the tests read the evaluation fixtures drawn at widths between
them too: right or not at all. A stream blurs the counts and the student screen's texts beyond any
scaling, so their references also come from a tablet's own stream at 1920 (and 1600) wide, and
the tests read a few of its frames (`tests/fixtures/tablet.json`, the `android/tablet-stu-*`
student screens). A phone sending fewer lines (a Pixel 9 1920 by 854, the game 1646 wide, read
at 1600) thins them further: the 1600 references of the student screen are also drawn from the
tuning fixtures as such a stream shows them, scaled down on the phone and their colour halved
(`build_glyphs`'s `STREAMS`), and the tests read a Pixel 9's frames sent so
(`android/pixel9-*-1920`). A tablet's lossless student screens of a whole roster give references too
(`android/tablet-roster-*`, and the name plates `names/roster-*`). A screen drawn narrower than 1500 is not read
(`screen::too_small`).

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
a shop screen is read from, `list` the item or equipment list's panel, `name` the name plate's
name alone (`names.json`).

To measure against the running game, run `cargo run --release --example watch -- 60` in
`src-tauri`. It watches for 60 seconds and prints the frame rate and dropped frames.

## Releases

A release is its commit "release: vX.Y.Z" on main, pushed with its `vX.Y.Z` tag: GitHub
Actions builds the exe from that commit (not from the tag, so the Rust cache main keeps serves
every release) and publishes it with its SHA-256 (`bluearchive-aoi-x86_64-pc-windows-msvc.exe`
and `.exe.sha256`). Other pushes to main build nothing. The app's
アップデートを確認 reads the latest release's tag off where `github.com/…/releases/latest`
sends to (not the GitHub API, whose 60 calls an hour without a token a home shares);
更新して再起動 downloads that tag's exe from `releases/download/<tag>/`, checks it against the
SHA-256, puts it in place of the running one (the old one kept as `.old` until the next
start) and starts it.

A second job then adds the universal macOS disk image and its SHA-256
(`bluearchive-aoi-universal-apple-darwin.dmg` and `.dmg.sha256`), bundled as
`src-tauri/tauri.macos.conf.json` says: signed ad hoc, not notarized. On macOS the update
check looks for the disk image, and 入手ページを開く opens the release page.

To check a download by hand:

```powershell
Get-FileHash .\bluearchive-aoi-x86_64-pc-windows-msvc.exe -Algorithm SHA256
```
