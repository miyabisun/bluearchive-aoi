// トップ 2 本目「Aoi が画面を見て覚える」。書式は dotfiles の animated-arch-diagram skill の SKILL.md
// 色の名前がほかの図とぶつからないよう、ブロックの中に置く
{
  const GAME = "#d9668c"; // ゲームの画面
  const AOI = "#3b8fd9"; // Aoi
  const CALC = "#3f9d63"; // 提案

  window.SPECS = window.SPECS || {};
  window.SPECS.collect = {
    title: "Aoi が画面を見て覚える",
    grid: [3, 4],
    nodes: [
      { id: "items", label: "アイテム", sub: "一覧", color: GAME, col: 0, row: 0, w: 280 },
      { id: "equip", label: "装備", sub: "一覧", color: GAME, col: 0, row: 1, w: 280 },
      { id: "student", label: "生徒詳細", sub: "基本情報", color: GAME, col: 0, row: 2, w: 280 },
      { id: "shop", label: "ショップ", sub: "6 つの店", color: GAME, col: 0, row: 3, w: 280 },
      { id: "aoi", label: "Aoi", sub: "見て覚える", color: AOI, col: 1, row: 1.5, w: 300 },
      { id: "suggest", label: "提案", sub: "育成・お仕事", color: CALC, col: 2, row: 1.5, w: 300 },
    ],
    edges: [
      { from: "items", to: "aoi" },
      { from: "equip", to: "aoi" },
      { from: "student", to: "aoi" },
      { from: "shop", to: "aoi" },
      { from: "aoi", to: "suggest" },
    ],
    steps: [
      {
        kicker: "アイテム",
        caption: "アイテム一覧をスクロールすると、所持数を覚える",
        flow: [["items", "aoi"]],
        token: "所持数",
        note: { node: "aoi", side: "above", lines: ["同じ数を 3 回読めたら決める"] },
      },
      {
        kicker: "装備",
        caption: "装備一覧も、設計図までまとめて覚える",
        flow: [["equip", "aoi"]],
        token: "装備",
      },
      {
        kicker: "生徒",
        caption: "生徒の画面を送ると、Lv・星・スキルを覚える",
        flow: [["student", "aoi"]],
        token: "生徒",
        note: { node: "aoi", side: "below", lines: ["Lv 90  ★5", "スキル 5M7M", "装備 T10/9/8"] },
      },
      {
        kicker: "ショップ",
        caption: "ショップを送ると、残り回数を覚える",
        flow: [["shop", "aoi"]],
        token: "ショップ",
        note: { node: "aoi", side: "below", lines: ["残り回数@値段", "例: 40回@10"] },
      },
      {
        kicker: "記憶",
        caption: "覚えた内容はパソコンに保存し、次に開いても残る",
        focus: ["aoi"],
        note: { node: "aoi", side: "above", lines: ["ドキュメントの Aoi フォルダ", "aoi.yaml"] },
      },
      {
        kicker: "提案",
        caption: "覚えた数から、育成とお仕事の提案を出す",
        flow: [["aoi", "suggest"]],
        token: "提案",
      },
    ],
    outro: { kicker: "Aoi", caption: "ゲームは操作しない。画面を見て覚えるだけ" },
  };
}
