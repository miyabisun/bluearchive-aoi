import { describe, expect, it } from "vitest";
import { filterRows, followTab, formatCount, statusView, studentCells } from "./view.js";

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

describe("followTab", () => {
  const reading = (list) => ({ state: "reading", list, fps: 20, dropped: 0 });

  it("moves to the list the game starts showing", () => {
    expect(followTab({ tab: "items", lastRead: null }, reading("equipment"))).toEqual({
      tab: "equipment",
      lastRead: "equipment",
    });
  });

  it("keeps a tab chosen by hand while the same list goes on being read", () => {
    expect(followTab({ tab: "items", lastRead: "equipment" }, reading("equipment"))).toEqual({
      tab: "items",
      lastRead: "equipment",
    });
  });

  it("stays put while no list is read", () => {
    const view = { tab: "equipment", lastRead: "equipment" };
    expect(followTab(view, { state: "other_screen" })).toEqual(view);
    expect(followTab(view, { state: "no_game" })).toEqual(view);
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
