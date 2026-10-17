# Data and file formats

## Item data

Names and icons of items and equipment, and student names, come from a snapshot of
[SchaleDB](https://schaledb.com) built into the exe (`src-tauri/data/schaledb`). A few
items and students SchaleDB lacks, and a few icons the game draws unlike SchaleDB, are
added in `src-tauri/data/extra`, cut from the game; a student added there is told by name
alone, under an id of Aoi's own below zero, until SchaleDB lists them. Then they go by
SchaleDB's id and what was read of them moves with them; they stay in the list, as each
student there takes the id of their place in it.

The training plan counts with the students' skill materials and the equipment recipes
from the same snapshot, and with the level tables of the game (`growth.json`, from
[electricgoat/ba-data](https://github.com/electricgoat/ba-data)) and the skill credits the
archived SchaleDB site listed. The choice boxes (`boxes.json`) and the daily stages
(`stages.json`) come from SchaleDB too. `tools/schaledb/update.ps1` refreshes the
snapshot and the tables (it runs `stages.ps1` and `boxes.ps1`). New students come with
Wednesday's maintenance: every Thursday the SchaleDB workflow
(`.github/workflows/schaledb.yml`) runs the script, rebuilds the glyph banks where the
students changed (`build_glyphs`, so their names read), and commits the change to main.
It ships with the next release, which the pre-push gate tests.

The shops' prices, how many come with a purchase and the monthly limits are written in
`src-tauri/data/shops.yaml`, from the [Blue Archive wiki](https://bluearchive.wiki/wiki/Shop);
a good they do not cover shows `@?`. In カケラ(文字) an eleph costs its student's rate,
read off the card (1 to 5 fragments).

## Reading text

Aoi reads text without an OCR engine, so it reads alike on any system. Short texts (a
level, a tier, the credits, a card's times left and price) are read a character at a time
against references cut from the labeled fixtures; the students' names and the shops'
names are told among the names that can be there, whole. A video keeps the colour at half
the resolution of the brightness (4:2:0), so the student screen's coloured texts (a tier, an
ability release, the weapon level's white fill) are cut out by their brightness; the colour only
tells where a badge is and whether there is one, over its whole area. An equipment slot's level
is not read: the slot is taken at its tier's top level.

The references are a glyph bank for each width the texts are read at (`screen::WIDTHS`): grey
16x16 tiles in `src-tauri/data/glyphs-<width>.png`, and what each tile stands for in
`src-tauri/data/glyphs-<width>.json`. The students' names in it are
drawn with Noto Sans JP, the free font nearest the game's, with the game's own letters
from the fixtures standing in where there are some. Rebuild the bank after the fixtures or
SchaleDB's students change, with Noto Sans JP's Regular, Medium and Bold
(`NotoSansJP-*.otf` from
[noto-cjk](https://github.com/notofonts/noto-cjk/tree/main/Sans/SubsetOTF/JP), under the
SIL Open Font License) in a folder:

```powershell
cd src-tauri
cargo run --release --example build_glyphs -- <folder of the fonts>
```

The lists' counts have references of their own (`src-tauri/data/digits.json`), from the
fixtures and from a collection of a tablet's screens as scrcpy streams them (each with the
screen as screencap gives it, which labels them), kept outside the repository for its size:

```powershell
cargo run --release --example build_digits -- <collection>
```

## What is counted

- A list is read only in its default order (デフォルト, ascending). A count is taken once
  the same number has been read in three frames in a row, so a single misread frame does
  not change it. An item used up leaves the game's list; Aoi drops it once the entries
  before and after it show side by side in three frames in a row, or, for the list's first
  entries, once the list is scrolled to its top. More than four items held lying between
  two cards side by side is taken for a misread, and drops nothing.
- Counts of 10,000 or more are read as the game shows them, in thousands: 51K stands for
  50,500 to 51,499.
- A count Aoi cannot read for sure is skipped in that frame rather than guessed. The
  selected item's count, under its blinking frame, is read in the frames where the frame
  is faint. Items Aoi cannot tell apart for sure are left out.
- Event items, from the event points to the Valentine chocolates, are never listed.
- A student's training is taken once three frames in a row read alike; a field read later
  keeps its known value when a frame cannot read it.
- A shop's goods are told by their icon; icons too alike to tell apart (the tiers'
  blueprint boxes) by the price the shop sells each at. How many more times one can be
  bought is taken once three frames in a row read alike. A shop frame whose cards are the
  same to the pixel as the last one's reads as it did. All shops but カケラ(文字) reset as
  the month changes.
- The 育成 tab counts from what was read when Aoi started or 再読込 was pressed, not from
  what is read since.

## What is kept

Whenever something read changes, Aoi writes everything read so far to
`Documents\Aoi\aoi.yaml`, headed by when each list was last read, and keeps it for its
next run with the students marked for training and their targets
(`%APPDATA%\io.github.miyabisun.bluearchive-aoi\state.json`). A list read again replaces
what it showed, entry by entry. **YAMLを書き出す**, in the menu, writes the same as
`Documents\Aoi\aoi-<date>-<time>.yaml`.

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
    equipment: [{tier: 10}, {tier: 10}, null]
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
