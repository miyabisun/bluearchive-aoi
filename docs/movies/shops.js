// 「ゲームの画面を読ませる」のショップの流れ。書式は dotfiles の animated-arch-diagram skill の SKILL.md
// 色の名前がほかの図とぶつからないよう、ブロックの中に置く
{
  const GAME = "#d9668c"; // ゲームでの操作
  const AOI = "#3b8fd9"; // Aoi

  window.SPECS = window.SPECS || {};
  window.SPECS.shops = {
    title: "ショップを読ませる流れ",
    grid: [4, 1],
    nodes: [
      { id: "home", label: "ショップ", sub: "ホームの下", color: GAME, col: 0, row: 0 },
      { id: "shop", label: "店を選ぶ", sub: "6 つの店", color: GAME, col: 1, row: 0 },
      { id: "scroll", label: "スクロール", sub: "一番下まで", color: GAME, col: 2, row: 0 },
      { id: "tab", label: "Aoi", sub: "ショップタブ", color: AOI, col: 3, row: 0 },
    ],
    edges: [
      { from: "home", to: "shop" },
      { from: "shop", to: "scroll" },
      { from: "scroll", to: "tab" },
    ],
    steps: [
      {
        kicker: "開く",
        caption: "ホームの「ショップ」で、読む店を選ぶ",
        flow: [["home", "shop"]],
        token: "店",
        note: {
          node: "shop",
          side: "below",
          lines: ["総力戦・大決戦", "カケラ(文字)・合同火力演習", "指名手配・熟達証書"],
        },
      },
      {
        kicker: "送る",
        caption: "品物を、一番下までゆっくりスクロールする",
        flow: [["shop", "scroll"]],
        token: "画面",
        note: { node: "scroll", side: "above", lines: ["買い切った品は一番下"] },
      },
      {
        kicker: "並ぶ",
        caption: "残り回数と値段が、ショップタブに並ぶ",
        flow: [["scroll", "tab"]],
        token: "残り回数",
        note: { node: "tab", side: "below", lines: ["40回@10", "売切れ"] },
      },
      {
        kicker: "月替わり",
        caption: "カケラ(文字) 以外は、毎月 1 日の午前 4 時に戻る",
        focus: ["tab"],
        note: {
          node: "tab",
          side: "above",
          lines: ["「前の月の値」と出たら", "もう一度読ませる"],
        },
      },
    ],
    outro: { kicker: "ほかの店", caption: "通常・青輝石などの店は読まない" },
  };
}
