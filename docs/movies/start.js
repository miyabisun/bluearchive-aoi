// トップ 1 本目「起動方法」。書式は dotfiles の animated-arch-diagram skill の SKILL.md
// 色の名前がほかの図とぶつからないよう、ブロックの中に置く
{
  const GAME = "#d9668c"; // ゲーム・スマホの側
  const AOI = "#3b8fd9"; // Aoi

  window.SPECS = window.SPECS || {};
  window.SPECS.start = {
    title: "起動方法",
    grid: [3, 3],
    nodes: [
      { id: "aoi", label: "Aoi", sub: "起動する", color: AOI, col: 0, row: 1, w: 360 },
      { id: "pc", label: "PC 版", sub: "ゲームを起動", color: GAME, col: 1, row: 0, w: 360 },
      { id: "android", label: "Android", sub: "USB・Wi-Fi", color: GAME, col: 1, row: 1, w: 360 },
      { id: "iphone", label: "iPhone", sub: "動画配信アプリ", color: GAME, col: 1, row: 2, w: 360 },
      { id: "start", label: "録画開始", sub: "画面を受け取る", color: AOI, col: 2, row: 1, w: 360 },
    ],
    edges: [
      { from: "aoi", to: "pc" },
      { from: "aoi", to: "android" },
      { from: "aoi", to: "iphone" },
      { from: "pc", to: "start" },
      { from: "android", to: "start" },
      { from: "iphone", to: "start" },
    ],
    steps: [
      {
        kicker: "起動",
        caption: "まず Aoi を起動する",
        focus: ["aoi"],
      },
      {
        kicker: "PC 版",
        caption: "PC 版は、ゲームを起動すれば自動で見つける",
        flow: [
          ["aoi", "pc"],
          ["pc", "start"],
        ],
        token: "画面",
        note: { node: "pc", side: "left", lines: ["メニューで選ばなくてよい"] },
      },
      {
        kicker: "スマホ",
        caption: "スマホは、Aoi の右上のメニューで受け取り方を選ぶ",
        focus: ["aoi"],
        note: {
          node: "aoi",
          side: "below",
          lines: ["Android: Androidとの通信", "iPhone: 配信アプリとの連携"],
        },
      },
      {
        kicker: "Android",
        caption: "Android は USB か Wi-Fi でつなぐ",
        flow: [
          ["aoi", "android"],
          ["android", "start"],
        ],
        token: "USB・Wi-Fi",
        note: { node: "start", side: "above", lines: ["USB デバッグ、または QR でペア設定"] },
      },
      {
        kicker: "iPhone",
        caption: "iPhone は動画配信アプリで画面を送る",
        flow: [
          ["aoi", "iphone"],
          ["iphone", "start"],
        ],
        token: "配信",
        note: { node: "start", side: "below", lines: ["Moblin でブロードキャストを開始"] },
      },
    ],
    outro: { kicker: "準備完了", caption: "あとはゲームで一覧を開くだけ" },
  };
}
