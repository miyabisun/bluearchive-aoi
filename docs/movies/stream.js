// 「配信アプリでスマホの画面を送る」の流れ。書式は dotfiles の animated-arch-diagram skill の SKILL.md
// 色の名前がほかの図とぶつからないよう、ブロックの中に置く
{
  const GAME = "#d9668c"; // iPhone での操作
  const AOI = "#3b8fd9"; // Aoi

  window.SPECS = window.SPECS || {};
  window.SPECS.stream = {
    title: "iPhone から送る流れ",
    grid: [5, 2],
    nodes: [
      { id: "qr", label: "Aoi のメニュー", sub: "配信アプリとの連携", color: AOI, col: 0, row: 1 },
      { id: "moblin", label: "Moblin", sub: "QR から開く", color: GAME, col: 1, row: 0 },
      { id: "scene", label: "Scenes", sub: "Screen capture", color: GAME, col: 2, row: 0 },
      { id: "rec", label: "画面収録", sub: "ブロードキャスト", color: GAME, col: 3, row: 0 },
      { id: "recv", label: "Aoi", sub: "画面を受け取る", color: AOI, col: 4, row: 1 },
    ],
    edges: [
      { from: "qr", to: "moblin" },
      { from: "moblin", to: "scene" },
      { from: "scene", to: "rec" },
      { from: "rec", to: "recv" },
    ],
    steps: [
      {
        kicker: "QR",
        caption: "Aoi の右上のメニューから「配信アプリとの連携」を押す",
        focus: ["qr"],
        note: { node: "qr", side: "above", lines: ["QR コードと", "rtmp:// の URL が出る"] },
      },
      {
        kicker: "Moblin",
        caption: "iPhone のカメラで QR を読み、Moblin で開く",
        flow: [["qr", "moblin"]],
        token: "QR",
        note: {
          node: "moblin",
          side: "below",
          lines: ["配信先「Aoi」が入る", "H.264・2560x1440・15 fps"],
        },
      },
      {
        kicker: "シーン",
        caption: "映像を「Screen capture」にする（初回だけ）",
        flow: [["moblin", "scene"]],
        token: "設定",
      },
      {
        kicker: "収録",
        caption: "画面収録を長押しして、ブロードキャストを開始",
        flow: [["scene", "rec"]],
        token: "収録",
        note: { node: "rec", side: "below", lines: ["コントロールセンターで", "Moblin を選ぶ"] },
      },
      {
        kicker: "配信",
        caption: "Moblin の配信ボタンを押して、ゲームを開く",
        flow: [["rec", "recv"]],
        token: "画面",
        note: { node: "recv", side: "above", lines: ["「…から画面を", "受け取っています」"] },
      },
    ],
    outro: { kicker: "完了", caption: "あとは PC と同じように、一覧を開いて送る" },
  };
}
