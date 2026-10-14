# Aoi for developers

Aoi is a Tauri app (Rust and Svelte) for Windows 11. It captures Blue Archive's window
with Windows Graphics Capture, reads the item and equipment lists, the student screen,
the shops and the credits from each frame, and shows what it has read in a WebView2
window. It sends no input to the game.

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

To check a download by hand:

```powershell
Get-FileHash .\bluearchive-aoi-x86_64-pc-windows-msvc.exe -Algorithm SHA256
```
