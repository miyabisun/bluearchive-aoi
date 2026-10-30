import { describe, expect, it } from "vitest";
import {
  blueprintCells,
  boxTotals,
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
  commissionRows,
  currencyRows,
  currencyCoins,
  normalItemRows,
  scrimmageRows,
  enhancementCells,
  hardTile,
  hardCost,
  starMarks,
  starChoices,
  menuPlace,
  planCard,
  planCoin,
  planTitle,
  schoolCells,
  farmCells,
  goodTile,
  goodFace,
  heldTone,
  heldText,
  tileTitle,
  placeName,
  stockCells,
  goldenTitle,
  updateText,
  castLine,
  castNote,
  phoneLine,
  qrPath,
  shopStale,
  formatCount,
  sortStudents,
  statusView,
  studentCells,
  starTitle,
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
  const at = (status, links = {}) => statusView({ status, ...links });

  it("names each state and marks which are trouble", () => {
    const badge = (status) => {
      const { label, tone } = at(status);
      return { label, tone };
    };
    expect(badge({ state: "no_game" })).toEqual({ label: "ゲーム未検出", tone: "idle" });
    expect(badge({ state: "no_source" })).toEqual({ label: "取り込み元なし", tone: "idle" });
    expect(badge({ state: "other_screen" })).toEqual({ label: "対象外の画面", tone: "idle" });
    expect(badge({ state: "reading", list: "items", fps: 27.5, dropped: 3 })).toEqual({
      label: "アイテムを集計中",
      tone: "active",
    });
    expect(badge({ state: "reading", list: "equipment", fps: 9.6, dropped: 0 })).toEqual({
      label: "装備を集計中",
      tone: "active",
    });
    expect(badge({ state: "failed", message: "denied" })).toEqual({
      label: "キャプチャ失敗",
      tone: "danger",
    });
    expect(badge({ state: "paused" })).toEqual({ label: "一時中断", tone: "paused" });
    expect(badge({ state: "too_small" })).toEqual({
      label: "画面が小さすぎます",
      tone: "idle",
    });
  });

  it("shows where the frames come from by its icon, and says it and the frame rate on hover", () => {
    const reading = { state: "reading", list: "shops", fps: 4.8, dropped: 0 };
    const tap = "押すと読み取りを一時中断";
    const lines = (...l) => l.join("\n");
    expect(at(reading)).toMatchObject({
      source: "window",
      title: lines("取り込み元: ゲームのウィンドウ", "5 fps", tap),
    });
    const usb = { state: "streaming", device: "Pixel 9", via: "usb" };
    expect(at(reading, { phone: usb })).toMatchObject({
      source: "phone",
      title: lines("取り込み元: Pixel 9（USB）", "5 fps", tap),
    });
    const wifi = { state: "streaming", device: "TB322FC", via: "wifi" };
    expect(at({ state: "paused" }, { phone: wifi })).toMatchObject({
      source: "phone",
      title: lines("取り込み元: TB322FC（Wi-Fi）", "押すと読み取りを再開"),
    });
    // A phone linked but not sending leaves the window the source.
    expect(at(reading, { phone: { state: "searching", device: "Pixel 9" } })).toMatchObject({
      source: "window",
    });
    const cast = { state: "receiving", from: "192.168.1.5", sideways: false };
    expect(at(reading, { cast })).toMatchObject({
      source: "cast",
      title: lines("取り込み元: 配信アプリ（192.168.1.5）", "5 fps", tap),
    });
    expect(at({ state: "no_source" })).toMatchObject({ source: null, title: tap });
  });
});

describe("formatCount", () => {
  it("shows exact counts with separators and rounded ones as the game does", () => {
    expect(formatCount({ exact: 1383 })).toBe("1,383");
    expect(formatCount({ exact: 7 })).toBe("7");
    expect(formatCount({ thousands: 51 })).toBe("51K");
  });
});

describe("starTitle", () => {
  it("tells a student's star on hover: the unique weapon's level, then the elephs held", () => {
    const NL = String.fromCharCode(10);
    expect(starTitle("固有武器 Lv60", { exact: 148 })).toBe(
      ["固有武器 Lv60", "神名文字: 148"].join(NL),
    );
    expect(starTitle("", { thousands: 2 })).toBe("神名文字: 2K");
    // Never read in the item list: not told from none held.
    expect(starTitle("", undefined)).toBe("神名文字: ?");
  });
});

describe("studentCells", () => {
  const known = {
    level: 90,
    rarity: 5,
    bond: 34,
    skills: [5, 10, 7, 10],
    weapon: { level: 60, stars: 4 },
    equipment: [{ tier: 10 }, { tier: 9 }, { tier: 8 }],
    gear: 2,
    ability: [25, 0, 25],
  };

  it("writes a student's training the way timeline-arona does", () => {
    expect(studentCells(known)).toEqual({
      level: "90",
      star: 8,
      weapon: "固有武器 Lv60",
      skills: "5M7M",
      equipment: "T10/9/8",
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
      equipment: [{ tier: 1 }, null, { tier: 1 }],
      gear: 0,
      ability: [0, null, 0],
    });
    expect(cells.level).toBe("?");
    // Under ★5 the weapon is locked; at ★5 its stars must be read.
    expect(studentCells({ ...known, rarity: 3, weapon: null })).toMatchObject({
      star: 3,
      weapon: "",
    });
    expect(studentCells({ ...known, rarity: null }).star).toBe(null);
    expect(cells.skills).toBe("3?11");
    expect(cells.equipment).toBe("T1/?/1");
    expect(cells.gear).toBe("—");
    expect(cells.ability).toBe("0/?/0");
    expect(studentCells({ ...known, weapon: undefined })).toMatchObject({ star: null, weapon: "" });
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
      equipment: [{ tier: 10 }, { tier: 10 }, { tier: 10 }],
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
      student(1, { equipment: [{ tier: 9 }, { tier: 10 }, { tier: 10 }] }),
      student(2, { equipment: [{ tier: 8 }, { tier: 10 }, { tier: 10 }] }),
      student(3),
    ];
    expect(ids(sortStudents(geared, "equipment"))).toEqual([3, 1, 2]);
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

  it("starts a shop's month on the 1st at 4:00", () => {
    const november = (hour, minute) => new Date(2026, 10, 1, hour, minute);
    const october = shop("総力戦", "2026-10-31 23:00:00", true, []);
    expect(shopStale(october, november(3, 59))).toBe(false);
    expect(shopStale(october, november(4, 0))).toBe(true);
    // Read in the 1st's first hours, it is the month before's.
    const early = shop("総力戦", "2026-11-01 03:30:00", true, []);
    expect(shopStale(early, november(3, 50))).toBe(false);
    expect(shopStale(early, november(4, 0))).toBe(true);
    // New Year's first hours are December's.
    const december = shop("総力戦", "2025-12-31 22:00:00", true, []);
    expect(shopStale(december, new Date(2026, 0, 1, 3, 0))).toBe(false);
    expect(shopStale(december, new Date(2026, 0, 1, 4, 0))).toBe(true);
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

  it("paints a good on its rarity, a student's eleph on its own, and one unknown on none", () => {
    expect(goodFace({ rarity: "SR", eleph: false })).toBe("SR");
    expect(goodFace({ rarity: "N", eleph: true })).toBe("eleph");
    expect(goodFace({ rarity: null, eleph: false })).toBeUndefined();
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

  it("draws a star as the game does: the student's in yellow, the unique weapon's in blue", () => {
    expect(starMarks(3)).toEqual({ kind: "student", places: [0, 1, 2], label: "★3" });
    expect(starMarks(4)).toMatchObject({ kind: "student", places: [0, 1, 2, 3] });
    // 固有1 (the student's ★5) to 固有4 as the weapon's stars.
    expect(starMarks(5)).toEqual({ kind: "weapon", places: [0], label: "固有1" });
    expect(starMarks(7)).toEqual({ kind: "weapon", places: [0, 1, 2], label: "固有3" });
    expect(starMarks(8).places).toHaveLength(4);
  });

  it("offers every star as a target, none below the student's now", () => {
    expect(starChoices(3)).toEqual([
      { star: 1, disabled: true },
      { star: 2, disabled: true },
      { star: 3, disabled: false },
      { star: 4, disabled: false },
      { star: 5, disabled: false },
      { star: 6, disabled: false },
      { star: 7, disabled: false },
      { star: 8, disabled: false },
    ]);
    // A star never read takes any.
    expect(starChoices(null).every((c) => !c.disabled)).toBe(true);
  });

  it("opens a menu under its button, over it where the window ends first", () => {
    const window = { width: 720, height: 800 };
    const menu = { width: 100, height: 250 };
    const button = { top: 100, bottom: 130, left: 40 };
    expect(menuPlace(button, window, menu)).toEqual({ left: 40, top: 134 });
    // 310 + 4 + 250 runs past 400: above the button instead.
    const low = { top: 280, bottom: 310, left: 40 };
    expect(menuPlace(low, { ...window, height: 400 }, menu)).toEqual({ left: 40, top: 26 });
    // Nowhere whole: below, from the window's top at the least.
    const short = { width: 720, height: 200 };
    expect(menuPlace({ top: 20, bottom: 50, left: 0 }, short, menu)).toEqual({ left: 4, top: 4 });
    // A button by the window's right edge: the menu kept inside it.
    const right = { top: 100, bottom: 130, left: 300 };
    expect(menuPlace(right, { width: 360, height: 800 }, menu)).toEqual({ left: 256, top: 134 });
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
    // Short of the ratio never shows as reaching it: 238 / 60 is 3.96, cut to a tenth.
    expect(stockCells({ held: 238, about: false, need: 60, pieces: 238 / 60 }, 4)).toMatchObject({
      pieces: "3.9個分",
      tone: "low",
    });
    expect(stockCells({ held: 2150, about: true, need: 430, pieces: 5.0 }, 4).held).toBe(
      "約2,150 / 430",
    );
  });

  it("tells why a slot's golden ratio is so many: its share of every student's slots in 24", () => {
    expect(goldenTitle({ students: 3, slots: 103, all: 834, total: 24 })).toBe(
      [
        "黄金比 3人",
        "全実装生徒の装備欄 834 のうち 103",
        "103 ÷ 834 × 24 = 2.96（0.25以上は切り上げ）",
      ].join("\n"),
    );
    // Cut to two decimals as the rounding goes by: 2.129… shows 2.12, never 2.13.
    expect(goldenTitle({ students: 2, slots: 74, all: 834, total: 24 })).toContain("= 2.12（");
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

  // The reports by tier, as the enhancement materials show theirs.
  const reportTiers = [0, 1, 2, 3].map((t) => ({ id: 10 + t, held: { exact: t } }));

  it("counts the students each commission's haul raises, the reports first, the fewer marked", () => {
    const row = {
      credits_name: "クレジット回収",
      credits: 248955686,
      credits_each: 41500000,
      credit_students: 5.999,
      reports_name: "拠点防衛",
      report_tiers: reportTiers,
      report_exp: 6245925,
      report_about: true,
      exp_each: 1249185,
      report_students: 5.0,
    };
    // A commission's name over its haul held: the reports by tier, the credits as a
    // count beside their icon; the students raised and how at the right.
    expect(commissionRows(row)).toEqual([
      {
        key: "reports",
        title: "拠点防衛",
        tiers: reportTiers,
        students: "5.0人分",
        numerator: "約6.2M EXP",
        denominator: "/ 1.2M EXP",
        tone: "fewer",
      },
      {
        key: "credits",
        title: "クレジット回収",
        held: "248,955,686",
        students: "5.9人分",
        numerator: "249M",
        denominator: "/ 41.5M",
        tone: "more",
      },
    ]);
    // Credits never read: no count, and neither raises fewer.
    expect(commissionRows({ ...row, credits: null, credit_students: null })).toMatchObject([
      { tone: "" },
      { held: "—", students: "?", numerator: "?", denominator: "/ 41.5M", tone: "" },
    ]);
  });

  it("lists the credits, then the reports, as the commissions count them", () => {
    const row = {
      credits_name: "クレジット回収",
      credits: 248955686,
      credits_each: 41500000,
      credit_students: 5.999,
      reports_name: "拠点防衛",
      report_tiers: reportTiers,
      report_exp: 6245925,
      report_about: false,
      exp_each: 1249185,
      report_students: 5.0,
    };
    const [reports, credits] = commissionRows(row);
    expect(currencyRows(row)).toEqual([
      { ...credits, title: "クレジット" },
      { ...reports, title: "レポート" },
    ]);
  });

  it("lists the coins held, then the shops' coins the item list lacks, each once", () => {
    const coin = (name, held) => ({ id: 1, name, icon: `icon_${name}`, held, need: null });
    const coins = [coin("総力戦コイン", { exact: 1160 }), coin("戦術コイン", null)];
    const shop = (...names) => ({
      coins: names.map((name) => ({ name, icon: `icon_${name}`, held: { exact: 50 } })),
    });
    const shops = [shop("総力戦レアコイン", "総力戦コイン"), shop("熟達証書"), shop("熟達証書")];
    expect(currencyCoins(coins, shops).map((c) => [c.name, c.held])).toEqual([
      ["総力戦コイン", { exact: 1160 }],
      ["戦術コイン", null],
      ["総力戦レアコイン", { exact: 50 }],
      ["熟達証書", { exact: 50 }],
    ]);
  });

  it("names each school's scrimmage over its parts held, the first to run out marked", () => {
    const parts = (id) => [0, 1, 2, 3].map((t) => ({ id: id + t, held: { exact: t } }));
    const places = [
      { name: "学園交流会 - トリニティ", kinds: [parts(10)] },
      { name: "学園交流会 - ゲヘナ", kinds: [parts(20)] },
    ];
    const school = { exp: 220280, about: false, rate: 1.5, exp_each: 49605, share: 0.4593 };
    const schools = [
      { ...school, students: 6.661, first: true },
      { ...school, students: 7.0, first: false, share: 0.26 },
    ];
    const rows = scrimmageRows(places, schools);
    expect(rows).toMatchObject([
      {
        key: "学園交流会 - トリニティ",
        title: "トリニティ",
        tiers: parts(10),
        students: "6.6人分",
        numerator: "330.4K EXP",
        denominator: "/ 49.6K EXP",
        tone: "first",
      },
      {
        key: "学園交流会 - ゲヘナ",
        title: "ゲヘナ",
        tiers: parts(20),
        students: "7.0人分",
        numerator: "330.4K EXP",
        denominator: "/ 49.6K EXP",
        tone: "",
      },
    ]);
    // Why on hover, as `schoolCells` tells it.
    expect(rows[0].why).toBe(schoolCells(schools[0]).why);
  });
});

describe("month-end and daily shops", () => {
  const commissions = {
    credits_name: "クレジット回収",
    credits: 248955686,
    credits_each: 41500000,
    credit_students: 5.999,
    reports_name: "拠点防衛",
    report_exp: 6245925,
    report_about: false,
    exp_each: 1249185,
    report_students: 5.0,
  };

  it("counts the students the credits and the orbs held raise, the fewer marked", () => {
    // The orbs as the equipment tab's enhancement materials show them.
    const tiers = [0, 1, 2, 3].map((t) => ({ id: 1 + t, held: { exact: t } }));
    const raise = { exp: 5700780, about: true, rate: 1, exp_each: 741690, students: 7.686 };
    const orbs = { category: "Exp", tiers, raise };
    expect(normalItemRows(commissions, orbs)).toEqual([
      {
        key: "credits",
        title: "クレジット",
        held: "248,955,686",
        students: "5.9人分",
        numerator: "249M",
        denominator: "/ 41.5M",
        tone: "fewer",
      },
      {
        key: "orbs",
        title: "強化珠",
        tiers,
        students: "7.6人分",
        numerator: "約5.7M EXP",
        denominator: "/ 0.74M EXP",
        tone: "more",
      },
    ]);
    const unread = { ...commissions, credits: null, credit_students: null };
    expect(normalItemRows(unread, orbs).map((r) => r.tone)).toEqual(["", ""]);
    // Before the equipment is counted by kind, the credits alone.
    expect(normalItemRows(commissions, undefined)).toMatchObject([{ key: "credits", tone: "" }]);
  });

  it("tells a card's times left at its price, bought up, or its student done", () => {
    const card = { left: 16, price: 50, amount: 5, barred: false };
    expect(planCard(card)).toEqual({ amount: "×5", text: "16回@50", star: null });
    expect(planCard({ ...card, price: null, amount: null })).toEqual({
      amount: "",
      text: "16回@?",
      star: null,
    });
    expect(planCard({ ...card, left: 0 })).toEqual({ amount: "×5", text: "売切れ", star: null });
    // The student at 固有4: the weapon's four stars in the place of the times left.
    expect(planCard({ ...card, left: 0, barred: true })).toEqual({
      amount: "×5",
      text: "",
      star: 8,
    });
  });

  it("tells on hover how many of a card's good are held, and an eleph's student's star", () => {
    const azusa = { id: 10019, name: "アズサの神名文字", held: { exact: 120 }, eleph: true };
    const students = [
      { id: 10019, name: "アズサ", training: { rarity: 5, weapon: { stars: 2, level: 40 } } },
      { id: 10020, name: "ハナコ", training: { rarity: 3, weapon: null } },
    ];
    const lines = (...l) => l.join("\n");
    expect(planTitle(azusa, students)).toBe(lines("アズサの神名文字", "星: 固有2", "所持: 120"));
    expect(planTitle({ ...azusa, id: 10020, name: "ハナコの神名文字" }, students)).toBe(
      lines("ハナコの神名文字", "星: ★3", "所持: 120"),
    );
    expect(planTitle({ ...azusa, id: 10099, held: null }, students)).toBe(
      lines("アズサの神名文字", "星: 未読み取り", "所持: ?"),
    );
    const note = { id: 9999, name: "秘伝ノート", held: { thousands: 2 }, eleph: false };
    expect(planTitle(note, students)).toBe(lines("秘伝ノート", "所持: 2K"));
  });

  it("tells the coins left once the cards reserved are paid, marked while any is reserved", () => {
    const coin = { name: "総力戦コイン", icon: "", held: { exact: 600 }, need: 200, short: false };
    expect(planCoin(coin)).toEqual({
      left: "400",
      reserved: true,
      short: false,
      title: "総力戦コイン: 所持 600 − 予約 200",
    });
    // Nothing reserved: the count held, unmarked.
    expect(planCoin({ ...coin, need: 0 })).toMatchObject({ left: "600", reserved: false });
    expect(planCoin({ ...coin, need: 1700, short: true })).toMatchObject({
      left: "-1,100",
      short: true,
    });
    expect(planCoin({ ...coin, held: { thousands: 3 }, need: 500 })).toMatchObject({
      left: "2,500",
    });
    expect(planCoin({ ...coin, held: null })).toEqual({
      left: "?",
      reserved: true,
      short: false,
      title: "総力戦コイン: 所持 ? − 予約 200",
    });
  });
});

describe("item tiles", () => {
  it("tells on hover an eleph's student's star, any other item by its name", () => {
    const students = [
      { id: 10019, name: "アズサ", training: { rarity: 5, weapon: { stars: 2, level: 40 } } },
      { id: 10020, name: "ハナコ", training: { rarity: 3, weapon: null } },
    ];
    const eleph = (id, name) => ({ id, name, eleph: true });
    const lines = (...l) => l.join("\n");
    expect(tileTitle(eleph(10019, "アズサの神名文字"), students)).toBe(
      lines("アズサの神名文字", "星: 固有2"),
    );
    expect(tileTitle(eleph(10020, "ハナコの神名文字"), students)).toBe(
      lines("ハナコの神名文字", "星: ★3"),
    );
    expect(tileTitle(eleph(10099, "ミカの神名文字"), students)).toBe(
      lines("ミカの神名文字", "星: 未読み取り"),
    );
    expect(tileTitle({ id: 9999, name: "秘伝ノート", eleph: false }, students)).toBe("秘伝ノート");
  });

  it("writes a count held, a dash where its list never read it", () => {
    expect(heldText({ exact: 1234 })).toBe("1,234");
    expect(heldText({ thousands: 51 })).toBe("51K");
    expect(heldText(null)).toBe("—");
  });
});

describe("hard missions", () => {
  const card = {
    id: 10000,
    name: "アルの神名文字",
    stages: ["18-3"],
    ap: 60,
    star: 7,
    left: 120,
    held: { exact: 80 },
    done: false,
    reserved: false,
  };
  const lines = (...l) => l.join("\n");

  it("tells a student's card: the missions it is dropped at, the star and the elephs held", () => {
    // The user's アル: 148 held at 固有3 reads "固有3 148文字", never what 固有4 still takes (52).
    const aru = { ...card, held: { exact: 148 } };
    expect(hardTile(aru)).toEqual({
      amount: "1個所",
      star: 7,
      held: "148文字",
      title: lines("アルの神名文字", "星: 固有3", "所持: 148", "任務 Hard: 18-3", "1日 60 AP"),
    });
    const yuuka = {
      ...card,
      stages: ["1-1", "4-3", "12-2"],
      ap: 180,
      star: 2,
      held: { exact: 1280 },
    };
    expect(hardTile(yuuka)).toMatchObject({ amount: "3個所", star: 2, held: "1,280文字" });
    expect(hardTile(yuuka).title).toContain("任務 Hard: 1-1、4-3、12-2");
    // A student never read, the item list never read; one at 固有4.
    const unread = { ...card, star: null, held: null };
    expect(hardTile(unread)).toMatchObject({ star: null, held: "?文字" });
    expect(hardTile(unread).title).toContain("星: 未読み取り\n所持: ?");
    expect(hardTile({ ...card, star: 8, done: true })).toMatchObject({ star: 8, held: "80文字" });
  });

  it("counts a day's AP of the students reserved, those at 固有4 too", () => {
    const reserved = (ap, done = false) => ({ ...card, ap, done, reserved: true });
    expect(hardCost([reserved(60), reserved(180), { ...card, ap: 120 }])).toBe(240);
    // Elephs farmed past 固有4 wait for a 固有5 the game hints at (the user's of 2026-10-22).
    expect(hardCost([reserved(60), reserved(120, true)])).toBe(180);
    expect(hardCost([])).toBe(0);
  });
});

describe("enhancementCells", () => {
  it("tells how many students a kind of enhancement material raises, as the work tab counts it", () => {
    const raise = { exp: 5700780, about: true, rate: 1, exp_each: 741690, students: 7.686 };
    // The orbs in millions, as the work tab's normal items shop: the count, the exp held
    // over the exp one student takes (the user's of 2026-10-29: three lines).
    expect(enhancementCells({ category: "Exp", raise })).toEqual({
      students: "7.6人分",
      numerator: "約5.7M EXP",
      denominator: "/ 0.74M EXP",
    });
    // A kind of parts with its weapons' rate taken in (the user's 「x1.5倍は計算してしまう」),
    // thousands as K rounded to a tenth (49,605 → 49.6K).
    const parts = { exp: 220280, about: false, rate: 1.5, exp_each: 49605, students: 6.661 };
    expect(enhancementCells({ category: "WeaponExpGrowthA", raise: parts })).toEqual({
      students: "6.6人分",
      numerator: "330.4K EXP",
      denominator: "/ 49.6K EXP",
    });
    // Rounded half up, not cut: 172,650 is 172.7K; below a thousand as it is, a million on in M.
    const at = (exp, rate = 1) =>
      enhancementCells({ category: "WeaponExpGrowthZ", raise: { ...parts, exp, rate } }).numerator;
    expect([at(172650), at(999), at(1_234_567)]).toEqual(["172.7K EXP", "999 EXP", "1.2M EXP"]);
    expect(at(129050, 1.5)).toBe("193.6K EXP");
    // Always to a tenth, as the counts of students: 258,975 is 259.0K.
    expect(at(172650, 1.5)).toBe("259.0K EXP");
  });
});

describe("schoolCells", () => {
  const NL = String.fromCharCode(10);
  const lines = (...l) => l.join(NL);

  it("counts the weapons a school's parts raise and how, the first to run out marked", () => {
    const row = {
      exp: 220280,
      about: false,
      rate: 1.5,
      exp_each: 49605,
      students: 6.661,
      share: 0.4593,
      first: true,
    };
    expect(schoolCells(row)).toEqual({
      students: "6.6人分",
      numerator: "330.4K EXP",
      denominator: "/ 49.6K EXP",
      first: true,
      // Why it is red, on hover: the students its parts suit, and per them.
      why: lines("素材が合う生徒: 46%", "6.6人分 ÷ 46% = 14.5", "最も少ないので先に回します"),
    });
    expect(schoolCells({ ...row, about: true, first: false })).toMatchObject({
      numerator: "約330.4K EXP",
      first: false,
      why: lines("素材が合う生徒: 46%", "6.6人分 ÷ 46% = 14.5"),
    });
  });
});

describe("boxTotals", () => {
  const cell = (id, held) => ({ id, name: `箱${id}`, icon: `icon_${id}`, held, need: null });
  const set = (bd, notes) => ({ all: false, schools: [], bd, notes });

  it("adds every set's boxes tier by tier and says how much each holds", () => {
    const sets = [
      set(
        [cell(1, { exact: 429 }), cell(2, null), null, cell(4, { exact: 9000 })],
        [cell(5, null), null, null, null],
      ),
      set(
        [cell(11, { exact: 5 }), cell(12, null), null, cell(14, { exact: 2000 })],
        [cell(15, null), null, null, null],
      ),
      set([null, cell(22, null), null, cell(24, { thousands: 12 })], [null, null, null, null]),
    ];
    const totals = boxTotals(sets);
    // A set without the tier's box, or holding none, counts 0.
    expect(totals.bd[0]).toEqual({ ...cell(1, { exact: 434 }), parts: "429 + 5 + 0" });
    // None held anywhere stays unknown, as one box would.
    expect(totals.bd[1]).toEqual({ ...cell(2, null), parts: "0 + 0 + 0" });
    expect(totals.bd[2]).toBeNull();
    // From 10,000 in thousands, as the game writes it.
    expect(totals.bd[3]).toEqual({
      ...cell(4, { thousands: 23 }),
      parts: "9,000 + 2,000 + 12K",
    });
    expect(totals.notes).toEqual([{ ...cell(5, null), parts: "0 + 0 + 0" }, null, null, null]);
  });

  it("has nothing without sets", () => {
    expect(boxTotals([])).toBeNull();
  });
});

describe("update", () => {
  it("tells where checking for an update stands", () => {
    expect(updateText({ state: "checking" })).toBe("アップデートを確認しています…");
    expect(updateText({ state: "latest", current: "v0.1.29" })).toBe(
      "最新のバージョンです（v0.1.29）",
    );
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

describe("streaming app", () => {
  const waiting = {
    state: "waiting",
    url: "rtmp://192.168.1.20:1935/aoi/Ab3",
    moblin: [],
    plain: [],
  };

  it("tells where waiting for a streaming app stands", () => {
    expect(castLine({ state: "off" })).toBe(
      "待ち受けると、スマホの配信アプリ（iPhone は Moblin）から画面を受け取れます",
    );
    expect(castLine({ ...waiting, problem: null })).toBe(
      "配信アプリでこの QR コードを読み取り、配信を始めてください",
    );
    expect(castLine({ state: "receiving", from: "192.168.1.20", sideways: false })).toBe(
      "192.168.1.20 から画面を受け取っています",
    );
    expect(castLine({ state: "failed", message: "ポート 1935 で待ち受けられません" })).toBe(
      "ポート 1935 で待ち受けられません",
    );
  });

  it("adds what went wrong with the last stream, or how to send the screen larger", () => {
    expect(castNote({ ...waiting, problem: null })).toBeNull();
    expect(castNote({ ...waiting, problem: "キーの違う配信が届いたので断りました" })).toBe(
      "キーの違う配信が届いたので断りました",
    );
    expect(castNote({ state: "receiving", from: "192.168.1.20", sideways: false })).toBeNull();
    expect(castNote({ state: "receiving", from: "192.168.1.20", sideways: true })).toBe(
      "画面が横倒しで小さく届いています。配信アプリで縦向き（Portrait）の配信にすると大きく届きます",
    );
    expect(castNote({ state: "off" })).toBeNull();
  });
});
