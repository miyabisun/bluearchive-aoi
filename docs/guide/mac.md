---
title: macOS で初めて開く
---

# macOS で初めて開く

macOS 版の Aoi は Apple の公証を受けていないので、初めて開くときだけ許可が要ります。ブルーアーカイブの PC 版は Windows 専用なので、macOS 版はスマホから送った画面を読みます（[Android](android.md)、[iPhone](stream.md)）。スマホがつながるまでは、右上に「取り込み元なし」と出ます。

## 入れる

1. [最新版のリリースページ](https://github.com/miyabisun/bluearchive-aoi/releases/latest) の「Assets」から `bluearchive-aoi-universal-apple-darwin.dmg` をダウンロードする（Apple シリコン・Intel 共通）
2. ダウンロードしたファイルを開き、`bluearchive-aoi` を「アプリケーション」フォルダへドラッグする

## 初めて開く（macOS 13 Ventura 以降）

1. 「アプリケーション」フォルダの `bluearchive-aoi` をダブルクリックする。開けないという表示が出たら閉じる（「ゴミ箱に入れる」は押しません）
2. アップルメニュー ＞「システム設定」＞「プライバシーとセキュリティ」を開き、「セキュリティ」の欄の「開く」（バージョンによっては「このまま開く」）を押す
3. 確認で「このまま開く」を押し、Mac のログインパスワードを入れて「OK」を押す

<details markdown="1">
<summary>ボタンが見つからないとき、古い macOS のとき</summary>

- 「開く」のボタンは、手順 1 で開こうとしてから約 1 時間だけ出ます。消えていたら手順 1 からやり直します。
- macOS 13 Ventura と 14 Sonoma では、Finder からも開けます。Control キーを押しながら `bluearchive-aoi` をクリックして「開く」を選び、出てきた確認で「開く」を押します。
- macOS 12 Monterey 以前は、アップルメニュー ＞「システム環境設定」＞「セキュリティとプライバシー」＞「一般」にある「このまま開く」を押します。
- 手順は Apple の [開発元が不明な Mac アプリを開く](https://support.apple.com/ja-jp/guide/mac-help/mh40616/mac) に沿っています。

</details>

<details markdown="1">
<summary>新しいバージョンにする</summary>

Aoi の右上の ≡ から「アップデートを確認」を押します。新しいバージョンがあると「入手ページを開く」のボタンが出ます。押して開いたページから、上の「入れる」と同じように入れ直し、「アプリケーション」フォルダの古い `bluearchive-aoi` を置き換えます。新しいバージョンも、初めて開くときに上の許可を求められることがあります。

</details>
