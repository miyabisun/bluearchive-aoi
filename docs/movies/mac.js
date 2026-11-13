// 「macOS で初めて開く」の流れ。書式は dotfiles の animated-arch-diagram skill の SKILL.md
// 色の名前がほかの図とぶつからないよう、ブロックの中に置く
{
  const MAC = "#d9668c"; // Mac での操作
  const AOI = "#3b8fd9"; // Aoi

  window.SPECS = window.SPECS || {};
  window.SPECS.mac = {
    title: "初めて開く流れ",
    grid: [5, 1],
    nodes: [
      { id: "dmg", label: "ダウンロード", sub: ".dmg", color: MAC, col: 0, row: 0 },
      { id: "app", label: "ドラッグ", sub: "アプリケーションへ", color: MAC, col: 1, row: 0 },
      { id: "open", label: "ダブルクリック", sub: "開けない表示", color: MAC, col: 2, row: 0 },
      { id: "allow", label: "システム設定", sub: "「開く」を押す", color: MAC, col: 3, row: 0 },
      { id: "aoi", label: "Aoi", sub: "起動する", color: AOI, col: 4, row: 0 },
    ],
    edges: [
      { from: "dmg", to: "app" },
      { from: "app", to: "open" },
      { from: "open", to: "allow" },
      { from: "allow", to: "aoi" },
    ],
    steps: [
      {
        kicker: "入れる",
        caption: "dmg を開いて、「アプリケーション」へドラッグする",
        flow: [["dmg", "app"]],
        token: "Aoi",
      },
      {
        kicker: "開く",
        caption: "ダブルクリックする。開けない表示は閉じる",
        flow: [["app", "open"]],
        token: "Aoi",
        note: { node: "open", side: "below", lines: ["「ゴミ箱に入れる」は", "押さない"] },
      },
      {
        kicker: "許可",
        caption: "「プライバシーとセキュリティ」で「開く」を押す",
        flow: [["open", "allow"]],
        token: "許可",
        note: {
          node: "allow",
          side: "above",
          lines: ["ボタンが出るのは約 1 時間", "消えたらダブルクリックから"],
        },
      },
      {
        kicker: "確認",
        caption: "「このまま開く」を押して、Mac のパスワードを入れる",
        focus: ["allow"],
      },
      {
        kicker: "起動",
        caption: "Aoi が開く。あとはスマホをつなぐ",
        flow: [["allow", "aoi"]],
        token: "Aoi",
        note: { node: "aoi", side: "below", lines: ["つなぐまでは", "「取り込み元なし」"] },
      },
    ],
    outro: { kicker: "2 回目から", caption: "許可は、初めて開くときだけ" },
  };
}
