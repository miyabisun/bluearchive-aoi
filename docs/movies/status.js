// 「困ったとき」の右上の表示の移り変わり。書式は dotfiles の animated-arch-diagram skill の SKILL.md
// 色の名前がほかの図とぶつからないよう、ブロックの中に置く
{
  const IDLE = "#8a8a8a"; // 読んでいない
  const READ = "#3f9d63"; // 読んでいる
  const PAUSED = "#d9534f"; // 止めている

  window.SPECS = window.SPECS || {};
  window.SPECS.status = {
    title: "右上の表示の移り変わり",
    grid: [3, 2],
    nodes: [
      { id: "none", label: "ゲーム未検出", sub: "ゲームを起動する", color: IDLE, col: 0, row: 0 },
      { id: "off", label: "対象外の画面", sub: "一覧を開く", color: IDLE, col: 1, row: 0 },
      { id: "read", label: "○○を集計中", sub: "読んでいる", color: READ, col: 2, row: 0 },
      { id: "pause", label: "一時中断", sub: "表示を押す", color: PAUSED, col: 2, row: 1 },
    ],
    edges: [
      { from: "none", to: "off" },
      { from: "off", to: "read" },
      { from: "read", to: "pause" },
    ],
    steps: [
      {
        kicker: "起動",
        caption: "ゲームを起動すると、数秒で見つける",
        flow: [["none", "off"]],
        token: "ゲーム",
        note: { node: "none", side: "below", lines: ["スマホは、画面を送ると", "見つかる"] },
      },
      {
        kicker: "読む",
        caption: "一覧・生徒・ショップを開くと「集計中」になる",
        flow: [["off", "read"]],
        token: "画面",
        note: {
          node: "off",
          side: "below",
          lines: ["一覧の並び順が", "デフォルト ≡↑ 以外なら", "対象外の画面のまま"],
        },
      },
      {
        kicker: "戻る",
        caption: "読まない画面へ移ると「対象外の画面」に戻る",
        flow: [["read", "off"]],
        token: "画面",
      },
      {
        kicker: "中断",
        caption: "表示を押すと一時中断。もう一度押すと再開",
        flow: [
          ["read", "pause"],
          ["pause", "read"],
        ],
        note: {
          node: "pause",
          side: "left",
          lines: ["左のアイコンが赤になる", "パソコンの負担も小さくなる"],
        },
      },
    ],
    outro: { kicker: "困ったら", caption: "まず、右上の表示を見る" },
  };
}
