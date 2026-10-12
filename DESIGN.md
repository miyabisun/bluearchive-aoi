---
version: alpha
name: Sumi / bluearchive-aoi
description: ゲームの横に置いて集計を見るだけの小さな窓のデザイン。
colors:
  primary: "#1a5fa8"
  accent: "#1a5fa8"
  surface: "#faf6ef"
  on-surface: "#3a2f28"
  muted: "#6f6257"
  border: "#e3d9c9"
  danger: "#9c2b1d"
  danger-subtle: "#f9e9e4"
  wash-base: "#f6efe0"
typography:
  title:
    fontFamily: system-ui
    fontSize: 17px
    fontWeight: 600
    lineHeight: 1.3
  body:
    fontFamily: system-ui
    fontSize: 16px
    fontWeight: 400
    lineHeight: 1.6
  body-sm:
    fontFamily: system-ui
    fontSize: 14px
    fontWeight: 400
    lineHeight: 1.5
  caption:
    fontFamily: system-ui
    fontSize: 12px
    fontWeight: 400
    lineHeight: 1.4
rounded:
  sm: 6px
  full: 9999px
spacing:
  sp-1: 4px
  sp-2: 8px
  sp-3: 12px
  sp-4: 16px
  sp-5: 24px
components:
  app-header:
    backgroundColor: "{colors.wash-base}"
    textColor: "{colors.on-surface}"
    height: 48px
  status-badge:
    textColor: "{colors.muted}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.full}"
  error-banner:
    backgroundColor: "{colors.danger-subtle}"
    textColor: "{colors.danger}"
    typography: "{typography.body-sm}"
    rounded: "{rounded.sm}"
    padding: 12px
---

# bluearchive-aoi

## Overview

ゲームの横に置き、アイテム一覧・装備一覧をスクロールする間に読めた個数と、生徒詳細を送る間に読めた
育成状況を見るだけの窓。操作はタブの切り替えと名前の検索だけで、主役は集計表と、いま読めているかの状態である。

Sumi family の共通原本（rust-svelte-template の DESIGN.md、2026-10-11 参照）から派生した。
派生後はこの文書を正とし、原本の更新は差分を見て明示的に取り込む。

## Colors

暗色は Sumi、明色は Kinari とし、OS の明暗設定に従う。手動の切り替えは持たない
（メニューを置かない小さな窓のため）。`:root` を Sumi にし、`prefers-color-scheme: light` で Kinari にする。

| 役割 | Kinari | Sumi |
|---|---|---|
| surface | #faf6ef | #191919 |
| on-surface | #3a2f28 | #e6e6e6 |
| muted | #6f6257 | #9a9a9a |
| border | #e3d9c9 | #333333 |
| accent / primary | #1a5fa8 | #5cb3ff |
| danger | #9c2b1d | #ff6b6b |
| danger-subtle | #f9e9e4 | #3a1a1a |
| wash-base | #f6efe0 | #232323 |

アクセントは原本の金色からブルーアーカイブの青へ変えた。
コントラストは accent が Kinari の surface に 6.0:1、Sumi の surface に 7.8:1。
アクセントの用途はフォーカスの輪郭、入力欄のフォーカス枠、読めている状態の点、選択中のタブの下線だけとする。面を塗らない。
状態は色だけに頼らず、バッジの文字で示す。失敗は danger の文字で原因を書く。

## Typography

書体は system-ui。アプリ名は title（17px / 600）、表と案内は body（16px）、
状態バッジ・列見出し・失敗の原因は body-sm（14px）、検索欄のラベルは caption（12px、muted）。
個数は等幅数字で右にそろえ、桁区切りを入れる。

## Layout

- app header は全幅・高さ 48px とし、左にアプリ名、右に状態バッジだけを置く。
  その下に一覧のタブ（高さ 40px）を並べ、header とタブは常に上に見え、wash-base と 1px の下境界を使う。
- タブは文字だけのボタンで、選択中は 2px の accent の下線・600・on-surface、他は muted。面を塗らない。
  「生徒」タブには読み取った人数を muted・400 の等幅数字で添える（「生徒 42」、0 人のときは出さない。2026-10-12 の user の
  依頼「スキャンした全生徒数を管理してヘッダー部で出して欲しい」）。
  ゲームが別の一覧を映し始めたら、そのタブへ移る（手で選んだタブは、次に一覧が変わるまで保つ）。
- タブの行の右端に「書き出す」を置く（2026-10-12 の user の依頼「YAML書き出し」）。高さ 28px、1px の border の枠だけで面を塗らず、
  文字は on-surface（押せないときは muted）。結果は本文の先頭に body-sm・muted で示し、失敗は error-banner で原因を書く。
- 装備の名前の前にティア（T1〜T10）を body-sm・muted で添える。設計図は名前だけではティアが分からないため。
- 本文は中央配置で最大 720px、左右 12px、上 16px の余白。ページ自体はスクロールせず、上部と検索欄を残して表だけを枠の中で
  スクロールし、列見出しを枠の上端に固定する（2026-10-12 の user の依頼「表の1行目はヘッダーとして固定する」）。
  見出しは surface の面で下の行を隠す。
- 生徒の表は 9 列あり 720px にも収まらないため、同じ枠で横にもスクロールする。数値の列は折り返さず、
  生徒名の列だけ折り返す。表記は timeline-arona に合わせ、スキルは「5M7M」、装備は「T10/9/8」と詰めて書く。
  読めない欄は「?」、生徒が持たないもの（未解放の固有武器、愛用品なし）は「—」で区別する。
- 生徒の表の列見出し（生徒名を除く）は押すとその列の降順に並べ替えるボタンにする（2026-10-12 の user の依頼
  「ヘッダークリックで降順で並べ直す」）。見た目は見出しの文字のままとし、並べている列だけ on-surface・600 にして
  caption の「▼」を添える（aria-sort も付ける）。
- 集計が無いときは表と検索欄を出さず、読ませる方法を一文で示す。
  検索の不一致は「一致する項目がありません」と示し、集計が空の場合と区別する。
- キャプチャの失敗は header の下に error-banner で原因を出す。通常時に場所を予約しない。
- 幅 320px でもページは横にはみ出さない。長い名前は折り返し、個数の列は折り返さない。

## Shapes

角丸は入力欄・ボタン・error-banner に 6px、状態バッジに 9999px。影は使わない。
