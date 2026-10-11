# bluearchive-aoi

Aoi counts your Blue Archive items and equipment for you. Keep it open next to the game
and scroll through the item list or the equipment list. Aoi reads each entry and its
count from the game's window, up to 30 times a second, and lists what it has seen by
name.

It only looks: it sends no input to the game and saves nothing. The list lasts until
you close Aoi.

## Requirements

- Windows 11. Aoi captures the game with Windows Graphics Capture and draws its window
  with WebView2, both part of Windows 11.
- The Japanese PC version of Blue Archive (`BlueArchive.exe`). Item names are shown in
  Japanese.

## Install

Download `bluearchive-aoi-x86_64-pc-windows-msvc.exe` from the
[latest release](https://github.com/miyabisun/bluearchive-aoi/releases/latest). Check it
against the `.sha256` file published beside it:

```powershell
Get-FileHash .\bluearchive-aoi-x86_64-pc-windows-msvc.exe -Algorithm SHA256
```

It is a single exe with nothing to install; run it from anywhere.

## Use

1. Start Aoi, before or after the game. It finds the game's window by itself.
2. In the game, open the item list (アイテム) or the equipment list (装備) and sort it by
   **デフォルト** with the ascending arrow (≡↑), the game's default.
3. Scroll the list. Entries appear in Aoi in the game's order with their counts, under
   the tab of their list; type in **名前で検索** to find one. Equipment names carry their
   tier (T1 to T10), since a blueprint's name alone does not tell it.

Aoi switches to the tab of the list the game starts showing; a tab you pick stays until
the game shows another list.

The badge at the top right tells what Aoi sees:

| Badge                                       | Meaning                                                                          |
| ------------------------------------------- | -------------------------------------------------------------------------------- |
| ゲーム未検出                                | Blue Archive is not running.                                                     |
| 対象外の画面                                | The game shows another screen, or the list in another order. Nothing is counted. |
| アイテムを集計中 N fps / 装備を集計中 N fps | That list is being read, N frames a second.                                      |
| キャプチャ失敗                              | Windows refused to capture the game's window; the reason is shown below.         |

Other windows may cover the game; Aoi reads the game window itself, not the screen.

### What is counted

- A count is taken once the same number has been read in three frames in a row, so a
  single misread frame does not change it. When you use an item in the game, its count
  follows when the list is shown again.
- Counts of 10,000 or more are shown as the game shows them, in thousands: 51K stands
  for 50,500 to 51,499.
- A count Aoi cannot read for sure is skipped in that frame rather than guessed. The
  selected item's count, under its blinking frame, is read in the frames where the frame
  is faint.
- Items Aoi cannot tell apart for sure are left out rather than guessed. Items released
  after the bundled item data are left out until a release with newer data.
- Event items, from the event points to the Valentine chocolates, are never listed. They
  expire with their event, and the event's own page shows them.

## Item data

Names and icons of items and equipment come from a snapshot of
[SchaleDB](https://schaledb.com) built into the exe. A few items SchaleDB lacks, and a
few icons the game draws unlike SchaleDB, are added in `src-tauri/data/extra`, cut from
the game. `tools/schaledb/update.ps1` refreshes the snapshot.

## Development

Needs Node.js 24 and Rust 1.96 (pinned by `rust-toolchain.toml`).

```powershell
npm ci
npm run tauri dev       # the app, reloading on changes
npm run verify          # every check: format, lint, unit, browser and recognition tests
git config core.hooksPath .githooks   # run verify before pushing main or a tag
```

Recognition is tested against labeled screenshots in `src-tauri/tests/fixtures`.
To measure against the running game, run `cargo run --release --example watch -- 60` in
`src-tauri`. It watches for 60 seconds and prints the frame rate and dropped frames.

Pushing a `v*` tag builds the exe on GitHub Actions and publishes it with its SHA-256.

## License

MIT
