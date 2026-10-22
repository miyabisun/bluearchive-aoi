// トップの「全体の流れ」。書式は dotfiles の animated-arch-diagram skill の SKILL.md
const GAME = "#d9668c"; // ゲーム・スマホの側
const AOI = "#3b8fd9"; // Aoi
const TAB = "#b88600"; // Aoi のタブ
const CALC = "#3f9d63"; // 計算した結果

window.SPEC = {
  title: "Aoi の全体の流れ",
  grid: [4, 3],
  nodes: [
    { id: "game", label: "ゲームの画面", sub: "PC・スマホ", color: GAME, col: 0, row: 1 },
    { id: "aoi", label: "Aoi", sub: "横で見て数える", color: AOI, col: 1, row: 1 },
    { id: "items", label: "アイテム・装備", sub: "所持数", color: TAB, col: 2, row: 1 },
    { id: "students", label: "生徒タブ", sub: "Lv・星・スキル", color: TAB, col: 2, row: 0 },
    { id: "shop", label: "ショップタブ", sub: "残り回数", color: TAB, col: 2, row: 2 },
    { id: "plan", label: "育成タブ", sub: "足りない素材", color: CALC, col: 3, row: 0.5 },
    { id: "work", label: "お仕事タブ", sub: "集める目安", color: CALC, col: 3, row: 1.5 },
  ],
  edges: [
    { from: "game", to: "aoi" },
    { from: "aoi", to: "items" },
    { from: "aoi", to: "students" },
    { from: "aoi", to: "shop" },
    { from: "students", to: "plan" },
    { from: "items", to: "plan" },
    { from: "items", to: "work" },
    { from: "shop", to: "work" },
  ],
  steps: [
    {
      kicker: "開く",
      caption: "ゲームで一覧を開いて、ゆっくりスクロールする",
      flow: [["game", "aoi"]],
      token: "画面",
    },
    {
      kicker: "数える",
      caption: "同じ数を 3 回読めたら、その数に決める",
      flow: [["aoi", "items"]],
      token: "個数",
      note: {
        node: "aoi",
        side: "above",
        lines: ["同じ数が 3 回続いたら決める", "1 万以上は 51K のように出る"],
      },
    },
    {
      kicker: "生徒",
      caption: "生徒の画面を「＞」で 1 人ずつ送る",
      flow: [
        ["game", "aoi"],
        ["aoi", "students"],
      ],
      token: "生徒",
      note: { node: "students", side: "left", lines: ["Lv 90  ★5", "スキル 5M7M", "装備 T10/9/8"] },
    },
    {
      kicker: "ショップ",
      caption: "ショップを下まで送ると、残り回数を読む",
      flow: [
        ["game", "aoi"],
        ["aoi", "shop"],
      ],
      token: "ショップ",
      note: { node: "shop", side: "left", lines: ["残り回数@値段", "例: 40回@10"] },
    },
    {
      kicker: "育成",
      caption: "育てたい生徒に印を付けると、足りない素材が出る",
      flow: [
        ["students", "plan"],
        ["items", "plan"],
      ],
      token: "不足",
      note: { node: "plan", side: "above", lines: ["必要・所持・過不足", "足りないものは赤"] },
    },
    {
      kicker: "お仕事",
      caption: "回る場所と、月末に買っておく品の目安が出る",
      flow: [
        ["items", "work"],
        ["shop", "work"],
      ],
      token: "目安",
      note: { node: "work", side: "below", lines: ["指名手配: 赤・青・緑", "月末: 予約して買う"] },
    },
  ],
  outro: { kicker: "Aoi", caption: "ゲームは操作しない。画面を見て数えるだけ" },
};
