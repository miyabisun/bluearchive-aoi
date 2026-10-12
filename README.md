# bluearchive-aoi

Aoi counts your Blue Archive items and equipment and notes how far your students are
trained. Keep it open next to the game and scroll through the item list or the
equipment list, or step through your students' screens. Aoi reads what the game's
window shows, up to 30 times a second, and lists what it has seen by name.

It only looks: it sends no input to the game, and saves nothing unless you export what
it read. The lists last until you close Aoi.

## Requirements

- Windows 11. Aoi captures the game with Windows Graphics Capture and draws its window
  with WebView2, both part of Windows 11.
- The Japanese PC version of Blue Archive (`BlueArchive.exe`). Item names are shown in
  Japanese.
- Japanese text recognition of Windows, installed with the Japanese language. Aoi uses it
  to read student names; without it, no student is listed.

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

For students, open a student's screen (生徒 → a student, the 基本情報 tab) and step
through your students with the game's ＜ and ＞ buttons, pausing a moment on each. Each
student appears under the 生徒 tab with their training:

| Column   | Shows                                                                       |
| -------- | --------------------------------------------------------------------------- |
| Lv, ★    | Level and star rarity.                                                      |
| 固有武器 | The unique weapon's stars and level; — while locked.                        |
| スキル   | EX skill level, then the other three, M for a skill at its maximum: `5M7M`. |
| 装備     | The tiers of the three equipment slots, `T10/9/8`; hover for their levels.  |
| 愛用品   | The favorite item's tier; — without one.                                    |
| 能力解放 | The levels of the three ability release rows.                               |
| 絆       | Bond rank.                                                                  |

A field Aoi cannot read for sure shows ?. On the student screen's other tabs (レベルアップ,
神秘解放) Aoi reads the name plate alone, so a student levelled up there shows the new
level without going back to 基本情報.

The 生徒 tab tells how many students have been read (生徒 42). Compare it with how many
students you have in the game to see whether any were missed; a student whose name could
not be read is not counted.

Students are listed highest level first, then by rarity, then by bond. Click a column's
heading to list them by that column instead, highest first (skills, ability release and
the equipment's tiers by their total); ties keep the default order, and ? comes last.

Aoi switches to the tab of what the game starts showing; a tab you pick stays until the
game shows another list or the student screen.

The badge at the top right tells what Aoi sees:

| Badge                                       | Meaning                                                                          |
| ------------------------------------------- | -------------------------------------------------------------------------------- |
| ゲーム未検出                                | Blue Archive is not running.                                                     |
| 対象外の画面                                | The game shows another screen, or the list in another order. Nothing is counted. |
| アイテムを集計中 N fps / 装備を集計中 N fps | That list is being read, N frames a second.                                      |
| 生徒を集計中 N fps                          | A student's screen is being read, N frames a second.                             |
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
- A student's training is taken once three frames in a row read alike, and a field read
  later keeps its known value when a frame cannot read it. Students released after the
  bundled student data are not listed.

### Exporting

**書き出す**, at the right of the tabs, writes everything read so far (students, items and
equipment) to `Documents\Aoi\aoi-<date>-<time>.yaml` and shows the file's path. Hand the
file to an agent or a tool to check what Aoi read or to plan with it:

<!-- prettier-ignore -->
```yaml
students:
  - id: 10059
    name: "ミカ"
    level: 90
    rarity: 5
    bond: 43
    skills: [5, 10, 10, 10]  # EX, normal, passive, sub
    weapon: {stars: 3, level: 50}  # `locked` before it is unlocked
    equipment: [{tier: 10, level: 70}, {tier: 10, level: 70}, null]
    gear: 0  # the favorite item's tier, 0 without one
    ability: [25, 25, 0]  # HP, attack, healing
items:
  - id: 1
    name: "初級強化珠"
    count: 1383
equipment:
  - id: 501000
    name: "帽子の万能設計図"
    count: 51K  # thousands: 50,500 to 51,499
```

`null` marks a field Aoi could not read, apart from what the student lacks. A student
SchaleDB does not list yet has `id: null`.

## Item data

Names and icons of items and equipment, and student names, come from a snapshot of
[SchaleDB](https://schaledb.com) built into the exe. A few items and students SchaleDB
lacks, and a few icons the game draws unlike SchaleDB, are added in
`src-tauri/data/extra`, cut from the game; a student added there is told by name alone
until SchaleDB lists them. `tools/schaledb/update.ps1` refreshes the snapshot.

## Development

Needs Node.js 24 and Rust 1.96 (pinned by `rust-toolchain.toml`).

```powershell
npm ci
npm run tauri dev       # the app, reloading on changes
npm run verify          # every check: format, lint, unit, browser and recognition tests
git config core.hooksPath .githooks   # run verify before pushing main or a tag
```

Recognition is tested against labeled screenshots in `src-tauri/tests/fixtures`.
A student screenshot is added there with only the areas Aoi reads kept, a tenth of its
size: `cargo run --release --example mask_fixture -- <screenshot.png> <fixture.png>`.
Level samples (`levels.json`), taken on any tab of the student screen, keep the level
alone: add `level` after the two paths.
To measure against the running game, run `cargo run --release --example watch -- 60` in
`src-tauri`. It watches for 60 seconds and prints the frame rate and dropped frames.

Pushing a `v*` tag builds the exe on GitHub Actions and publishes it with its SHA-256.

## License

MIT
