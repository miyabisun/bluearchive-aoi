// 「お仕事タブ」の流れ。書式は dotfiles の animated-arch-diagram skill の SKILL.md
const GAME = "#d9668c"; // ゲームでの操作
const TAB = "#b88600"; // Aoi のタブ
const CALC = "#3f9d63"; // 計算した結果

window.SPEC = {
  title: "お仕事タブの流れ",
  grid: [4, 3],
  nodes: [
    { id: "have", label: "手持ち", sub: "アイテム・ショップ", color: TAB, col: 0, row: 1 },
    { id: "work", label: "お仕事タブ", sub: "目安を出す", color: CALC, col: 1, row: 1 },
    { id: "daily", label: "日課", sub: "指名手配など", color: CALC, col: 2, row: 0 },
    {
      id: "double",
      label: "2倍キャンペーン",
      sub: "任務 Normal・Hard",
      color: CALC,
      col: 2,
      row: 1,
    },
    { id: "month", label: "月末", sub: "ショップの予約", color: CALC, col: 2, row: 2 },
    { id: "game", label: "ゲーム", sub: "回る・買う", color: GAME, col: 3, row: 1 },
  ],
  edges: [
    { from: "have", to: "work" },
    { from: "work", to: "daily" },
    { from: "work", to: "double" },
    { from: "work", to: "month" },
    { from: "daily", to: "game" },
    { from: "double", to: "game" },
    { from: "month", to: "game" },
  ],
  steps: [
    {
      kicker: "手持ち",
      caption: "読んだ持ち物とショップから計算する",
      flow: [["have", "work"]],
      token: "手持ち",
    },
    {
      kicker: "日課",
      caption: "指名手配は、足りない品が落ちる場所から回る",
      flow: [["work", "daily"]],
      token: "指名手配",
      note: {
        node: "daily",
        side: "right",
        lines: ["赤 … 1 人分に足りない", "青 … 2 人分に足りない", "緑 … 2 人分以上"],
      },
    },
    {
      kicker: "2倍",
      caption: "2 倍キャンペーン中に、黄金比まで設計図を集める",
      flow: [["work", "double"]],
      token: "設計図",
      note: {
        node: "work",
        side: "above",
        lines: ["どのステージを何回回るか", "AP が一番少なくなるように"],
      },
    },
    {
      kicker: "月末",
      caption: "今月のうちに買っておく品を予約する",
      flow: [["work", "month"]],
      token: "予約",
      note: { node: "month", side: "right", lines: ["押すと「予約中」", "コインの残りは青で出る"] },
    },
    {
      kicker: "ゲーム",
      caption: "その目安で、ゲームで回って買う",
      flow: [
        ["daily", "game"],
        ["double", "game"],
        ["month", "game"],
      ],
      hop: 0.6,
    },
  ],
  outro: { kicker: "お仕事", caption: "足りないものから先に集める" },
};
