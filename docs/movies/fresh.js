// 「育成タブ」の計算に使う数。書式は dotfiles の animated-arch-diagram skill の SKILL.md
// 色の名前がほかの図とぶつからないよう、ブロックの中に置く
{
  const GAME = "#d9668c"; // ゲームで読んだもの
  const AOI = "#3b8fd9"; // Aoi が覚えている数
  const CALC = "#3f9d63"; // 計算した結果

  window.SPECS = window.SPECS || {};
  window.SPECS.fresh = {
    title: "計算に使う数",
    grid: [4, 3],
    nodes: [
      { id: "held", label: "持ち物", sub: "アイテム・装備・店", color: GAME, col: 0, row: 0 },
      { id: "now", label: "いつも今の数", sub: "読むたび変わる", color: AOI, col: 1, row: 0 },
      {
        id: "students",
        label: "育成状況",
        sub: "Lv・星・スキル・装備",
        color: GAME,
        col: 0,
        row: 2,
      },
      { id: "ground", label: "起動時の数", sub: "再読込まで同じ", color: AOI, col: 1, row: 2 },
      { id: "plan", label: "育成タブ", sub: "必要・所持・不足", color: CALC, col: 2.6, row: 1 },
    ],
    edges: [
      { from: "held", to: "now" },
      { from: "students", to: "ground" },
      { from: "now", to: "plan" },
      { from: "ground", to: "plan" },
    ],
    steps: [
      {
        kicker: "持ち物",
        caption: "持ち物は、読むたびに新しい数で数える",
        flow: [
          ["held", "now"],
          ["now", "plan"],
        ],
        token: "所持数",
        note: { node: "now", side: "below", lines: ["増えても減っても", "所持と不足がすぐ変わる"] },
      },
      {
        kicker: "育成状況",
        caption: "生徒の育成状況は、起動したときの数で計算する",
        flow: [
          ["students", "ground"],
          ["ground", "plan"],
        ],
        token: "育成状況",
        note: { node: "ground", side: "above", lines: ["強化しても", "必要な数はそのまま"] },
      },
      {
        kicker: "お知らせ",
        caption: "育成対象を強化して読ませると、オレンジの文字で知らせる",
        focus: ["ground", "plan"],
        note: { node: "ground", side: "above", lines: ["○○の育成状況が", "変わっています"] },
      },
      {
        kicker: "再読込",
        caption: "「再読込」を押すと、今の育成状況で数え直す",
        flow: [
          ["students", "ground"],
          ["ground", "plan"],
        ],
        token: "再読込",
        note: { node: "ground", side: "above", lines: ["払った分は取り消される"] },
      },
    ],
    outro: { kicker: "育成タブ", caption: "強化の途中でも、計画は消えない" },
  };
}
