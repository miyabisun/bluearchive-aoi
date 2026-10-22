// トップ 1 本目「はじめる」。書式は dotfiles の animated-arch-diagram skill の SKILL.md
// 色の名前がほかの図とぶつからないよう、ブロックの中に置く
{
  const GAME = "#d9668c"; // ゲーム・スマホの側
  const AOI = "#3b8fd9"; // Aoi

  window.SPECS = window.SPECS || {};
  window.SPECS.start = {
    title: "はじめる",
    grid: [3, 3],
    nodes: [
      { id: "pc", label: "PC 版", sub: "Windows", color: GAME, col: 0, row: 0, w: 380 },
      { id: "android", label: "Android", sub: "USB・Wi-Fi", color: GAME, col: 0, row: 1, w: 380 },
      { id: "iphone", label: "iPhone", sub: "配信アプリ", color: GAME, col: 0, row: 2, w: 380 },
      { id: "menu", label: "Aoi の ≡", sub: "受け取り方を選ぶ", color: AOI, col: 1, row: 1 },
      { id: "start", label: "録画開始", sub: "画面を受け取る", color: AOI, col: 2, row: 1 },
    ],
    edges: [
      { from: "pc", to: "menu" },
      { from: "android", to: "menu" },
      { from: "iphone", to: "menu" },
      { from: "menu", to: "start" },
    ],
    steps: [
      {
        kicker: "ゲーム",
        caption: "PC 版・Android・iPhone のどれかで遊ぶ",
        focus: ["pc", "android", "iphone"],
      },
      {
        kicker: "接続",
        caption: "Aoi の右上の ≡ から、受け取り方を選ぶ",
        flow: [
          ["pc", "menu"],
          ["android", "menu"],
          ["iphone", "menu"],
        ],
        token: "ゲーム",
        hop: 0.6,
        note: {
          node: "menu",
          side: "above",
          lines: [
            "PC 版: 選ばなくてよい",
            "Android: Androidとの通信",
            "iPhone: 配信アプリとの連携",
          ],
        },
      },
      {
        kicker: "録画開始",
        caption: "画面を送り始めると、Aoi が読み始める",
        flow: [["menu", "start"]],
        token: "画面",
        note: {
          node: "start",
          side: "below",
          lines: [
            "PC 版: ゲームを起動するだけ",
            "Android: USB か QR でつなぐ",
            "iPhone: ブロードキャストを開始",
          ],
        },
      },
    ],
    outro: { kicker: "準備完了", caption: "あとはゲームで一覧を開くだけ" },
  };
}
