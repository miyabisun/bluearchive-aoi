// 「ゲームの画面を読ませる」の流れ。書式は dotfiles の animated-arch-diagram skill の SKILL.md
const GAME = "#d9668c"; // ゲームでの操作
const AOI = "#3b8fd9"; // Aoi

window.SPEC = {
  title: "アイテムを読ませる流れ",
  grid: [5, 1],
  nodes: [
    { id: "menu", label: "≡ メニュー", sub: "ホームの右上", color: GAME, col: 0, row: 0 },
    { id: "list", label: "アイテム", sub: "一覧を開く", color: GAME, col: 1, row: 0 },
    { id: "sort", label: "並び順", sub: "デフォルト ≡↑", color: GAME, col: 2, row: 0 },
    { id: "scroll", label: "スクロール", sub: "少しずつ", color: GAME, col: 3, row: 0 },
    { id: "tab", label: "Aoi", sub: "アイテムタブ", color: AOI, col: 4, row: 0 },
  ],
  edges: [
    { from: "menu", to: "list" },
    { from: "list", to: "sort" },
    { from: "sort", to: "scroll" },
    { from: "scroll", to: "tab" },
  ],
  steps: [
    {
      kicker: "開く",
      caption: "ホームの ≡ から「アイテム」を開く",
      flow: [["menu", "list"]],
      token: "アイテム",
      note: { node: "list", side: "above", lines: ["装備は「装備」を開く"] },
    },
    {
      kicker: "並び順",
      caption: "並び順を「デフォルト」の上向きにする",
      flow: [["list", "sort"]],
      token: "≡↑",
      note: {
        node: "sort",
        side: "below",
        lines: ["デフォルト・上向き … 数える", "それ以外 … 対象外の画面"],
      },
    },
    {
      kicker: "スクロール",
      caption: "1 回スクロールしたら、一瞬止める",
      flow: [["sort", "scroll"]],
      token: "画面",
      note: { node: "scroll", side: "above", lines: ["速く流すと読み飛ばす"] },
    },
    {
      kicker: "決める",
      caption: "同じ数を 3 回続けて読めたら、数が決まる",
      flow: [["scroll", "tab"]],
      token: "個数",
      note: {
        node: "tab",
        side: "below",
        lines: ["1 万以上は 51K のように出る", "イベントのアイテムは数えない"],
      },
    },
  ],
  outro: { kicker: "読む", caption: "装備・生徒・ショップも、開いて送るだけ" },
};
