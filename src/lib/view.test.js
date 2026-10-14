import { describe, expect, it } from "vitest";
import {
  blueprintCells,
  filterRows,
  filterShops,
  groupPlan,
  needCells,
  fragmentCost,
  procureCells,
  settle,
  step,
  starName,
  starOf,
  tierCell,
  unknownText,
  commissionCells,
  farmCells,
  goodTile,
  heldTone,
  placeName,
  stockCells,
  updateText,
  phoneLine,
  qrPath,
  shopStale,
  formatCount,
  sortStudents,
  statusView,
  studentCells,
} from "./view.js";

const rows = [
  { id: 10, name: "初級戦術教育BD（ゲヘナ）", count: 12 },
  { id: 20, name: "ＡＰ回復ドリンク", count: 3 },
  { id: 30, name: "Ooparts Nebra Disk", count: 1480 },
];

describe("filterRows", () => {
  it("keeps every row for an empty query", () => {
    expect(filterRows(rows, "  ")).toEqual(rows);
  });

  it("matches part of a name, ignoring case and full-width forms", () => {
    expect(filterRows(rows, "ap回復").map((r) => r.id)).toEqual([20]);
    expect(filterRows(rows, "nebra").map((r) => r.id)).toEqual([30]);
  });

  it("matches katakana typed in hiragana", () => {
    const students = [
      { id: 1, name: "アカネ（バニーガール）" },
      { id: 2, name: "ホシノ" },
    ];
    expect(filterRows(students, "あかね").map((r) => r.id)).toEqual([1]);
    expect(filterRows(students, "ばにー").map((r) => r.id)).toEqual([1]);
    expect(filterRows(rows, "ぎじゅつ")).toEqual([]);
  });

  it("needs every word of the query", () => {
    expect(filterRows(rows, "BD ゲヘナ").map((r) => r.id)).toEqual([10]);
    expect(filterRows(rows, "BD ミレニアム")).toEqual([]);
  });
});

describe("statusView", () => {
  it("names each state and marks which are trouble", () => {
    expect(statusView({ state: "no_game" })).toEqual({ label: "ゲーム未検出", tone: "idle" });
    expect(statusView({ state: "no_source" })).toEqual({ label: "取り込み元なし", tone: "idle" });
    expect(statusView({ state: "other_screen" })).toEqual({ label: "対象外の画面", tone: "idle" });
    expect(statusView({ state: "reading", list: "items", fps: 27.5, dropped: 3 })).toEqual({
      label: "アイテムを集計中 28 fps",
      tone: "active",
    });
    expect(statusView({ state: "reading", list: "equipment", fps: 9.6, dropped: 0 })).toEqual({
      label: "装備を集計中 10 fps",
      tone: "active",
    });
    expect(statusView({ state: "failed", message: "denied" })).toEqual({
      label: "キャプチャ失敗",
      tone: "danger",
    });
    expect(statusView({ state: "paused" })).toEqual({ label: "一時中断", tone: "paused" });
  });
});

describe("formatCount", () => {
  it("shows exact counts with separators and rounded ones as the game does", () => {
    expect(formatCount({ exact: 1383 })).toBe("1,383");
    expect(formatCount({ exact: 7 })).toBe("7");
    expect(formatCount({ thousands: 51 })).toBe("51K");
  });
});

describe("studentCells", () => {
  const known = {
    level: 90,
    rarity: 5,
    bond: 34,
    skills: [5, 10, 7, 10],
    weapon: { level: 60, stars: 4 },
    equipment: [
      { tier: 10, level: 70 },
      { tier: 9, level: 65 },
      { tier: 8, level: 60 },
    ],
    gear: 2,
    ability: [25, 0, 25],
  };

  it("writes a student's training the way timeline-arona does", () => {
    expect(studentCells(known)).toEqual({
      level: "90",
      star: "固有4",
      weapon: "固有武器 Lv60",
      skills: "5M7M",
      equipment: "T10/9/8",
      equipmentLevels: "Lv70/65/60",
      gear: "T2",
      ability: "25/0/25",
      bond: "34",
    });
  });

  it("marks what is locked, missing or unknown", () => {
    const cells = studentCells({
      ...known,
      level: null,
      weapon: null,
      skills: [3, null, 1, 1],
      equipment: [{ tier: 1, level: 1 }, null, { tier: 1, level: 10 }],
      gear: 0,
      ability: [0, null, 0],
    });
    expect(cells.level).toBe("?");
    // Under ★5 the weapon is locked; at ★5 its stars must be read.
    expect(studentCells({ ...known, rarity: 3, weapon: null })).toMatchObject({
      star: "★3",
      weapon: "",
    });
    expect(studentCells({ ...known, rarity: null }).star).toBe("?");
    expect(cells.skills).toBe("3?11");
    expect(cells.equipment).toBe("T1/?/1");
    expect(cells.equipmentLevels).toBe("Lv1/?/10");
    expect(cells.gear).toBe("—");
    expect(cells.ability).toBe("0/?/0");
    expect(studentCells({ ...known, weapon: undefined })).toMatchObject({ star: "?", weapon: "" });
  });
});

describe("sortStudents", () => {
  /** A student whose training is a 5MMM T10 one with `overrides`. */
  const student = (id, overrides) => ({
    id,
    name: `生徒${id}`,
    training: {
      level: 90,
      rarity: 5,
      bond: 20,
      skills: [5, 10, 10, 10],
      weapon: { stars: 4, level: 60 },
      equipment: [
        { tier: 10, level: 70 },
        { tier: 10, level: 70 },
        { tier: 10, level: 70 },
      ],
      gear: 0,
      ability: [0, 0, 0],
      ...overrides,
    },
  });
  const ids = (rows) => rows.map((r) => r.id);

  it("puts the highest level first, then the star, then the bond, by default", () => {
    const rows = [
      student(1, { level: 80, bond: 50 }),
      student(2, { rarity: 4, bond: 50 }),
      student(3, { bond: 20 }),
      student(4, { bond: 30, weapon: { stars: 1, level: 30 }, skills: [1, 1, 1, 1] }),
    ];
    // The star counts on into the unique weapon's: ★5 with it at ★4 is above ★5 with it at ★1.
    expect(ids(sortStudents(rows))).toEqual([3, 4, 2, 1]);
  });

  it("puts what could not be read last", () => {
    const rows = [student(1, { level: null }), student(2, { level: 1 }), student(3)];
    expect(ids(sortStudents(rows))).toEqual([3, 2, 1]);
  });

  it("orders by a chosen column, highest first, the default order breaking ties", () => {
    const rows = [
      student(1, { weapon: { stars: 1, level: 30 } }),
      student(2, { weapon: null }),
      student(3, { weapon: undefined }),
      student(4, { weapon: { stars: 4, level: 50 } }),
      student(5, { level: 87, weapon: { stars: 4, level: 60 } }),
      student(6),
    ];
    expect(ids(sortStudents(rows, "star"))).toEqual([6, 5, 4, 1, 2, 3]);
  });

  it("adds up the skills, the equipment tiers and the ability releases", () => {
    const rows = [
      student(1, { skills: [5, 10, 10, 9] }),
      student(2, { skills: [3, 10, 10, 10] }),
      student(3, { skills: [5, 10, null, 10] }),
    ];
    expect(ids(sortStudents(rows, "skills"))).toEqual([1, 2, 3]);
    const geared = [
      student(1, {
        equipment: [
          { tier: 9, level: 65 },
          { tier: 10, level: 70 },
          { tier: 10, level: 70 },
        ],
      }),
      student(2, {
        equipment: [
          { tier: 9, level: 70 },
          { tier: 10, level: 70 },
          { tier: 10, level: 70 },
        ],
      }),
      student(3),
    ];
    expect(ids(sortStudents(geared, "equipment"))).toEqual([3, 2, 1]);
    const released = [student(1, { ability: [25, 0, 0] }), student(2, { ability: [25, 0, 25] })];
    expect(ids(sortStudents(released, "ability"))).toEqual([2, 1]);
  });
});

describe("shops", () => {
  const shop = (name, readAt, resets, goods) => ({
    name,
    read_at: readAt,
    resets,
    coins: ["総力戦レアコイン", "総力戦コイン"],
    goods,
  });
  const good = (id, name, offers) => ({ id, name, offers });
  const terms = (price, amount, limit) => ({ price, amount, limit });

  it("tells a shop that resets read in an earlier month", () => {
    const now = new Date(2026, 10, 1, 9, 0);
    expect(shopStale(shop("総力戦", "2026-10-31 23:00:00", true, []), now)).toBe(true);
    expect(shopStale(shop("総力戦", "2026-11-01 08:00:00", true, []), now)).toBe(false);
    expect(shopStale(shop("カケラ(文字)", "2026-09-01 08:00:00", false, []), now)).toBe(false);
  });

  it("writes what each coin buys: the times left at its price, or sold out", () => {
    expect(
      goodTile(
        good(10019, "アズサの神名文字", [
          { left: 20, price: terms(50, 5, 20) },
          { left: 0, price: terms(50, 5, 10) },
        ]),
      ),
    ).toEqual({
      amount: "×5",
      offers: [
        { text: "20回@50", sold: false },
        { text: "売切れ", sold: true },
      ],
    });
    expect(
      goodTile(
        good(3000, "初級戦術教育BD（百鬼夜行）", [{ left: 1200, price: terms(1800, 1, 6) }, null]),
      ),
    ).toEqual({ amount: "×1", offers: [{ text: "1,200回@1,800", sold: false }, null] });
    // Without terms, the price is unknown and so is how many come.
    expect(goodTile(good(1, "?", [{ left: 3, price: null }]))).toEqual({
      amount: "",
      offers: [{ text: "3回@?", sold: false }],
    });
  });

  it("narrows each shop's goods by name and leaves out a shop with none", () => {
    const shops = [
      shop("総力戦", "", true, [good(1, "秘伝ノート", []), good(2, "エーテルの粉", [])]),
      shop("指名手配", "", true, [good(3, "エーテルの欠片", [])]),
    ];
    expect(filterShops(shops, "秘伝")).toEqual([{ ...shops[0], goods: [shops[0].goods[0]] }]);
    expect(filterShops(shops, "")).toEqual(shops);
  });
});

describe("plan", () => {
  it("tells what is short against what is held", () => {
    expect(
      needCells({ kind: "item", name: "初級技術ノート（ゲヘナ）", need: 30, have: { exact: 12 } }),
    ).toEqual({
      name: "初級技術ノート（ゲヘナ）",
      need: "30",
      have: "12",
      short: "不足 18",
      enough: false,
    });
    expect(
      needCells({ kind: "credits", name: "", need: 1500000, have: { exact: 244654006 } }),
    ).toEqual({
      name: "クレジット",
      need: "1.5M",
      have: "244.7M",
      short: "足りる",
      enough: true,
    });
    expect(
      needCells({ kind: "credits", name: "", need: 28431500, have: { exact: 12000000 } }).short,
    ).toBe("不足 16.4M");
    expect(
      needCells({ kind: "report_exp", name: "", need: 60000, have: { about: 51000 } }),
    ).toEqual({
      name: "レポート（経験値）",
      need: "60,000",
      have: "約51,000",
      short: "不足 約9,000",
      enough: false,
    });
    expect(needCells({ kind: "orb_exp", name: "", need: 5, have: null }).short).toBe("?");
  });

  it("puts the four tiers of a kind on one line, the rest apart", () => {
    const row = (id, family) => ({
      kind: "item",
      id,
      name: String(id),
      family,
      need: 1,
      have: null,
    });
    const rows = [
      { kind: "credits", id: 0, name: "", family: null, need: 5, have: null },
      row(4002, [4000, 3]),
      row(3000, [3000, 1]),
      row(4000, [4000, 1]),
      row(9999, null),
    ];
    expect(groupPlan(rows)).toEqual({
      families: [
        { id: 3000, tiers: [rows[2], null, null, null] },
        { id: 4000, tiers: [rows[3], null, rows[1], null] },
      ],
      rest: [rows[0], rows[4]],
    });
  });

  it("writes a tier as needed against held", () => {
    expect(tierCell({ need: 30, have: { exact: 12 } })).toEqual({ text: "30 / 12", enough: false });
    expect(tierCell({ need: 3, have: { about: 51000 } })).toEqual({
      text: "3 / 約51,000",
      enough: true,
    });
  });

  it("writes a slot's blueprints by tier, and how they are paid", () => {
    const tier = (tier, need, have) => ({
      tier,
      id: 100999 + tier,
      name: `T${tier}の設計図`,
      icon: `t${tier}`,
      need,
      have: { exact: have },
    });
    const universal = {
      tier: 0,
      id: 501000,
      name: "帽子の万能設計図",
      icon: "u",
      need: 1200,
      have: { about: 1000 },
    };
    const row = {
      name: "帽子",
      icon: "hat",
      tiers: [tier(2, 15, 10), tier(3, 0, 30), tier(10, 60, 17)],
      universal,
      short: 43,
    };
    expect(blueprintCells(row)).toEqual({
      name: "帽子",
      icon: "hat",
      tiers: [
        // Below T10 the universal ones pay the rest: no colour.
        { key: 101001, label: "T2", name: "T2の設計図", icon: "t2", text: "15 / 10", tone: "" },
        // Not needed: nothing to show.
        { key: 101002, label: "T3", name: "T3の設計図", icon: "t3", text: "", tone: "" },
        {
          key: 101009,
          label: "T10",
          name: "T10の設計図",
          icon: "t10",
          text: "60 / 17",
          tone: "short",
        },
      ],
      // What T2 to T9 take of the universal ones over those held, coloured like T10.
      universal: { name: "帽子の万能設計図", icon: "u", text: "1,200 / 約1,000", tone: "short" },
    });
    const enough = blueprintCells({
      ...row,
      tiers: [tier(10, 60, 60)],
      universal: { ...universal, need: 10 },
    });
    expect([enough.tiers[0].tone, enough.universal.tone]).toEqual(["enough", "enough"]);
  });

  it("names a star: the student's 1 to 5, then the unique weapon's", () => {
    // A teacher calls ★5 by its weapon's first star.
    expect([1, 4, 5, 6, 8].map(starName)).toEqual(["★1", "★4", "固有1", "固有2", "固有4"]);
  });

  it("counts a student's star on into the unique weapon's", () => {
    const at = (rarity, weapon) => starOf({ rarity, weapon });
    expect([at(3, null), at(5, { level: 40, stars: 2 }), at(5, undefined), at(null, null)]).toEqual(
      [3, 6, null, null],
    );
  });

  it("names the parts that could not be counted, student by student", () => {
    expect(
      unknownText([
        { id: -1, name: "ツムギ", part: { skill: 0 } },
        { id: -1, name: "ツムギ", part: { equipment: 2 } },
        { id: 10, name: "アル", part: "level" },
      ]),
    ).toEqual(["ツムギ: EX・装備3", "アル: Lv"]);
  });
});

describe("procure", () => {
  const box = {
    type: "box",
    id: 150003,
    name: "最上級技術ノート選択ボックス",
    icon: "b",
    held: 24,
    choices: 12,
  };
  const plain = {
    type: "shop",
    shop: "総力戦",
    coin: "総力戦コイン",
    icon: "c",
    held: 5940,
    price: 25,
    gives: 1,
    left: 20,
  };
  const oopart = {
    ...plain,
    shop: "合同火力演習",
    coin: "火力演習コイン",
    price: 2,
    gives: 2,
    left: 20,
  };
  const kakera = {
    type: "fragments",
    name: "神名のカケラ",
    icon: "k",
    held: 2941,
    rate: 1,
    left: 586,
  };

  const universal = {
    type: "universal",
    id: 501000,
    name: "帽子の万能設計図",
    icon: "u",
    held: 4825,
    rate: 50,
  };

  it("pays T10 blueprints from the universal ones, 50 each", () => {
    const hat = {
      kind: "equipment",
      id: 101009,
      name: "帽子のT10設計図",
      icon: "t10",
      short: 60,
      sources: [universal],
    };
    expect(step(universal, 0, 60, { shift: true })).toBe(60);
    const settled = settle([hat], { "equipment-101009|universal:501000": 3 });
    expect([settled.lines[0].given, settled.lines[0].left]).toEqual([3, 57]);
    expect(settled.resources).toEqual([
      {
        key: "universal:501000",
        name: "帽子の万能設計図",
        icon: "u",
        held: 4825,
        spent: 150,
        after: 4675,
      },
    ]);
    expect(procureCells(settled).lines[0].chips[0]).toMatchObject({
      text: "4,675",
      given: "+3",
      out: false,
      title: "帽子の万能設計図: T10設計図 1 枚に 50 枚（T2〜T9 に使った残り 4,825 枚）",
    });
  });

  it("prices elephs in fragments: 20 at each rate up to 4, then 5 each", () => {
    expect([0, 10, 30, 100].map((n) => fragmentCost(1, n))).toEqual([0, 10, 40, 300]);
    expect(fragmentCost(4, 30)).toBe(80 + 50);
    expect(fragmentCost(5, 3)).toBe(15);
  });

  it("pays one a click, takes one back a right click, all or none with shift", () => {
    // 29 lacking: one more, or as many as make it up.
    expect(step(box, 0, 29)).toBe(1);
    expect(step(box, 0, 29, { shift: true })).toBe(29);
    // A box may be opened past those held: the rest can be tailor-made.
    expect(step(box, 30, 5, { shift: true })).toBe(35);
    // A shop sells no more than it has left this month.
    expect(step(plain, 0, 29, { shift: true })).toBe(20);
    expect(step(plain, 20, 9)).toBe(20);
    // Two ooparts a purchase.
    expect(step(oopart, 0, 5, { shift: true })).toBe(3);
    // A click pays one even once the line is made up; shift adds none then.
    expect(step(box, 3, 0)).toBe(4);
    expect(step(box, 3, 0, { shift: true })).toBe(3);
    expect(step(box, 3, 0, { right: true })).toBe(2);
    expect(step(box, 0, 5, { right: true })).toBe(0);
    expect(step(box, 7, 5, { right: true, shift: true })).toBe(0);
  });

  it("settles the payments: what each line still lacks and what each resource has left", () => {
    const notes = {
      kind: "item",
      id: 4033,
      name: "最上級技術ノート（ゲヘナ）",
      icon: "n",
      short: 29,
      sources: [box, plain],
    };
    const bd = {
      kind: "item",
      id: 3032,
      name: "上級戦術教育BD（ゲヘナ）",
      icon: "d",
      short: 6,
      sources: [{ ...plain, price: 50 }],
    };
    const mutsuki = {
      kind: "item",
      id: 20059,
      name: "ムツキ",
      icon: "m",
      short: 386,
      sources: [kakera],
    };
    const payments = {
      "item-4033|box:150003": 30,
      "item-4033|shop:総力戦:総力戦コイン": 2,
      "item-3032|shop:総力戦:総力戦コイン": 6,
      "item-20059|fragments": 30,
    };
    const result = settle([notes, bd, mutsuki], payments);
    expect(result.lines.map((l) => [l.key, l.given, l.left])).toEqual([
      ["item-4033", 32, 0],
      ["item-3032", 6, 0],
      ["item-20059", 30, 356],
    ]);
    // A chip shows how much of its resource is left after every line's payments.
    expect(result.lines[0].chips.map((c) => [c.key, c.n, c.gives, c.after])).toEqual([
      ["box:150003", 30, 30, -6],
      ["shop:総力戦:総力戦コイン", 2, 2, 5940 - 50 - 300],
    ]);
    expect(result.resources).toEqual([
      {
        key: "box:150003",
        name: "最上級技術ノート選択ボックス",
        icon: "b",
        held: 24,
        spent: 30,
        after: -6,
      },
      {
        key: "coin:総力戦コイン",
        name: "総力戦コイン",
        icon: "c",
        held: 5940,
        spent: 350,
        after: 5590,
      },
      {
        key: "fragments",
        name: "神名のカケラ",
        icon: "k",
        held: 2941,
        spent: 40,
        after: 2941 - 40,
      },
    ]);
    // A coin not read is "?" after too.
    const unread = settle([{ ...bd, sources: [{ ...plain, held: null }] }], {});
    expect(unread.resources[0].after).toBe(null);
  });

  it("writes the procurement: each line, its chips and the resources after", () => {
    const credits = {
      kind: "credits",
      id: 0,
      name: "",
      icon: "",
      short: 7_000_000,
      sources: [
        {
          type: "shop",
          shop: "熟達証書",
          coin: "熟達証書",
          icon: "j",
          held: 5009,
          price: 1500,
          gives: 5_000_000,
          left: 4,
        },
      ],
    };
    const notes = {
      kind: "item",
      id: 4033,
      name: "最上級技術ノート（ゲヘナ）",
      icon: "n",
      short: 29,
      sources: [box, plain],
    };
    const cells = procureCells(
      settle([credits, notes], {
        "credits-0|shop:熟達証書:熟達証書": 1,
        "item-4033|box:150003": 30,
      }),
    );
    expect(cells.lines.map((l) => ({ ...l, chips: undefined }))).toEqual([
      { key: "credits-0", name: "クレジット", icon: "", short: "7M", left: "2M", done: false },
      {
        key: "item-4033",
        name: "最上級技術ノート（ゲヘナ）",
        icon: "n",
        short: "29",
        left: "0",
        done: true,
      },
    ]);
    expect(cells.lines[0].chips).toEqual([
      {
        key: "shop:熟達証書:熟達証書",
        icon: "j",
        text: "3,509",
        negative: false,
        given: "+5M",
        out: false,
        title: "熟達証書: 熟達証書 1,500 で ×5,000,000、今月あと 4 回",
      },
    ]);
    expect(cells.lines[1].chips[0]).toMatchObject({
      text: "-6",
      negative: true,
      given: "+30",
      title: "最上級技術ノート選択ボックス（12 種から 1 つ）",
    });
    // The settlement lists what is paid from, the resources left untouched aside:
    // held, spent and what is left, above none green and below red.
    expect(cells.resources).toEqual([
      {
        key: "coin:熟達証書",
        name: "熟達証書",
        icon: "j",
        held: "5,009",
        spent: "-1,500",
        after: "3,509",
        tone: "plus",
      },
      {
        key: "box:150003",
        name: "最上級技術ノート選択ボックス",
        icon: "b",
        held: "24",
        spent: "-30",
        after: "-6",
        tone: "minus",
      },
    ]);
    // A shop that sells no more this month says so; a box never runs out.
    const out = procureCells(settle([credits], { "credits-0|shop:熟達証書:熟達証書": 4 })).lines[0]
      .chips[0];
    expect(out).toMatchObject({ text: "売切れ", out: true, given: "+20M" });
    expect(cells.lines[1].chips[0].out).toBe(false);
    const even = settle([notes], { "item-4033|box:150003": 24 });
    expect(procureCells(even).resources[0]).toMatchObject({ after: "0", tone: "" });
  });
});

describe("work", () => {
  it("writes how many pieces a slot's blueprints make, short of one red, of the golden ratio orange", () => {
    const stock = (pieces) => ({ held: 270, about: false, need: 60, pieces });
    expect(stockCells(stock(4.5), 3)).toEqual({ pieces: "4.5個分", held: "270 / 60", tone: "" });
    expect(stockCells(stock(2.96), 3).tone).toBe("low");
    expect(stockCells(stock(0.9), 3).tone).toBe("short");
    expect(stockCells({ held: 2150, about: true, need: 430, pieces: 5.0 }, 4).held).toBe(
      "約2,150 / 430",
    );
  });

  it("tells a drop held short of one student's need, of two, or enough for two", () => {
    expect(heldTone({ exact: 30 }, 75)).toBe("short");
    expect(heldTone({ exact: 100 }, 75)).toBe("fair");
    expect(heldTone({ exact: 150 }, 75)).toBe("enough");
    expect(heldTone({ thousands: 2 }, 75)).toBe("enough");
    expect(heldTone(null, 75)).toBe("short");
    expect(heldTone({ exact: 3 }, null)).toBe("");
  });

  it("writes the farm to the golden ratio, a line a mission, and the missions past it", () => {
    const farm = {
      multiplier: 2,
      runs: 71,
      ap: 710,
      stages: [
        { name: "29-1", runs: 60, ap: 600, slots: ["お守り", "時計"] },
        { name: "25-1", runs: 11, ap: 110, slots: ["帽子"] },
      ],
      next: [{ name: "29-2", slots: ["ヘアピン", "シューズ", "腕時計"] }],
    };
    expect(farmCells(farm)).toEqual({
      lines: [
        { name: "29-1", runs: "60回", ap: "600AP", slots: "お守り・時計" },
        { name: "25-1", runs: "11回", ap: "110AP", slots: "帽子" },
      ],
      total: { runs: "71回", ap: "710AP" },
      next: [{ name: "29-2", slots: "ヘアピン・シューズ・腕時計" }],
    });
  });

  it("names a place without the kind of play it is", () => {
    expect(["指名手配：ハイウェイ", "学園交流会 - トリニティ", "拠点防衛"].map(placeName)).toEqual([
      "ハイウェイ",
      "トリニティ",
      "拠点防衛",
    ]);
  });

  it("counts the students each commission's haul raises, the fewer marked", () => {
    const row = {
      credits_name: "クレジット回収",
      credits: 248955686,
      credits_each: 41500000,
      credit_students: 5.999,
      reports_name: "拠点防衛",
      report_exp: 6245925,
      report_about: true,
      exp_each: 1249185,
      report_students: 5.0,
    };
    expect(commissionCells(row)).toEqual({
      credits: { name: "クレジット回収", students: "6.0人分", detail: "249M ÷ 41.5M" },
      reports: { name: "拠点防衛", students: "5.0人分", detail: "約6.2M EXP ÷ 1.2M EXP" },
      fewer: "reports",
    });
    expect(commissionCells({ ...row, credits: null, credit_students: null }).credits).toEqual({
      name: "クレジット回収",
      students: "?",
      detail: "? ÷ 41.5M",
    });
  });
});

describe("update", () => {
  it("tells where checking for an update stands", () => {
    expect(updateText({ state: "checking" })).toBe("アップデートを確認しています…");
    expect(updateText({ state: "latest", current: "v0.1.29" })).toBe("最新の版です（v0.1.29）");
    expect(updateText({ state: "available", current: "v0.1.29", latest: "v0.1.30" })).toBe(
      "v0.1.30 があります（今 v0.1.29）",
    );
    expect(updateText({ state: "installing", latest: "v0.1.30" })).toBe(
      "v0.1.30 に更新しています…",
    );
    expect(updateText({ state: "failed", error: "通信できません" })).toBe(
      "アップデートできませんでした: 通信できません",
    );
  });
});

describe("phone", () => {
  it("tells where the link to a phone stands", () => {
    expect(phoneLine({ state: "unpaired" })).toBe("スマホとはまだつないでいません");
    expect(phoneLine({ state: "pairing", qr: [] })).toBe(
      "この QR コードを読み取るか、USB ケーブルでつないでください",
    );
    expect(phoneLine({ state: "searching", device: "Pixel 9" })).toBe("Pixel 9 を探しています…");
    expect(phoneLine({ state: "allowing" })).toBe(
      "スマホに出た「USB デバッグを許可しますか？」で許可を押してください",
    );
    expect(phoneLine({ state: "streaming", device: "Pixel 9", via: "wifi" })).toBe(
      "Pixel 9 の画面を Wi-Fi で受け取っています",
    );
    expect(phoneLine({ state: "streaming", device: "TB322FC", via: "usb" })).toBe(
      "TB322FC の画面を USB で受け取っています",
    );
    expect(phoneLine({ state: "stopped", device: "Pixel 9" })).toBe("Pixel 9 とは切断しています");
    expect(phoneLine({ state: "failed", message: "ペア設定できませんでした" })).toBe(
      "ペア設定できませんでした",
    );
  });

  it("draws a QR code's dark modules inside its quiet zone", () => {
    const modules = [
      [true, false],
      [false, true],
    ];
    // Four modules of quiet zone around the code.
    expect(qrPath(modules)).toBe("M4 4h1v1h-1z M5 5h1v1h-1z");
  });
});
