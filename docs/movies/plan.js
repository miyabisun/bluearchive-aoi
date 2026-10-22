// 「育成タブ」の流れ。書式は dotfiles の animated-arch-diagram skill の SKILL.md
// 色の名前がほかの図とぶつからないよう、ブロックの中に置く
{
  const TAB = "#b88600"; // Aoi のタブ
  const CALC = "#3f9d63"; // 計算した結果

  window.SPECS = window.SPECS || {};
  window.SPECS.plan = {
    title: "育成タブの流れ",
    grid: [5, 1],
    nodes: [
      { id: "mark", label: "生徒タブ", sub: "「育成」に印", color: TAB, col: 0, row: 0 },
      { id: "goal", label: "目標", sub: "Lv・星・スキル", color: CALC, col: 1, row: 0 },
      { id: "need", label: "必要素材", sub: "過不足", color: CALC, col: 2, row: 0 },
      { id: "fill", label: "不足分の調達", sub: "箱・カケラ・店", color: CALC, col: 3, row: 0 },
      { id: "settle", label: "精算", sub: "払った後の残り", color: CALC, col: 4, row: 0 },
    ],
    edges: [
      { from: "mark", to: "goal" },
      { from: "goal", to: "need" },
      { from: "need", to: "fill" },
      { from: "fill", to: "settle" },
    ],
    steps: [
      {
        kicker: "印",
        caption: "育てたい生徒の「育成」に印を付ける",
        flow: [["mark", "goal"]],
        token: "生徒",
        note: {
          node: "goal",
          side: "above",
          lines: ["最初は Lv90", "スキル 5/10/10/10", "装備 T10/10/10"],
        },
      },
      {
        kicker: "目標",
        caption: "欄を押して、数字か ↑↓ で目標を変える",
        flow: [["goal", "need"]],
        token: "目標",
        note: { node: "need", side: "below", lines: ["足りる … 緑", "不足 N … 赤"] },
      },
      {
        kicker: "補う",
        caption: "足りないものを、チップを押して補う",
        flow: [["need", "fill"]],
        token: "不足",
        note: {
          node: "fill",
          side: "above",
          lines: [
            "クリック … 1 つ払う",
            "右クリック … 1 つ戻す",
            "Shift+クリック … 不足がなくなるまで",
          ],
        },
      },
      {
        kicker: "精算",
        caption: "払った後の残りが、下に出る",
        flow: [["fill", "settle"]],
        token: "払った分",
        note: {
          node: "settle",
          side: "below",
          lines: ["所持・消費・残り", "ゲームの持ち物は減らない"],
        },
      },
    ],
    outro: { kicker: "ゲームで", caption: "決めたとおりに、箱を開けてショップで買う" },
  };
}
