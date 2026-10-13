# bluearchive-aoi

Aoi counts your Blue Archive items and equipment, notes how far your students are
trained and how many more times the shops sell each of their goods. Keep it open next
to the game and scroll through the item list, the equipment list or a shop, or step
through your students' screens. Aoi reads what the game's
window shows, up to 30 times a second, and lists what it has seen by name.

It only looks: it sends no input to the game. What it reads is kept: Aoi starts from it
the next time, and writes the latest of it to `Documents\Aoi\aoi.yaml`.

## Requirements

- Windows 11. Aoi captures the game with Windows Graphics Capture and draws its window
  with WebView2, both part of Windows 11.
- The Japanese PC version of Blue Archive (`BlueArchive.exe`). Item names are shown in
  Japanese.
- Japanese text recognition of Windows, installed with the Japanese language. Aoi uses it
  to read student names and the shops; without it, no student or shop is listed.

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
3. Scroll the list. Entries appear in Aoi in the game's order, each as its icon with its
   count at the corner, under the tab of their list; hover an icon for its name, or type
   in **名前で検索** to find one (hiragana finds the katakana names).

For students, open a student's screen (生徒 → a student, the 基本情報 tab) and step
through your students with the game's ＜ and ＞ buttons, pausing a moment on each. Each
student appears under the 生徒 tab with their training:

| Column   | Shows                                                                       |
| -------- | --------------------------------------------------------------------------- |
| Lv       | Level.                                                                      |
| 星       | ★1 to ★4, then the unique weapon's 固有1 to 固有4; hover for its level.     |
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

Students are listed highest level first, then by 星, then by bond. Click a column's
heading to list them by that column instead, highest first (skills, ability release and
the equipment's tiers by their total); ties keep the default order, and ? comes last.

For the shops, open ショップ, choose 総力戦, 大決戦, カケラ(文字), 合同火力演習, 指名手配 or
熟達証書, and scroll its goods to the bottom, where those bought up go. Each shop is listed
under the ショップ tab, closed until you open it (a search opens them all), headed by the
coins held: a coin that is an item counts as under the アイテム tab, whichever was read
last, and the 熟達証書, which only its shop shows, as read there. Its goods go in the
order they are worth buying: 秘伝ノート, 神名のカケラ, the elephs (those the plain coin buys
first, then fest, limited, always and event students), テイラーストーン, tactical BDs, tech
notes, ooparts, the rest, チタン撃針, credits, then reports, booster tickets and blueprint
boxes (チタン撃針 and credits, which only 熟達証書 sells, are read there too); goods bought
up with every coin go last, greyed out. Each shows as its icon with how many come with a
purchase (`×5`; hover for its name), and under it, for each coin that buys it, the coin's icon with the times left at
its price, `40回@10`, or 売切れ once bought up. 総力戦 and 大決戦 take two coins: the
plain one on the first line, the rare one under it. The prices are the shops' own, written in
`src-tauri/data/shops.yaml` from the [Blue Archive wiki](https://bluearchive.wiki/wiki/Shop);
a good they do not cover shows `@?`. In カケラ(文字) an eleph costs its student's rate,
read off the card (1 to 5 fragments).
All but カケラ(文字) reset as the month changes: a shop read in an earlier month is marked
前の月の値 until it is read again. 通常アイテム, 青輝石, カケラ(その他) and 戦術対抗戦 are
not read.

To plan training, tick 育成 beside the students to train under the 生徒 tab. The 育成 tab
lists them with their targets, Lv90, skills 5/10/10/10 and equipment T10/10/10 to start
with; click a target and change it with ↑ and ↓ (the value read stands over it). The star
(星) stays at the student's own until you choose one: ★1 to ★4, then the unique weapon's
固有1 to 固有4 (★5 is 固有1), counting the elephs and credits each step takes. The elephs short are
bought with 神名のカケラ at the student's rate the カケラ(文字) shop showed: 20 at that rate,
20 at each next rate up to 4, then 5 each (the shop does not tell how many of the 20 are
already bought), no more than it has left; the fragments join the materials below, and a
student whose rate was not read is named as not counted. Below come what the
targets take, all together, against what is held: credits (in millions, M), the exp of
activity reports and of enhancement orbs, then the other materials and blueprints, each
足りる or 不足 by how much (約 where a count is in thousands). Tech notes, tactical BDs and
ooparts follow, one line a kind, their four tiers left to right on grey, blue, yellow and
purple: each by its icon (hover for its name) with what is needed / held, green when
enough and red when short. A student SchaleDB does not
list yet, or a value not read, is named as what could not be counted. The exp within a
level is not read, so a student partway to the next level takes a little less than shown;
equipment is counted levelled to the top of each tier before it is made into the next.

Under them, 不足 lists what falls short and how much of it what you hold makes up: the
choice boxes the item list counts (SchaleDB's, `data/schaledb/boxes.json`; a box of a
few schools is opened before a box of any school), and for elephs the 神名のカケラ when
they are enough for every eleph the plan buys. 残り is what is still short (補える when
nothing is), for the shops to make up.

The お仕事 tab weighs the daily plays against what is held, from the item and equipment
lists and the credits read:

- 任務: for each equipment slot, how many pieces from T1 to T10 the blueprints held make,
  the T10 blueprints apart from the universal ones (those of T9 and below counted in the
  universal ones they are made from: 2 for a T2 … 30 for a T9), red short of one piece
  and orange short of the golden ratio (オニア先生の黄金比: 帽子 3, 手袋 2, 靴 4, バッグ 2,
  バッジ 2, ヘアピン 4, お守り 1, 腕時計 4, ネックレス 2 students). Under them, how many times
  to play each normal mission, in the doubled-drop campaign, to stock the golden ratio
  with the fewest AP, by SchaleDB's drop rates (a linear program; blueprints the universal ones
  held make count), the totals under a double rule.
- 指名手配 and 学園交流会: what each place drops, tier by tier, with how many are held; in
  指名手配, red short of what one student takes (skills and potential raised fully; the
  median over the students that take it), blue short of two and green from two on.
- 特別依頼: how many students raised from Lv1 to Lv90, skills 5/10/10/10, equipment
  T10 and the unique weapon's ★4 at Lv60 the credits held pay for, and how many Lv1 to 90 the reports held level; the more
  shows green, the fewer, the commission to play, orange.

Each of these opens and closes as a shop does.

The stages come from SchaleDB as `src-tauri/data/schaledb/stages.json`
(`tools/schaledb/stages.ps1`, run by `update.ps1`).

A tab stays as you pick it, whatever the game shows. The menu (☰) at the top right
writes the YAML export (below), checks for an update, and chooses the theme: ライト, ダーク,
or 自動, which follows Windows. アップデートを確認 asks GitHub for the latest release; when
it is newer, 更新して再起動 downloads its exe, checks it against the SHA-256 published
with it, puts it in place of the running one, and starts it.

Under the 装備 tab each slot's pieces T1 to T10 come first, then its blueprints T2 to T10
and its universal blueprint, each tile with its tier at the bottom left.

The credits on the game's top bar are read on any screen that shows the bar (a dialog
dimming the screen pauses it) and count in the 育成 tab's plan and the お仕事 tab's 特別依頼.

The badge at the top right tells what Aoi sees:

| Badge                                       | Meaning                                                                          |
| ------------------------------------------- | -------------------------------------------------------------------------------- |
| ゲーム未検出                                | Blue Archive is not running.                                                     |
| 対象外の画面                                | The game shows another screen, or the list in another order. Nothing is counted. |
| アイテムを集計中 N fps / 装備を集計中 N fps | That list is being read, N frames a second.                                      |
| 生徒を集計中 N fps                          | A student's screen is being read, N frames a second.                             |
| ショップを集計中 N fps                      | A shop is being read, N frames a second.                                         |
| キャプチャ失敗                              | Windows refused to capture the game's window; the reason is shown below.         |
| 一時中断                                    | You paused the reading; nothing is read until you resume it.                     |

Press the badge to pause the reading (its lamp turns red); press it again to resume. A
shop screen left still costs next to nothing to read: a frame whose cards are the same to
the pixel as the last one's reads as it did.

Other windows may cover the game; Aoi reads the game window itself, not the screen.

### What is counted

- A count is taken once the same number has been read in three frames in a row, so a
  single misread frame does not change it. When you use an item in the game, its count
  follows when the list is shown again. An item used up leaves the game's list; Aoi drops
  it once the entries before and after it show side by side, or, for the list's first
  entries, once the list is scrolled to its top.
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
- A shop's goods are told by their icon and their name; goods Aoi cannot tell for sure
  are left out. How many more times one can be bought is taken once three frames in a
  row read alike; a price Aoi cannot read shows ? until a frame reads it.

### What is kept

Whenever something read changes, Aoi writes everything read so far (students, items,
equipment and shops) to `Documents\Aoi\aoi.yaml`, headed by when each list was last read, and keeps
it for its next run with the students marked for training and their targets: a list read
again replaces what it showed, entry by entry. Hand the
file to an agent or a tool to check what Aoi read or to plan with it. **YAMLを書き出す**,
in the menu, writes the same as `Documents\Aoi\aoi-<date>-<time>.yaml`, to keep how
things stood at that moment:

<!-- prettier-ignore -->
```yaml
credits: 244213276
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
shops:
  - name: "総力戦"
    read_at: "2026-10-12 22:10:00"
    offers:
      - id: 9999
        name: "秘伝ノート"
        coin: "総力戦コイン"
        left: 0  # how many more times it can be bought
        price: 500  # the shop's terms, null where they are unknown
        amount: 1  # how many come with a purchase
        limit: 3  # how many purchases a month allows
```

`null` marks a field Aoi could not read, apart from what the student lacks. A student
SchaleDB does not list yet has `id: null`.

## Item data

Names and icons of items and equipment, and student names, come from a snapshot of
[SchaleDB](https://schaledb.com) built into the exe. A few items and students SchaleDB
lacks, and a few icons the game draws unlike SchaleDB, are added in
`src-tauri/data/extra`, cut from the game; a student added there is told by name alone
until SchaleDB lists them. The training plan counts with the students' skill materials
and the equipment recipes from the same snapshot, and with the level tables of the game
(`growth.json`, from [electricgoat/ba-data](https://github.com/electricgoat/ba-data)) and
the skill credits the archived SchaleDB site listed. `tools/schaledb/update.ps1` refreshes
the snapshot and the tables.

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
