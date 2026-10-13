import { describe, expect, it } from "vitest";
import {
  filterRows,
  filterShops,
  groupPlan,
  needCells,
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
      rarity: "★5",
      weapon: "★4 Lv60",
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
    expect(cells.weapon).toBe("—");
    expect(cells.skills).toBe("3?11");
    expect(cells.equipment).toBe("T1/?/1");
    expect(cells.equipmentLevels).toBe("Lv1/?/10");
    expect(cells.gear).toBe("—");
    expect(cells.ability).toBe("0/?/0");
    expect(studentCells({ ...known, weapon: undefined }).weapon).toBe("?");
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

  it("puts the highest level first, then the rarity, then the bond, by default", () => {
    const rows = [
      student(1, { level: 80, bond: 50 }),
      student(2, { rarity: 4, bond: 50 }),
      student(3, { bond: 20 }),
      student(4, { bond: 30, weapon: { stars: 1, level: 30 }, skills: [1, 1, 1, 1] }),
    ];
    expect(ids(sortStudents(rows))).toEqual([4, 3, 2, 1]);
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
    expect(ids(sortStudents(rows, "weapon"))).toEqual([6, 5, 4, 1, 2, 3]);
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

  it("names a star: the student's 1 to 5, then the unique weapon's", () => {
    expect([1, 4, 5, 6, 8].map(starName)).toEqual(["★1", "★4", "★5", "固有2", "固有4"]);
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

  it("tells a drop held short of one student's need red, of two orange", () => {
    expect(heldTone({ exact: 30 }, 75)).toBe("short");
    expect(heldTone({ exact: 100 }, 75)).toBe("low");
    expect(heldTone({ thousands: 2 }, 75)).toBe("");
    expect(heldTone(null, 75)).toBe("short");
    expect(heldTone({ exact: 3 }, null)).toBe("");
  });

  it("writes the farm to the golden ratio, a line a mission", () => {
    const farm = {
      multiplier: 2,
      runs: 71,
      ap: 710,
      stages: [
        { name: "29-1", runs: 60, ap: 600, slots: ["お守り", "時計"] },
        { name: "25-1", runs: 11, ap: 110, slots: ["帽子"] },
      ],
    };
    expect(farmCells(farm)).toEqual({
      lines: [
        { name: "29-1", runs: "60回", ap: "600AP", slots: "お守り・時計" },
        { name: "25-1", runs: "11回", ap: "110AP", slots: "帽子" },
      ],
      total: "計 71回 710AP（1日 102AP）",
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
