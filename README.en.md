# Aoi

<p align="center"><img src="docs/aoi.png" width="320" alt="Aoi, holding a monocle and a finance binder"></p>

Aoi counts your Blue Archive items and equipment, notes how far your students are
trained and how many more times the shops sell each of their goods. Keep it open next
to the game and scroll through the item list, the equipment list or a shop, or step
through your students' screens: Aoi reads what the game's window shows and lists it.
From what it has counted it works out what the students you train still lack, and
lets you choose how to make it up from the choice boxes and the shops.

It only looks: it sends no input to the game. What it reads stays on this computer; it
goes online only to look for and download a newer version when you ask it to.

日本語: [README.md](README.md)

## Requirements

- The Japanese PC version of Blue Archive on Windows 11, or the Japanese Android or iPhone
  version, with Aoi on Windows 11 or macOS (see below). Names are shown in Japanese.

Aoi also reads the Android version of the game: the phone sends its screen over a USB cable
or Wi-Fi, with no app on the phone (USB or wireless debugging, Android 11 or later; steps in
Japanese: [docs/guide/android.md](docs/guide/android.md)). An iPhone sends its screen
through a streaming app (Moblin) over RTMP on the same Wi-Fi, as any phone can; this is not
tried on an iPhone yet (steps in Japanese: [docs/guide/stream.md](docs/guide/stream.md)).

There is a macOS version too, which reads a phone's screen only: the PC version of Blue
Archive runs on Windows only. It is `bluearchive-aoi-universal-apple-darwin.dmg` in the same release,
for Apple silicon and Intel Macs. The app is not notarized, so macOS blocks its first
start: open it once, then choose Open Anyway in System Settings → Privacy & Security
(steps in Japanese: [docs/guide/mac.md](docs/guide/mac.md)). Its update check opens the
release page instead of replacing the app.

## Download and run

Download `bluearchive-aoi-x86_64-pc-windows-msvc.exe` from the
[latest release](https://github.com/miyabisun/bluearchive-aoi/releases/latest), put it in
a folder you can write to (your desktop or Documents, not Program Files) and run it.
There is nothing to install. If Windows SmartScreen stops it, choose "More info", then
"Run anyway": the exe is not signed.

Later versions come through the app: ☰ → アップデートを確認, then 更新して再起動.

## Use

Start Aoi and the game, in either order. In the game, open the item or equipment list
sorted by デフォルト ascending, a student's 基本情報 tab, or a shop, and scroll or step
through it slowly. The step-by-step guides are in Japanese, under
[docs/guide](docs/guide): [reading the game](docs/guide/reading.md),
[the 育成 tab](docs/guide/plan.md), [the お仕事 tab](docs/guide/work.md) and
[troubleshooting](docs/guide/trouble.md).

## For developers

See [docs/dev](docs/dev/README.md).

## License

MIT ([LICENSE](LICENSE)). The bundled scrcpy server is under the Apache License 2.0
([src-tauri/vendor/scrcpy](src-tauri/vendor/scrcpy)).
