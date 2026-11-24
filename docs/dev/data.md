# Data and file formats

## Item data

Names and icons of items and equipment, and student names, come from a snapshot of
[SchaleDB](https://schaledb.com) built into the exe (`data/schaledb`). A few
items and students SchaleDB lacks, and a few icons the game draws unlike SchaleDB, are
added in `data/extra`, cut from the game; a student added there is told by name
alone, under an id of Aoi's own below zero, until SchaleDB lists them. Then they go by
SchaleDB's id and what was read of them moves with them; they stay in the list, as each
student there takes the id of their place in it.

The training plan counts with the students' skill materials and the equipment recipes
from the same snapshot, and with the level tables of the game (`growth.json`, from
[electricgoat/ba-data](https://github.com/electricgoat/ba-data)) and the skill credits the
archived SchaleDB site listed. The choice boxes (`boxes.json`) and the daily stages
(`stages.json`: normal and hard missions, bounties, scrimmages) come from SchaleDB too, as
do the stars each student comes with (`students.json`'s `star`, which orders the hard
missions' elephs), and so do the schools' emblems the item tab's
skill materials show (`school-icons`, white, drawn in the theme's text colour by a
filter). `tools/schaledb/update.ps1` refreshes the
snapshot and the tables (it runs `stages.ps1` and `boxes.ps1`). New students come with
Wednesday's maintenance and reach SchaleDB some days later: every day the SchaleDB workflow
(`.github/workflows/schaledb.yml`) runs the script, rebuilds the glyph banks where the
students changed (`build_glyphs`, so their names read), and commits the change to main.
It ships with the next release, which the pre-push gate tests; but where the students
changed, the workflow releases them the same day, untested (the user's choice of
2026-10-30): `tools/release/bump-patch.ps1` raises the patch version, the workflow commits
"release: vX.Y.Z" with its tag to main and dispatches the release workflow, which its own
pushes would not start.

The shops' prices, how many come with a purchase and the monthly limits are written in
`data/shops.yaml`, from the [Blue Archive wiki](https://bluearchive.wiki/wiki/Shop);
a good they do not cover shows `@?`. In カケラ(文字) an eleph costs its student's rate,
read off the card (1 to 5 fragments).

## Reading text

Aoi reads text without an OCR engine, so it reads alike on any system. Short texts (a
level, a tier, the credits, a card's times left and price) are read a character at a time
against references cut from the labeled fixtures; the students' names and the shops'
names are told among the names that can be there, whole, and so is the title at a screen's top
left ("学園交流会"), which tells the work tab where the game is when no list, student or shop is read
(the titles are taken from the game's own drawing in `tests/fixtures/titles.json`; the 任務
screen, "任務" on its Normal and Hard tabs alike, is told by its campaign's label and text, read so
too up to the rate, and the shop screen of a shop whose offers are not read, 通常アイテム, by the
name its menu has chosen; the schedule past its list of locations by the location's name at the top
right of the screen "スケジュールを選択", whose title no "?" button follows, and under its
"すべてのスケジュール" dialog, which dims the title, by the first room's name on the top left card,
both labeled in `tests/fixtures/schedule.json`). The names under the student list's cards are
told as the name on a student's plate is, with the plate's letters; the list spaces the
letters otherwise than the font the names' bounds come from, so each is cut at the gap
beside it (`Ink::letters_at_gaps`). A video keeps the colour at half
the resolution of the brightness (4:2:0), so the student screen's coloured texts (a tier, an
ability release, the weapon level's white fill) are cut out by their brightness; the colour only
tells where a badge is and whether there is one, over its whole area. An equipment slot's level
is not read: the slot is taken at its tier's top level.

The references are a glyph bank for each width the texts are read at (`screen::WIDTHS`): grey
16x16 tiles in `data/glyphs-<width>.png`, and what each tile stands for in
`data/glyphs-<width>.json`. The students' names in it are
drawn with Noto Sans JP, the free font nearest the game's, with the game's own letters
from the fixtures standing in where there are some. Rebuild the bank after the fixtures or
SchaleDB's students change, with Noto Sans JP's Regular, Medium and Bold
(`NotoSansJP-*.otf` from
[noto-cjk](https://github.com/notofonts/noto-cjk/tree/main/Sans/SubsetOTF/JP), under the
SIL Open Font License) in a folder:

```powershell
cargo run --release --example build_glyphs -- <folder of the fonts>
```

A screen newly labeled in `tests/fixtures/titles.json` needs its title alone: `build_glyphs -- titles`
adds the titles the banks lack and leaves every other tile and word as it is (no font needed), where
drawing the banks anew would move other readings. `build_glyphs -- schedule` adds so the schedule's
locations and first rooms newly labeled in `tests/fixtures/schedule.json`.

The lists' counts are read as the other texts are, with a bank of their own
(`data/counts.png` and `.json`, one for every width), from the fixtures and from a collection of
a tablet's screens as scrcpy streams them (each with the screen as screencap gives it, which
labels them), kept outside the repository for its size:

```powershell
cargo run --release --example build_counts -- <collection>
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
- The order the students were accepted in is read from the student list (生徒リスト) while
  it shows every student sorted by it (the All tab, 受け入れ順, either direction, no filter
  and no search: `student::list`, labeled in `tests/fixtures/student-lists.json`). The
  student accepted first is 1 and each later one a number higher, so a number never changes
  as students join (`student::accepted`). The numbers start where the list shows it: with
  the newest first, at the held card before the first card not held (grey, under the banner
  未所属) or the first empty place; with the oldest first, at the list's first card, the
  scroll bar's thumb at its start. Any other frame counts from a student whose number is
  known, and the frames between are laid over one another by the students they name or by
  the cards' pictures. A place counts once three frames in a row show the same there; a card
  whose name is not told is still a place. A frame at odds with the numbers known is not
  taken, and no number given is changed.
- A shop's goods are told by their icon; icons too alike to tell apart (the tiers'
  blueprint boxes) by the price the shop sells each at. How many more times one can be
  bought is taken once three frames in a row read alike. A shop frame whose cards are the
  same to the pixel as the last one's reads as it did. All shops but カケラ(文字) reset as
  the month changes.
- The 育成 tab counts the marked students as they were read when Aoi started or 再読込 was
  pressed, not as they are read since; what is held (items, equipment, credits, the shops) it
  counts as read now, again with every reading. The snapshot's `plan_behind`
  (`ground::Ground::behind`) tells the window when the students were read and which marked
  ones were read otherwise since: those whose level, star, skills or equipment changed.
- A training or a purchase read as it happened is counted into the lists before they are read
  again (`ledger`, the user's rules of 2026-11-15). A star takes the student's elephs. A skill
  takes its materials, then the choice boxes that give one, those of fewest choices first (the
  game spends the six schools' box before the one of every school). An equipment tier takes
  its blueprints, then the slot's universal ones at `plan::RATE`, never the blueprints' boxes.
  A shop's times left gone down add what was bought (`shops.yaml`'s `amount` a purchase). A
  change is counted only where its old value was read in this run, in a frame that read it,
  and the list it counts in was not shown since: otherwise the list may show it already. An
  item never counted stays so; a count short of what was spent is forgotten (some were had
  unseen), and so is a count in thousands fallen under 10,000. A count in thousands keeps
  what its thousands leave out until the list is read again. Levels are not counted (reports,
  enhancement orbs, the unique weapon's parts): which of them were fed is not told.

## What is kept

Whenever something read changes, Aoi writes everything read so far to
`Documents\Aoi\aoi.yaml`, headed by when each list was last read, and keeps it for its
next run with the students marked for training and their targets
(`%APPDATA%\io.github.miyabisun.bluearchive-aoi\state.json`). A list read again replaces
what it showed, entry by entry. **データを保存**, on the settings page, writes the same as
`Documents\Aoi\aoi-<date>-<time>.yaml`, ending with a line (`aoi_state`) of everything kept; **データを復元**
puts back what a file chosen there kept and starts the app again.

<!-- prettier-ignore -->
```yaml
credits: 244213276
students:
  - id: 10059
    name: "ミカ"
    level: 90
    rarity: 5
    bond: 43
    index: 131  # the 131st student accepted; null until the student list told it
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
SchaleDB does not list yet has `id: null`. The numbers of students whose training was never
read are kept in `state.json` (`accepted`, by id) and written once the student is listed.
