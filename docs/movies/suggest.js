// トップ 3 本目「Aoi の提案」。書式は dotfiles の animated-arch-diagram skill の SKILL.md
// 色の名前がほかの図とぶつからないよう、ブロックの中に置く
{
  const AOI = "#3b8fd9"; // Aoi
  const TAB = "#b88600"; // 覚えた数の一覧のタブ
  const CALC = "#3f9d63"; // 提案のタブ

  window.SPECS = window.SPECS || {};
  window.SPECS.suggest = {
    title: "Aoi の提案",
    grid: [3, 3],
    nodes: [
      { id: "aoi", label: "Aoi", sub: "覚えた数", color: AOI, col: 0, row: 1 },
      {
        id: "students",
        label: "生徒タブ",
        sub: "一覧: Lv・星・スキル",
        color: TAB,
        col: 1,
        row: 0,
      },
      { id: "items", label: "アイテム・装備", sub: "一覧: 所持数", color: TAB, col: 1, row: 1 },
      { id: "shop", label: "ショップ", sub: "一覧: 残り回数", color: TAB, col: 1, row: 2 },
      { id: "plan", label: "育成タブ", sub: "提案: 足りない素材", color: CALC, col: 2, row: 0.5 },
      { id: "work", label: "お仕事タブ", sub: "提案: 集める目安", color: CALC, col: 2, row: 1.5 },
    ],
    edges: [
      { from: "aoi", to: "students" },
      { from: "aoi", to: "items" },
      { from: "aoi", to: "shop" },
      { from: "students", to: "plan" },
      { from: "items", to: "plan" },
      { from: "items", to: "work" },
      { from: "shop", to: "work" },
    ],
    steps: [
      {
        kicker: "一覧",
        caption: "覚えた数を、タブに一覧で並べる",
        flow: [
          ["aoi", "students"],
          ["aoi", "items"],
          ["aoi", "shop"],
        ],
        token: "一覧",
        hop: 0.6,
      },
      {
        kicker: "生徒タブ",
        caption: "生徒ごとの Lv・星・スキル・装備が並ぶ",
        focus: ["students"],
        note: { node: "students", side: "left", lines: ["Lv 90  ★5", "スキル 5M7M"] },
      },
      {
        kicker: "アイテム・装備",
        caption: "アイコンと所持数が並ぶ",
        focus: ["items"],
        note: { node: "items", side: "right", lines: ["1 万以上は 51K"] },
      },
      {
        kicker: "ショップ",
        caption: "店ごとに、残り回数と値段が並ぶ",
        focus: ["shop"],
        note: { node: "shop", side: "left", lines: ["40回@10", "売切れ"] },
      },
      {
        kicker: "育成タブ",
        caption: "育てたい生徒に足りない素材と、その補い方を出す",
        flow: [
          ["students", "plan"],
          ["items", "plan"],
        ],
        token: "不足",
        note: {
          node: "plan",
          side: "above",
          lines: ["必要・所持・過不足", "箱・カケラ・ショップで補う"],
        },
      },
      {
        kicker: "お仕事タブ",
        caption: "集める場所と、月末に買っておく品の目安を出す",
        flow: [
          ["items", "work"],
          ["shop", "work"],
        ],
        token: "目安",
        note: {
          node: "work",
          side: "below",
          lines: ["指名手配: 赤・青・緑", "月末: 予約して買う"],
        },
      },
    ],
    outro: { kicker: "提案", caption: "足りないものから、先に集める" },
  };
}
