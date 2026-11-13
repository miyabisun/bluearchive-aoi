// 「ゲームの画面を読ませる」の生徒の流れ。書式は dotfiles の animated-arch-diagram skill の SKILL.md
// 色の名前がほかの図とぶつからないよう、ブロックの中に置く
{
  const GAME = "#d9668c"; // ゲームでの操作
  const AOI = "#3b8fd9"; // Aoi

  window.SPECS = window.SPECS || {};
  window.SPECS.students = {
    title: "生徒を読ませる流れ",
    grid: [5, 1],
    nodes: [
      { id: "home", label: "生徒", sub: "ホームの下", color: GAME, col: 0, row: 0 },
      { id: "pick", label: "1 人選ぶ", sub: "誰でもよい", color: GAME, col: 1, row: 0 },
      { id: "info", label: "基本情報", sub: "右側のタブ", color: GAME, col: 2, row: 0 },
      { id: "next", label: "＜ ＞", sub: "1 人ずつ送る", color: GAME, col: 3, row: 0 },
      { id: "tab", label: "Aoi", sub: "生徒タブ", color: AOI, col: 4, row: 0 },
    ],
    edges: [
      { from: "home", to: "pick" },
      { from: "pick", to: "info" },
      { from: "info", to: "next" },
      { from: "next", to: "tab" },
    ],
    steps: [
      {
        kicker: "開く",
        caption: "ホームの「生徒」から、誰か 1 人を選ぶ",
        flow: [["home", "pick"]],
        token: "生徒",
      },
      {
        kicker: "基本情報",
        caption: "右側の「基本情報」タブを開く",
        flow: [["pick", "info"]],
        token: "生徒",
        note: {
          node: "info",
          side: "below",
          lines: ["レベルアップ・神秘解放でも", "名前と Lv は読む"],
        },
      },
      {
        kicker: "送る",
        caption: "＜ ＞ で、1 人ずつ 1 秒ほど止めて送る",
        flow: [["info", "next"]],
        token: "画面",
        note: { node: "next", side: "above", lines: ["速く送ると読み飛ばす"] },
      },
      {
        kicker: "並ぶ",
        caption: "読めた生徒が、生徒タブに並ぶ",
        flow: [["next", "tab"]],
        token: "生徒",
        note: {
          node: "tab",
          side: "below",
          lines: ["Lv 90  ★5", "スキル 5M7M", "装備 T10/9/8", "読めない項目は ?"],
        },
      },
    ],
    outro: { kicker: "確かめる", caption: "タブの「生徒 42」が、ゲームの所持人数と同じなら全員" },
  };
}
