// 「Android のスマホの画面を読む」の流れ。書式は dotfiles の animated-arch-diagram skill の SKILL.md
// 色の名前がほかの図とぶつからないよう、ブロックの中に置く
{
  const GAME = "#d9668c"; // スマホでの操作
  const AOI = "#3b8fd9"; // Aoi

  window.SPECS = window.SPECS || {};
  window.SPECS.android = {
    title: "Android とつなぐ流れ",
    grid: [4, 2],
    nodes: [
      {
        id: "phone",
        label: "スマホの設定",
        sub: "ビルド番号を 7 回",
        color: GAME,
        col: 0,
        row: 0.5,
      },
      { id: "menu", label: "Aoi の ≡", sub: "Androidとの通信", color: AOI, col: 1, row: 0.5 },
      { id: "usb", label: "USB", sub: "USB デバッグ", color: GAME, col: 2, row: 0 },
      { id: "wifi", label: "Wi-Fi", sub: "QR でペア設定", color: GAME, col: 2, row: 1 },
      { id: "recv", label: "Aoi", sub: "画面を受け取る", color: AOI, col: 3, row: 0.5 },
    ],
    edges: [
      { from: "phone", to: "menu" },
      { from: "menu", to: "usb" },
      { from: "menu", to: "wifi" },
      { from: "usb", to: "recv" },
      { from: "wifi", to: "recv" },
    ],
    steps: [
      {
        kicker: "準備",
        caption: "ビルド番号を 7 回タップする（初回だけ）",
        focus: ["phone"],
        note: { node: "phone", side: "below", lines: ["設定 → デバイス情報", "→ ビルド番号"] },
      },
      {
        kicker: "通信",
        caption: "Aoi の ≡ から「Androidとの通信」を押す",
        flow: [["phone", "menu"]],
        token: "スマホ",
      },
      {
        kicker: "USB",
        caption: "USB デバッグをオンにして、ケーブルでつなぐ",
        flow: [
          ["menu", "usb"],
          ["usb", "recv"],
        ],
        token: "USB",
        note: { node: "usb", side: "above", lines: ["「常に許可する」に印", "→「許可」を押す"] },
      },
      {
        kicker: "Wi-Fi",
        caption: "ワイヤレス デバッグの画面で、QR コードを読む",
        flow: [
          ["menu", "wifi"],
          ["wifi", "recv"],
        ],
        token: "QR",
        note: { node: "wifi", side: "below", lines: ["カメラアプリでは読まない"] },
      },
      {
        kicker: "完了",
        caption: "「…の画面を USB で受け取っています」と出る",
        focus: ["recv"],
      },
    ],
    outro: { kicker: "2 回目から", caption: "つなぐだけで、自動でつながる" },
  };
}
