// 「お仕事タブ」がゲームの画面についてくる流れ。書式は dotfiles の animated-arch-diagram skill の SKILL.md
// 色の名前がほかの図とぶつからないよう、ブロックの中に置く
{
  const GAME = "#d9668c"; // ゲームでの操作
  const AOI = "#3b8fd9"; // Aoi
  const CALC = "#3f9d63"; // お仕事タブの項目

  window.SPECS = window.SPECS || {};
  window.SPECS.follow = {
    title: "ゲームの画面についてくる",
    grid: [3, 2],
    nodes: [
      { id: "game", label: "ゲーム", sub: "指名手配を開く", color: GAME, col: 0, row: 0 },
      { id: "badge", label: "右上の表示", sub: "指名手配", color: AOI, col: 1, row: 0 },
      { id: "fold", label: "お仕事タブ", sub: "指名手配が開く", color: CALC, col: 2, row: 0 },
      { id: "set", label: "設定", sub: "お仕事の自動開閉", color: AOI, col: 2, row: 1 },
    ],
    edges: [
      { from: "game", to: "badge" },
      { from: "badge", to: "fold" },
      { from: "set", to: "fold", dashed: true },
    ],
    steps: [
      {
        kicker: "開く",
        caption: "お仕事タブを開いたまま、ゲームで画面を開く",
        flow: [["game", "badge"]],
        token: "画面",
        note: {
          node: "game",
          side: "below",
          lines: ["学園交流会・指名手配", "特別依頼・ショップ", "任務（キャンペーン中）"],
        },
      },
      {
        kicker: "表示",
        caption: "右上の表示が、その画面の名前になる",
        focus: ["badge"],
        note: {
          node: "badge",
          side: "below",
          lines: ["数は読まないので", "「集計中」にはならない"],
        },
      },
      {
        kicker: "ついてくる",
        caption: "その項目が開いて、一番上に来る",
        flow: [["badge", "fold"]],
        token: "指名手配",
        note: {
          node: "badge",
          side: "below",
          lines: ["ほかの項目は閉じる", "自分で閉じた項目は", "その画面の間は閉じたまま"],
        },
      },
      {
        kicker: "止める",
        caption: "設定で「しない」を選ぶと、ついてこなくなる",
        flow: [["set", "fold"]],
        token: "しない",
        note: { node: "set", side: "left", lines: ["お仕事の自動開閉", "する / しない"] },
      },
    ],
    outro: { kicker: "お仕事", caption: "ゲームを進めるだけで、見たい項目が開く" },
  };
}
