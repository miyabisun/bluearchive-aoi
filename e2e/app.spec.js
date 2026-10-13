import { existsSync } from "node:fs";
import { expect, test } from "@playwright/test";

const items = Array.from({ length: 120 }, (_, i) => ({
  id: 1000 + i,
  name:
    i === 3
      ? "とても長い名前の期間限定でない普通のアイテム（ゲヘナ・トリニティ・ミレニアム共通）"
      : `アイテム${i}`,
  icon: `item_icon_skillbook_gehenna_${i % 4}`,
  tier: null,
  count: i === 5 ? { thousands: 51 } : { exact: i * 997 },
}));

const equipment = [
  { id: 1, name: "初級強化珠", icon: "equipment_icon_exp_0", tier: null, count: { exact: 1383 } },
  {
    id: 1000,
    name: "無地のキャップ",
    icon: "equipment_icon_hat_tier1",
    tier: 1,
    count: { exact: 249 },
  },
  {
    id: 101009,
    name: "ゲーミングヘルメットの設計図",
    icon: "equipment_icon_hat_tier10_piece",
    tier: 10,
    count: { exact: 17 },
  },
  {
    id: 501000,
    name: "帽子の万能設計図",
    icon: "equipment_icon_hat_useall_piece",
    tier: null,
    count: { thousands: 51 },
  },
];

/** Where the app's icons are, which the page loads from `/icon/<name>` here. */
const ICONS = ["src-tauri/data/schaledb/icons", "src-tauri/data/schaledb/equipment-icons"];

const reading = (list, fps = 27.4) => ({ state: "reading", list, fps, dropped: 0 });

/**
 * Opens the page showing `snapshot` as the watcher's, with the app's icons,
 * and the `tab` named, if any, chosen.
 */
async function open(page, snapshot, tab) {
  await page.route("**/icon/*", (route) => {
    const name = new URL(route.request().url()).pathname.slice("/icon/".length);
    const file = ICONS.map((dir) => `${dir}/${name}.webp`).find(existsSync);
    return file ? route.fulfill({ path: file }) : route.fulfill({ status: 404 });
  });
  await page.addInitScript((demo) => (window.__AOI_DEMO__ = demo), snapshot);
  await page.goto("/");
  if (tab) await page.getByRole("tab", { name: tab }).click();
}

/** The names of the tiles shown, as their hover tells them. */
const tileNames = (page) =>
  page.locator(".tiles li").evaluateAll((tiles) => tiles.map((t) => t.title));

const overflow = (page) =>
  page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);

test("lists the item tally in order and narrows it by name", async ({ page }) => {
  await open(page, { status: reading("items"), items, equipment });
  await expect(page.getByRole("status")).toHaveText("アイテムを集計中 27 fps");
  await expect(page.getByRole("tab", { name: "アイテム" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  const tiles = page.locator(".tiles li");
  await expect(tiles).toHaveCount(120);
  await expect(tiles.nth(2)).toHaveText("1,994");
  await expect(tiles.nth(2)).toHaveAttribute("title", "アイテム2");
  await expect(tiles.nth(2).getByRole("img", { name: "アイテム2" })).toBeVisible();
  await expect(tiles.nth(5)).toHaveText("51K");

  await page.getByLabel("名前で検索").fill("ｱｲﾃﾑ11");
  await expect(tiles).toHaveCount(11);
  expect(await tileNames(page)).toEqual([
    "アイテム11",
    ...Array.from({ length: 10 }, (_, i) => `アイテム${110 + i}`),
  ]);
  await page.getByLabel("名前で検索").fill("ない名前");
  await expect(page.getByText("一致する項目がありません")).toBeVisible();
});

test("shows the equipment by tile, each piece's tier at its corner", async ({ page }) => {
  await open(page, { status: reading("equipment"), items, equipment });
  await expect(page.getByRole("status")).toHaveText("装備を集計中 27 fps");
  // The tab stays where it was: the game's screen does not choose it.
  await expect(page.getByRole("tab", { name: "アイテム" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await page.getByRole("tab", { name: "装備" }).click();
  await expect(page.locator(".tiles li")).toHaveText(["1,383", "T1249", "T1017", "51K"]);
  await expect(page.locator(".tiles li .badge")).toHaveText(["T1", "T10"]);
  expect(await tileNames(page)).toEqual([
    "初級強化珠",
    "無地のキャップ",
    "ゲーミングヘルメットの設計図",
    "帽子の万能設計図",
  ]);
  // The icons load from the app.
  const loaded = await page
    .locator(".tiles img")
    .evaluateAll((imgs) => imgs.map((i) => i.complete && i.naturalWidth > 0));
  expect(loaded).toEqual([true, true, true, true]);

  await page.getByRole("tab", { name: "アイテム" }).click();
  await expect(page.locator(".tiles li")).toHaveCount(120);
});

test("the status pauses the reading when pressed, and resumes it", async ({ page }) => {
  const snapshot = { status: reading("items"), items, equipment };
  await open(page, snapshot);
  const badge = page.getByRole("button", { name: /アイテムを集計中/ });
  await expect(badge).toHaveAttribute("aria-pressed", "false");
  await badge.click();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([{ paused: true }]);
  // The app answers with the reading paused: the lamp red.
  await page.evaluate((s) => window.__AOI_PUSH__({ ...s, status: { state: "paused" } }), snapshot);
  const paused = page.getByRole("button", { name: "一時中断" });
  await expect(paused).toHaveAttribute("aria-pressed", "true");
  const lamp = (selector) =>
    page.locator(selector).evaluate((dot) => getComputedStyle(dot).backgroundColor);
  const danger = await page.evaluate(() =>
    getComputedStyle(document.documentElement).getPropertyValue("--c-danger").trim(),
  );
  const red = await page.evaluate((c) => {
    const probe = document.createElement("span");
    probe.style.color = c;
    document.body.append(probe);
    const rgb = getComputedStyle(probe).color;
    probe.remove();
    return rgb;
  }, danger);
  expect(await lamp(".status .dot")).toBe(red);
  await paused.press("Enter");
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([
    { paused: true },
    { paused: false },
  ]);
});

test("guides to the list before anything is read", async ({ page }) => {
  await open(page, { status: { state: "other_screen" }, items: [], equipment });
  await expect(page.getByRole("status")).toHaveText("対象外の画面");
  await expect(page.locator("[data-state=empty]")).toContainText("アイテム一覧");
  await expect(page.getByLabel("名前で検索")).toHaveCount(0);
  await page.getByRole("tab", { name: "装備" }).click();
  await expect(page.locator(".tiles li")).toHaveCount(4);
});

test("shows why the capture failed", async ({ page }) => {
  await open(page, {
    status: { state: "failed", message: "アクセスが拒否されました。" },
    items,
    equipment,
  });
  await expect(page.getByRole("status")).toHaveText("キャプチャ失敗");
  await expect(page.getByRole("alert")).toHaveText("アクセスが拒否されました。");
});

for (const scheme of ["dark", "light"]) {
  test(`fits a narrow window in ${scheme}`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await page.setViewportSize({ width: 320, height: 640 });
    await open(page, { status: reading("equipment", 9.6), items, equipment });
    expect(await overflow(page)).toBe(0);
    await expect(page.locator("body")).toHaveCSS(
      "background-color",
      scheme === "dark" ? "rgb(25, 25, 25)" : "rgb(250, 246, 239)",
    );
    await page.screenshot({ path: `test-results/narrow-${scheme}.png`, fullPage: false });
  });
}

test("the tabs and the search field take focus from the keyboard", async ({ page }) => {
  await open(page, { status: reading("items", 20), items, equipment });
  await page.keyboard.press("Tab");
  await expect(page.getByRole("button", { name: /アイテムを集計中/ })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("button", { name: "メニュー" })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("tab", { name: "アイテム" })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("tab", { name: "装備" })).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("tab", { name: "装備" })).toHaveAttribute("aria-selected", "true");
  await page.keyboard.press("Tab");
  await expect(page.getByRole("tab", { name: "ショップ" })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("tab", { name: "生徒" })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("tab", { name: "育成" })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("tab", { name: "購買戦略" })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("tab", { name: "お仕事" })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByLabel("名前で検索")).toBeFocused();
  await page.screenshot({ path: "test-results/wide-dark.png" });
});

const students = [
  {
    id: 10005,
    name: "ホシノ",
    training: {
      level: 90,
      rarity: 5,
      bond: 34,
      skills: [5, 10, 10, 10],
      weapon: { level: 60, stars: 4 },
      equipment: [
        { tier: 10, level: 70 },
        { tier: 10, level: 70 },
        { tier: 10, level: 70 },
      ],
      gear: 2,
      ability: [25, 0, 25],
    },
  },
  {
    id: 20019,
    name: "アカネ（バニーガール）",
    training: {
      level: null,
      rarity: 3,
      bond: 20,
      skills: [2, 3, 3, 6],
      weapon: null,
      equipment: [{ tier: 4, level: 40 }, null, { tier: 3, level: 30 }],
      gear: 0,
      ability: [0, 0, 0],
    },
  },
];

test("shows the students read from the student screen", async ({ page }) => {
  await open(page, { status: reading("students"), items, equipment, students }, "生徒");
  await expect(page.getByRole("status")).toHaveText("生徒を集計中 27 fps");
  await expect(page.getByRole("tab", { name: "生徒" })).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("tbody tr").first().locator("td")).toHaveText([
    "",
    "ホシノ",
    "90",
    "固有4",
    "5MMM",
    "T10/10/10",
    "T2",
    "25/0/25",
    "34",
  ]);
  await expect(page.locator("tbody tr").nth(1).locator("td")).toHaveText([
    "",
    "アカネ（バニーガール）",
    "?",
    "★3",
    "2336",
    "T4/?/3",
    "—",
    "0/0/0",
    "20",
  ]);
  // The star counts on into the unique weapon's, its level on hover.
  await expect(page.locator("tbody tr").first().locator("td").nth(3)).toHaveAttribute(
    "title",
    "固有武器 Lv60",
  );
  await page.getByLabel("名前で検索").fill("ｱｶﾈ");
  await expect(page.locator("tbody tr")).toHaveCount(1);
});

test("keeps a narrow window from scrolling sideways with the student table", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 640 });
  await open(page, { status: reading("students"), items, equipment, students }, "生徒");
  expect(await overflow(page)).toBe(0);
  await page.screenshot({ path: "test-results/narrow-students.png" });
});

const written = String.raw`C:\Users\me\Documents\Aoi\aoi-20261012-153000.yaml`;

test("writes everything read to a file and tells where", async ({ page }) => {
  await open(
    page,
    { status: reading("students"), items, equipment, students, export: written },
    "生徒",
  );
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "YAMLを書き出す" }).click();
  await expect(page.getByText(`書き出しました: ${written}`)).toBeVisible();
});

test("tells why an export failed", async ({ page }) => {
  await open(page, {
    status: reading("items"),
    items,
    equipment,
    export: { error: "アクセスが拒否されました。" },
  });
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "YAMLを書き出す" }).click();
  await expect(page.getByRole("alert")).toHaveText(
    "書き出せませんでした: アクセスが拒否されました。",
  );
});

test("has nothing to export before anything is read", async ({ page }) => {
  await open(page, { status: { state: "no_game" }, items: [], equipment: [], students: [] });
  await page.getByRole("button", { name: "メニュー" }).click();
  await expect(page.getByRole("menuitem", { name: "YAMLを書き出す" })).toBeDisabled();
});

test("keeps the search field in sight down to the last tile", async ({ page }) => {
  await page.setViewportSize({ width: 640, height: 480 });
  await open(page, { status: reading("items"), items, equipment });
  await page.locator(".tiles li").last().scrollIntoViewIfNeeded();
  await expect(page.locator(".tiles li").last()).toBeInViewport();
  await expect(page.getByLabel("名前で検索")).toBeInViewport();
});

test("keeps the student headings over their columns when scrolled sideways", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 480 });
  await open(page, { status: reading("students"), items, equipment, students }, "生徒");
  const last = page.locator("tbody tr").first().locator("td").last();
  await last.scrollIntoViewIfNeeded();
  const cell = await last.boundingBox();
  const heading = await page.locator("thead th").last().boundingBox();
  expect(Math.abs(heading.x - cell.x)).toBeLessThan(1);
  await expect(page.locator("thead th").last()).toBeInViewport();
});

/** A student with a 5MMM T10 training and `overrides`. */
const trained = (id, name, overrides) => ({
  id,
  name,
  training: {
    level: 90,
    rarity: 5,
    bond: 20,
    skills: [5, 10, 10, 10],
    weapon: { level: 60, stars: 4 },
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

test("orders students by level, star and bond, or by the column chosen", async ({ page }) => {
  const roster = [
    trained(1, "アル", { level: 80, bond: 50 }),
    trained(2, "ミカ", { bond: 43, weapon: { level: 30, stars: 1 } }),
    trained(3, "ヒナ", { rarity: 4, bond: 50 }),
    trained(4, "ホシノ", { bond: 34 }),
  ];
  await open(page, { status: reading("students"), items, equipment, students: roster }, "生徒");
  const names = page.locator("tbody td:nth-child(2)");
  await expect(names).toHaveText(["ホシノ", "ミカ", "ヒナ", "アル"]);
  await expect(page.locator("th", { hasText: "Lv" })).toHaveAttribute("aria-sort", "descending");

  await page.getByRole("button", { name: "星" }).click();
  await expect(names).toHaveText(["ホシノ", "アル", "ミカ", "ヒナ"]);
  await expect(page.locator("th", { hasText: "星" })).toHaveAttribute("aria-sort", "descending");
  await expect(page.locator("th", { hasText: "Lv" })).not.toHaveAttribute(
    "aria-sort",
    "descending",
  );

  await page.getByRole("button", { name: "絆" }).focus();
  await page.keyboard.press("Enter");
  await expect(names).toHaveText(["ヒナ", "アル", "ミカ", "ホシノ"]);
});

test("counts the students read on their tab", async ({ page }) => {
  const tab = page.getByRole("tab", { name: "生徒" });
  await open(page, { status: { state: "no_game" }, items, equipment, students: [] });
  await expect(tab).toHaveText("生徒");

  await open(page, { status: reading("students"), items, equipment, students }, "生徒");
  await expect(tab).toHaveText("生徒 2");

  const roster = Array.from({ length: 42 }, (_, i) => trained(i + 1, `生徒${i + 1}`, {}));
  await open(page, { status: reading("students"), items, equipment, students: roster }, "生徒");
  await expect(tab).toHaveText("生徒 42");
  await expect(page.getByRole("tab", { name: "生徒 42" })).toHaveAttribute("aria-selected", "true");
});

test("tells why what was read could not be kept", async ({ page }) => {
  await open(page, {
    status: reading("items"),
    items,
    equipment,
    keep_error: "読み取り結果を保存できませんでした: アクセスが拒否されました。",
  });
  await expect(page.getByRole("alert")).toHaveText(
    "読み取り結果を保存できませんでした: アクセスが拒否されました。",
  );
});

const shops = [
  {
    name: "総力戦",
    read_at: "2026-09-30 22:00:00",
    resets: true,
    // The plain coin first, the rare under it.
    coins: [
      { name: "総力戦コイン", icon: "item_icon_raidcoin", held: { exact: 5940 } },
      { name: "総力戦レアコイン", icon: "item_icon_raidcoin_high", held: { exact: 530 } },
    ],
    goods: [
      {
        id: 3000,
        name: "初級戦術教育BD（百鬼夜行）",
        icon: "item_icon_material_exskill_hyakkiyako_0",
        offers: [
          { left: 40, price: { price: 10, amount: 1, limit: 40 } },
          { left: 12, price: { price: 10, amount: 1, limit: 12 } },
        ],
        sold: false,
      },
      {
        id: 10019,
        name: "アズサの神名文字",
        icon: "item_icon_secretstone_azusa",
        offers: [null, { left: 0, price: { price: 50, amount: 5, limit: 10 } }],
        sold: true,
      },
    ],
  },
  {
    name: "指名手配",
    read_at: "2999-01-01 08:30:00",
    resets: true,
    coins: [{ name: "指名手配コイン", icon: "item_icon_chasercoin", held: null }],
    goods: [
      {
        id: 160,
        name: "エーテルの粉",
        icon: "item_icon_material_ether_0",
        offers: [{ left: 25, price: null }],
        sold: false,
      },
    ],
  },
];

test("shows each shop's goods, shop by shop, when the game shows a shop", async ({ page }) => {
  await open(page, { status: reading("shops"), items, equipment, shops }, "ショップ");
  await expect(page.getByRole("status")).toHaveText("ショップを集計中 27 fps");
  await expect(page.getByRole("tab", { name: "ショップ" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  // Each shop names itself, when it was read and the coins held.
  await expect(page.locator("summary")).toHaveText([
    "総力戦09/30 22:00前の月の値 5,940530",
    "指名手配01/01 08:30 ?",
  ]);
  expect(
    await page
      .locator("summary")
      .first()
      .locator("img")
      .evaluateAll((imgs) => imgs.map((i) => i.alt)),
  ).toEqual(["総力戦コイン", "総力戦レアコイン"]);
  // Each shop opens and closes; closed, its goods are out of sight.
  const first = page.locator("details").first();
  const tiles = first.locator(".tiles li");
  await expect(tiles.first()).toBeHidden();
  await page.locator("summary").first().click();
  // A good by its icon and how many come, then each coin's times left at its
  // price, the plain coin first; a good bought up is sold out.
  await expect(tiles).toHaveText(["×1 40回@1012回@10", "×5 売切れ"]);
  await expect(tiles.first()).toHaveAttribute("title", "初級戦術教育BD（百鬼夜行）");
  expect(
    await tiles
      .first()
      .locator(".offer img")
      .evaluateAll((imgs) => imgs.map((i) => i.alt)),
  ).toEqual(["総力戦コイン", "総力戦レアコイン"]);
  await expect(tiles.nth(1).locator(".offer").first()).toHaveText("");
  // A good bought up with every coin is greyed out; the app lists it last.
  await expect(tiles.nth(1)).toHaveClass(/sold/);
  await expect(tiles.first()).not.toHaveClass(/sold/);
  expect(await tiles.nth(1).evaluate((t) => Number(getComputedStyle(t).opacity))).toBeLessThan(1);
  // The icons load from the app.
  await expect
    .poll(() =>
      first
        .locator("img")
        .evaluateAll((imgs) => imgs.every((i) => i.complete && i.naturalWidth > 0)),
    )
    .toBe(true);

  await page.getByLabel("名前で検索").fill("エーテル");
  await expect(page.locator("summary")).toHaveText(["指名手配01/01 08:30 ?"]);
  await expect(page.locator("details .tiles li")).toHaveText(["25回@?"]);
  expect(await overflow(page)).toBeLessThanOrEqual(0);
});

test("a shop opened stays open while the game is read", async ({ page }) => {
  const snapshot = { status: reading("shops"), items, equipment, shops };
  await open(page, snapshot, "ショップ");
  await page.locator("summary").first().click();
  await expect(page.locator("details").first()).toHaveAttribute("open", "");
  // Every frame read sends a new snapshot; what the user opened stays open.
  await page.evaluate(
    (s) => window.__AOI_PUSH__({ ...s, status: { ...s.status, fps: 9 } }),
    snapshot,
  );
  await page.evaluate(
    (s) => window.__AOI_PUSH__({ ...s, status: { ...s.status, fps: 8 } }),
    snapshot,
  );
  await expect(page.locator("details").first()).toHaveAttribute("open", "");
  await expect(page.locator("details").first().locator(".tiles li").first()).toBeVisible();
});

test("guides to the shop before any is read", async ({ page }) => {
  await open(page, { status: { state: "other_screen" }, items, equipment, shops: [] });
  await page.getByRole("tab", { name: "ショップ" }).click();
  await expect(page.locator("[data-state=empty]")).toContainText("ショップで総力戦");
});

test("marks a student for training from the student tab", async ({ page }) => {
  await open(
    page,
    { status: reading("students"), items, equipment, students, targets: [] },
    "生徒",
  );
  await page.getByLabel("ホシノを育成対象にする").check();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([{ id: 10005, target: {} }]);
});

const target = { level: 90, skills: [5, 10, 10, 10], equipment: [10, 10, 10], star: null };

/** A line of the plan for an item of the kinds that come in four tiers. */
const tiered = (id, name, family, tier, need, have) => ({
  kind: "item",
  id,
  name,
  icon: `item_icon_skillbook_gehenna_${tier - 1}`,
  family: [family, tier],
  need,
  have,
});

test("shows the targets and what they take against what is held", async ({ page }) => {
  await open(page, {
    status: { state: "other_screen" },
    items,
    equipment,
    students,
    targets: [{ id: 20019, target }],
    plan: [
      { kind: "credits", id: 0, name: "", need: 1500000, have: { exact: 244654006 } },
      { kind: "report_exp", id: 0, name: "", need: 60000, have: { about: 51000 } },
      tiered(4030, "初級技術ノート（ゲヘナ）", 4030, 1, 30, { exact: 12 }),
      tiered(4033, "最上級技術ノート（ゲヘナ）", 4030, 4, 3, { about: 51000 }),
      { kind: "item", id: 9999, name: "秘伝ノート", icon: "", family: null, need: 3, have: null },
    ],
    plan_unknown: [
      { id: 20019, name: "アカネ（バニーガール）", part: "level" },
      { id: 20019, name: "アカネ（バニーガール）", part: { equipment: 1 } },
    ],
    shorts: [
      {
        kind: "report_exp",
        id: 0,
        name: "",
        icon: "",
        short: 9000,
        boxes: [],
        left: 9000,
      },
      {
        kind: "item",
        id: 4030,
        name: "初級技術ノート（ゲヘナ）",
        icon: "item_icon_skillbook_gehenna_0",
        short: 18,
        boxes: [
          { name: "初級技術ノート選択ボックス", icon: "item_icon_skillbook_gehenna_0", count: 18 },
        ],
        left: 0,
      },
    ],
  });
  await page.getByRole("tab", { name: "育成" }).click();
  const row = page.locator("table.targets tbody tr");
  await expect(row.locator("td").first()).toHaveText("アカネ（バニーガール）");
  await expect(row.locator(".now")).toHaveText([
    "?→",
    "★3→",
    "2→",
    "3→",
    "3→",
    "6→",
    "4→",
    "?→",
    "3→",
  ]);
  await expect(page.getByText("アカネ（バニーガール）: Lv・装備2 は数えられません")).toBeVisible();
  await expect(page.locator("table.plan tbody tr")).toHaveText([
    "クレジット1.5M244.7M足りる",
    "レポート（経験値）60,000約51,000不足 約9,000",
    "秘伝ノート3??",
  ]);
  // A kind's four tiers on one line, by icon: what is needed over what is held.
  const tiers = page.locator("table.tiers tbody tr td");
  await expect(tiers).toHaveText(["30 / 12", "", "", "3 / 約51,000"]);
  await expect(tiers.first().locator(".short")).toHaveCount(1);
  await expect(tiers.last().locator(".enough")).toHaveCount(1);
  await expect(page.getByRole("img", { name: "最上級技術ノート（ゲヘナ）" })).toHaveAttribute(
    "src",
    "/icon/item_icon_skillbook_gehenna_3",
  );

  // What falls short, the choice boxes held to open for it, and what is left.
  await expect(page.locator("table.shorts tbody tr")).toHaveText([
    "レポート（経験値）9,000—9,000",
    "初級技術ノート（ゲヘナ）18×18補える",
  ]);
  await expect(page.locator("table.shorts .opened span")).toHaveAttribute(
    "title",
    "初級技術ノート選択ボックス",
  );
  await expect(page.locator("table.shorts tbody tr").first().locator(".short")).toHaveCount(1);
  await expect(page.locator("table.shorts tbody tr").last().locator(".enough")).toHaveCount(1);

  // A target changes from the keyboard, within its range.
  const ex = page.getByLabel("アカネ（バニーガール）のEXの目標");
  await ex.focus();
  await page.keyboard.press("ArrowDown");
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([
    { id: 20019, target: { ...target, skills: [4, 10, 10, 10] } },
  ]);
});

test("a target being typed is kept while the game is read", async ({ page }) => {
  const snapshot = {
    status: reading("items"),
    items,
    equipment,
    students,
    targets: [{ id: 20019, target }],
    plan: [],
    plan_unknown: [],
  };
  await open(page, snapshot);
  await page.getByRole("tab", { name: "育成" }).click();
  const level = page.getByLabel("アカネ（バニーガール）のLvの目標");
  await level.fill("8");
  await page.evaluate(
    (s) => window.__AOI_PUSH__({ ...s, status: { ...s.status, fps: 9 } }),
    snapshot,
  );
  await expect(level).toHaveValue("8");
  await level.press("Enter");
  await level.blur();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([
    { id: 20019, target: { ...target, level: 8 } },
  ]);
});

test("a star is chosen from the student's on, and kept while the game is read", async ({
  page,
}) => {
  const snapshot = {
    status: reading("items"),
    items,
    equipment,
    students,
    targets: [{ id: 20019, target }],
    plan: [],
    plan_unknown: [],
  };
  await open(page, snapshot);
  await page.getByRole("tab", { name: "育成" }).click();
  const star = page.getByLabel("アカネ（バニーガール）の星の目標");
  // Never chosen, the target stays at the student's star; none below it.
  await expect(star).toHaveValue("3");
  await expect(star.locator("option:disabled")).toHaveText(["★1", "★2"]);
  await star.selectOption({ label: "固有2" });
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([
    { id: 20019, target: { ...target, star: 6 } },
  ]);
  await page.evaluate(
    (s) =>
      window.__AOI_PUSH__({
        ...s,
        targets: [{ id: 20019, target: { ...s.targets[0].target, star: 6 } }],
      }),
    snapshot,
  );
  await page.evaluate(
    (s) =>
      window.__AOI_PUSH__({
        ...s,
        status: { ...s.status, fps: 9 },
        targets: [{ id: 20019, target: { ...s.targets[0].target, star: 6 } }],
      }),
    snapshot,
  );
  await expect(star).toHaveValue("6");
});

test("guides to the student tab before any student is marked", async ({ page }) => {
  await open(page, { status: { state: "other_screen" }, items, equipment, students, targets: [] });
  await page.getByRole("tab", { name: "育成" }).click();
  await expect(page.locator("[data-state=empty]")).toContainText("生徒タブで「育成」");
});

const work = {
  slots: [
    {
      name: "帽子",
      icon: "equipment_icon_hat_tier10",
      golden: 3,
      top: { held: 150, about: false, need: 60, pieces: 2.5 },
      universal: { held: 2150, about: true, need: 430, pieces: 5.0 },
    },
  ],
  farm: {
    multiplier: 2,
    runs: 71,
    ap: 710,
    stages: [
      { name: "29-1", runs: 60, ap: 600, slots: ["お守り", "時計"] },
      { name: "25-1", runs: 11, ap: 110, slots: ["帽子"] },
    ],
  },
  bounties: [
    {
      name: "指名手配：ハイウェイ",
      kinds: [
        [
          {
            id: 110,
            name: "ファイストス円盤の欠片",
            icon: "item_icon_material_phaistos_0",
            held: { exact: 51 },
            need: 75,
          },
          {
            id: 111,
            name: "摩耗したファイストス円盤",
            icon: "item_icon_material_phaistos_1",
            held: { exact: 100 },
            need: 75,
          },
          {
            id: 112,
            name: "破損したファイストス円盤",
            icon: "item_icon_material_phaistos_2",
            held: { exact: 200 },
            need: 75,
          },
          null,
        ],
      ],
    },
  ],
  scrimmages: [
    {
      name: "学園交流会 - トリニティ",
      kinds: [
        [
          {
            id: 10,
            name: "錆びたスプリング",
            icon: "equipment_icon_weaponexpgrowtha_0",
            held: null,
          },
          null,
          null,
          null,
        ],
      ],
    },
  ],
  commissions: {
    credits_name: "クレジット回収",
    credits: 248955686,
    credits_each: 41500000,
    credit_students: 5.999,
    reports_name: "拠点防衛",
    report_exp: 6245925,
    report_about: false,
    exp_each: 1249185,
    report_students: 5.0,
  },
};

test("tells what the daily plays give against what is held", async ({ page }) => {
  await open(page, { status: { state: "other_screen" }, items, equipment, work });
  await page.getByRole("tab", { name: "お仕事" }).click();
  // Each section folds as a shop does, closed until opened.
  const sections = page.locator(".work summary");
  await expect(sections).toHaveText(["任務", "指名手配", "学園交流会", "特別依頼"]);
  await expect(page.locator(".slots")).toBeHidden();
  for (const section of await sections.all()) await section.click();
  // A slot: how many pieces its T10 blueprints and its universal ones make,
  // short of its golden ratio orange.
  await expect(page.locator(".slots tbody tr")).toHaveText([
    "帽子黄金比 3人2.5個分 150 / 605.0個分 約2,150 / 430",
  ]);
  await expect(page.locator(".slots strong.low")).toHaveText(["2.5個分"]);
  // How many times to play each mission, in the campaign, to the golden ratio.
  // The totals under a double rule.
  await expect(page.locator(".farm tr")).toHaveText([
    "29-160回600APお守り・時計",
    "25-111回110AP帽子",
    "計71回710AP",
  ]);
  await expect(page.locator(".farm tfoot th")).toHaveCSS("border-top-style", "double");
  // Every cell of a mission's row fills the row, the slots it serves too.
  for (const row of await page.locator(".farm tbody tr").all()) {
    const boxes = await row.locator("td").evaluateAll((cells) =>
      cells.map((c) => {
        const box = c.getBoundingClientRect();
        return `${box.top}-${box.bottom}`;
      }),
    );
    expect(new Set(boxes).size).toBe(1);
  }
  // Each place's drops by tier with what is held, "—" where none is counted.
  await expect(page.locator(".work h3")).toHaveText([
    "黄金比までの周回（2倍キャンペーン中）",
    "ハイウェイ",
    "トリニティ",
  ]);
  await expect(page.locator(".work .tiers td.tier-1")).toHaveText(["51", "—"]);
  // Short of one student red, of two blue, enough for two green.
  await expect(page.locator(".work .tiers .short")).toHaveText(["51"]);
  await expect(page.locator(".work .tiers .fair")).toHaveText(["100"]);
  await expect(page.locator(".work .tiers .enough")).toHaveText(["200"]);
  const colours = await page
    .locator(".work .tiers :is(.short, .fair, .enough)")
    .evaluateAll((cells) => cells.map((c) => getComputedStyle(c).color));
  expect(new Set(colours).size).toBe(3);
  await expect(page.getByRole("img", { name: "ファイストス円盤の欠片" })).toBeVisible();
  // The commission raising more students is green, the fewer orange.
  await expect(page.locator(".commissions tr")).toHaveText([
    "クレジット回収6.0人分249M ÷ 41.5M",
    "拠点防衛5.0人分6.2M EXP ÷ 1.2M EXP",
  ]);
  await expect(page.locator(".commissions tr.more")).toContainText("クレジット回収");
  await expect(page.locator(".commissions tr.fewer")).toContainText("拠点防衛");
});

test("tells what to buy shop by shop for what is still short", async ({ page }) => {
  const coin = (name, icon, held, spent) => ({ name, icon, held, spent });
  const snapshot = {
    status: { state: "other_screen" },
    items,
    equipment,
    credits: 249330855,
    shorts: [{ kind: "item", id: 4033, name: "最上級技術ノート（ゲヘナ）", left: 29 }],
    buy: {
      shops: [
        {
          name: "総力戦",
          coins: [
            coin("総力戦コイン", "item_icon_raidcoin", 5940, 500),
            coin("総力戦レアコイン", "item_icon_raidcoin_high", 530, 150),
          ],
          goods: [
            {
              name: "最上級技術ノート（ゲヘナ）",
              icon: "item_icon_skillbook_gehenna_3",
              coin: 0,
              times: 20,
              cost: 500,
              gives: 20,
              kind: "item",
            },
            {
              name: "最上級技術ノート（ゲヘナ）",
              icon: "item_icon_skillbook_gehenna_3",
              coin: 1,
              times: 6,
              cost: 150,
              gives: 6,
              kind: "item",
            },
          ],
        },
      ],
      left: [{ kind: "item", name: "最上級技術ノート（ゲヘナ）", icon: "", left: 3 }],
      unread: ["大決戦"],
    },
  };
  await open(page, snapshot, "購買戦略");
  await expect(page.getByText("クレジット 249,330,855")).toBeVisible();
  // A shop with its coins held and left after, then the goods to buy.
  await expect(page.locator(".buy h2").first()).toHaveText("総力戦5,940 → 5,440530 → 380");
  await expect(page.locator(".buy table").first().locator("tr")).toHaveText([
    "最上級技術ノート（ゲヘナ）20回500×20",
    "最上級技術ノート（ゲヘナ）6回150×6",
  ]);
  expect(
    await page
      .locator(".buy table")
      .first()
      .locator("td:nth-child(3) img")
      .evaluateAll((imgs) => imgs.map((i) => i.alt)),
  ).toEqual(["総力戦コイン", "総力戦レアコイン"]);
  await expect(page.locator(".buy table").last().locator("tr")).toHaveText([
    "最上級技術ノート（ゲヘナ）3",
  ]);
  await expect(page.getByText("今月まだ読んでいないショップ: 大決戦")).toBeVisible();
  expect(await overflow(page)).toBeLessThanOrEqual(0);

  // Nothing is short that what is held does not make up: a guide instead.
  await page.evaluate(
    (s) => window.__AOI_PUSH__({ ...s, shorts: [{ ...s.shorts[0], left: 0 }] }),
    snapshot,
  );
  await expect(page.locator(".buy [data-state=empty]")).toBeVisible();
});

test("keeps the tab chosen when the game shows another list", async ({ page }) => {
  const snapshot = { status: reading("items"), items, equipment, students };
  await open(page, snapshot, "生徒");
  await page.evaluate(
    (s) => window.__AOI_PUSH__({ ...s, status: { ...s.status, list: "equipment" } }),
    snapshot,
  );
  await expect(page.getByRole("status")).toHaveText("装備を集計中 27 fps");
  await expect(page.getByRole("tab", { name: "生徒" })).toHaveAttribute("aria-selected", "true");
});

test("the menu chooses the theme, kept for the next start", async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark" });
  await open(page, { status: { state: "no_game" }, items, equipment });
  const body = page.locator("body");
  await expect(body).toHaveCSS("background-color", "rgb(25, 25, 25)");
  const choose = async (name) => {
    await page.getByRole("button", { name: "メニュー" }).click();
    await page.getByRole("menuitemradio", { name }).click();
  };
  await choose("ライト");
  await expect(body).toHaveCSS("background-color", "rgb(250, 246, 239)");
  await expect(page.getByRole("menu")).toHaveCount(0);
  await page.reload();
  await expect(body).toHaveCSS("background-color", "rgb(250, 246, 239)");
  await page.getByRole("button", { name: "メニュー" }).click();
  await expect(page.getByRole("menuitemradio", { name: "ライト" })).toHaveAttribute(
    "aria-checked",
    "true",
  );
  await page.keyboard.press("Escape");
  await expect(page.getByRole("menu")).toHaveCount(0);
  // 自動 follows the OS again.
  await choose("自動");
  await expect(body).toHaveCSS("background-color", "rgb(25, 25, 25)");
});

test("the menu checks for an update and takes it", async ({ page }) => {
  await open(page, {
    status: { state: "no_game" },
    items,
    equipment,
    update: { current: "v0.1.29", latest: "v0.1.30" },
  });
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "アップデートを確認" }).click();
  await expect(page.getByText("v0.1.30 があります（今 v0.1.29）")).toBeVisible();
  await page.getByRole("button", { name: "更新して再起動" }).click();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([{ update: true }]);
  await expect(page.getByText("v0.1.30 に更新しています…")).toBeVisible();
});

test("tells the app is up to date, or why it could not check", async ({ page }) => {
  await open(page, {
    status: { state: "no_game" },
    items,
    equipment,
    update: { current: "v0.1.29", latest: null },
  });
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "アップデートを確認" }).click();
  await expect(page.getByText("最新の版です（v0.1.29）")).toBeVisible();
  await expect(page.getByRole("button", { name: "更新して再起動" })).toHaveCount(0);

  await open(page, {
    status: { state: "no_game" },
    items,
    equipment,
    update: { error: "通信できません" },
  });
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "アップデートを確認" }).click();
  await expect(page.getByRole("alert")).toHaveText("アップデートできませんでした: 通信できません");
});
