---
title: macOS で初めて開く
---

# macOS で初めて開く

macOS 版の Aoi は、初めて開くときだけ許可が要ります。

<div class="movie" data-spec="mac"></div>

## 入れる

1. [最新版のリリースページ](https://github.com/miyabisun/bluearchive-aoi/releases/latest) の「Assets」から、`.dmg` のファイルをダウンロードする
2. ダウンロードしたファイルを開き、`bluearchive-aoi` を「アプリケーション」フォルダへドラッグする

## 初めて開く（macOS 13 Ventura 以降）

1. 「アプリケーション」フォルダの `bluearchive-aoi` をダブルクリックし、開けないという表示を閉じる
2. 「システム設定」＞「プライバシーとセキュリティ」の「セキュリティ」の欄で、「開く」を押す
3. 確認で「このまま開く」を押し、Mac のログインパスワードを入れる

## 開いたあと

読むのは、スマホから送った画面です（[Android](android.md)・[iPhone](stream.md)）。Mac でスマホを読む使い方は、まだ確かめていません。

<details markdown="1">
<summary>なぜ許可が要るのか、なぜスマホの画面なのか</summary>

- macOS 版の Aoi は、Apple の公証を受けていません。そのため、初めて開くときだけ許可が要ります。
- ブルーアーカイブの PC 版は Windows 専用です。そのため macOS 版は、スマホから送った画面を読みます。
- スマホがつながるまでは、右上に「取り込み元なし」と出ます。
- ファイルの名前は `bluearchive-aoi-universal-apple-darwin.dmg` です（Apple シリコン・Intel 共通）。

</details>

<details markdown="1">
<summary>ボタンが見つからないとき、古い macOS のとき</summary>

- 開けないという表示では、「ゴミ箱に入れる」は押しません。
- 「システム設定」は、アップルメニューから開きます。「開く」のボタンは、バージョンによっては「このまま開く」です。
- 「開く」のボタンは、ダブルクリックして開こうとしてから約 1 時間だけ出ます。消えていたら、ダブルクリックからやり直します。
- パスワードを入れたら「OK」を押します。
- macOS 13 Ventura と 14 Sonoma では、Finder からも開けます。Control キーを押しながら `bluearchive-aoi` をクリックして「開く」を選び、出てきた確認で「開く」を押します。
- macOS 12 Monterey 以前は、アップルメニュー ＞「システム環境設定」＞「セキュリティとプライバシー」＞「一般」にある「このまま開く」を押します。
- 手順は Apple の [開発元が不明な Mac アプリを開く](https://support.apple.com/ja-jp/guide/mac-help/mh40616/mac) に沿っています。

</details>

<details markdown="1">
<summary>新しいバージョンにする</summary>

1. Aoi の右上の ≡ で設定を開き、いちばん上の「アップデートを確認」を押す
2. 新しいバージョンがあると出る「入手ページを開く」を押す
3. 開いたページから、上の「入れる」と同じように入れ直す（「アプリケーション」フォルダの古い `bluearchive-aoi` を置き換えます）

新しいバージョンも、初めて開くときに上の許可を求められることがあります。

</details>
