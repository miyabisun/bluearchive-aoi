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

test("puts the tabs, the status and the menu on one row, without the app's name", async ({
  page,
}) => {
  await page.setViewportSize({ width: 720, height: 600 });
  await open(page, { status: reading("shops"), items, equipment, students });
  // The title bar of the window already says Aoi.
  await expect(page.getByText("Aoi", { exact: true })).toBeHidden();
  const middle = async (locator) => {
    const box = await locator.boundingBox();
    return box.y + box.height / 2;
  };
  const row = await middle(page.getByRole("tab", { name: "アイテム" }));
  for (const other of [
    page.getByRole("tab", { name: "お仕事" }),
    page.getByRole("button", { name: /ショップを集計中/ }),
    page.getByRole("button", { name: "メニュー" }),
  ]) {
    expect(Math.abs((await middle(other)) - row)).toBeLessThan(4);
  }
  expect(await overflow(page)).toBeLessThanOrEqual(0);
});

test("the tabs and the search field take focus from the keyboard", async ({ page }) => {
  await open(page, { status: reading("items", 20), items, equipment });
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
  await expect(page.getByRole("tab", { name: "お仕事" })).toBeFocused();
  // The status and the menu end the row.
  await page.keyboard.press("Tab");
  await expect(page.getByRole("button", { name: /集計中/ })).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(page.getByRole("button", { name: "メニュー" })).toBeFocused();
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
  // Each shop opens and closes as the work tab's sections do, its marker turned when open.
  await expect(page.locator("details.shop summary .chevron")).toHaveCount(2);
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

/** A line short with its sources: boxes held, a shop's coin, an unread coin. */
const procure = [
  {
    kind: "item",
    id: 4033,
    name: "最上級技術ノート（ゲヘナ）",
    icon: "item_icon_skillbook_gehenna_3",
    short: 29,
    sources: [
      {
        type: "box",
        id: 150003,
        name: "最上級技術ノート選択ボックス",
        icon: "item_icon_skillbook_selection_3",
        held: 4,
        choices: 12,
      },
      {
        type: "shop",
        shop: "総力戦",
        coin: "総力戦コイン",
        icon: "item_icon_raidcoin",
        held: 5940,
        price: 25,
        gives: 1,
        left: 20,
      },
      {
        type: "shop",
        shop: "指名手配",
        coin: "指名手配コイン",
        icon: "item_icon_chasercoin",
        held: null,
        price: 200,
        gives: 1,
        left: 4,
      },
    ],
  },
];

test("shows the targets and what they take against what is held", async ({ page }) => {
  const snapshot = {
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
    procure,
    plan_version: 1,
  };
  await open(page, snapshot);
  await page.getByRole("tab", { name: "育成" }).click();
  const row = page.locator("table.targets tbody tr");
  await expect(row.locator("td.name")).toHaveText("アカネ（バニーガール）");
  // The value read over each target, without an arrow.
  await expect(row.locator(".now")).toHaveText(["?", "★3", "2", "3", "3", "6", "4", "?", "3"]);
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

  // Nothing paid yet: no settlement.
  await expect(page.getByRole("heading", { name: "精算" })).toHaveCount(0);
  await expect(page.locator("table.settle")).toHaveCount(0);
  // The sections: the students, what they take, and how to make up what is short.
  await expect(page.locator(".section")).toHaveText(["既存の生徒一覧", "必要素材", "不足分の調達"]);
  await expect(page.locator("table.procure tr.line")).toHaveText([
    "最上級技術ノート（ゲヘナ）2929",
  ]);
  // Each source shows how much of its resource there is.
  const chips = page.locator("table.procure .source");
  await expect(chips).toHaveText(["4", "5,940", "?"]);
  await expect(chips.nth(1)).toHaveAttribute(
    "title",
    "総力戦: 総力戦コイン 25 で ×1、今月あと 20 回",
  );

  // A click pays one; a right click takes one back; shift pays all it can.
  await chips.first().click();
  await chips.first().click();
  await expect(chips.first()).toHaveText("2+2");
  await chips.first().click({ button: "right" });
  await expect(chips.first()).toHaveText("3+1");
  await chips.nth(1).click({ modifiers: ["Shift"] });
  // The shop sells 20 more this month: 21 of 29 made up, and it says it is bought up.
  await expect(chips.nth(1)).toHaveText("売切れ+20");
  await expect(chips.nth(1)).toHaveClass(/out/);
  await expect(page.locator("table.procure tr.line")).toHaveText(["最上級技術ノート（ゲヘナ）298"]);
  // Boxes past those held: the rest can be tailor-made.
  await chips.first().click({ modifiers: ["Shift"] });
  await expect(chips.first()).toHaveText("-5+9");
  await expect(page.locator("table.procure tr.line .enough")).toHaveText("0");
  // What it all comes to.
  // What it all comes to: held, spent, and what is left, red below none, green above.
  await expect(page.locator("table.settle thead th")).toHaveText([
    "使ったもの",
    "所持",
    "消費",
    "残り",
  ]);
  await expect(page.locator("table.settle tbody tr")).toHaveText([
    "最上級技術ノート選択ボックス4-9-5",
    "総力戦コイン5,940-5005,440",
  ]);
  const settleRows = page.locator("table.settle tbody tr");
  await expect(settleRows.first().locator("td").last()).toHaveClass(/short/);
  await expect(settleRows.last().locator("td").last()).toHaveClass(/enough/);
  await expect(settleRows.last().locator("td").nth(2)).toHaveClass(/short/);

  // A new reading of the game keeps the payments while the plan is the same.
  await page.evaluate(
    (s) => window.__AOI_PUSH__({ ...s, status: { state: "reading", list: "items", fps: 9 } }),
    snapshot,
  );
  await expect(chips.first()).toHaveText("-5+9");
  // Held down, a chip pays again and again until let go.
  await chips.nth(1).click({ button: "right", modifiers: ["Shift"] });
  await expect(chips.nth(1)).toHaveText("5,940");
  const box = await chips.nth(1).boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.waitForTimeout(1000);
  await page.mouse.up();
  const held = Number((await chips.nth(1).locator(".given").innerText()).slice(1));
  expect(held).toBeGreaterThan(3);
  await page.waitForTimeout(300);
  await expect(chips.nth(1).locator(".given")).toHaveText(`+${held}`);

  // Shift with a right click takes them all back; リセット, every payment.
  await chips.first().click({ button: "right", modifiers: ["Shift"] });
  await expect(chips.first()).toHaveText("4");
  await page.getByRole("button", { name: "リセット" }).click();
  await expect(chips.nth(1)).toHaveText("5,940");
  await expect(page.locator("table.settle")).toHaveCount(0);

  // 再読込 counts the plan again from what is read now; the payments start over.
  await chips.first().click();
  await page.getByRole("button", { name: "再読込" }).click();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([{ reload: true }]);
  await page.evaluate((s) => window.__AOI_PUSH__({ ...s, plan_version: 2 }), snapshot);
  await expect(chips.first()).toHaveText("4");
  await page.evaluate(() => (window.__AOI_CALLS__ = []));

  // A target changes from the keyboard, within its range.
  const ex = page.getByLabel("アカネ（バニーガール）のEXの目標");
  await ex.focus();
  await page.keyboard.press("ArrowDown");
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([
    { id: 20019, target: { ...target, skills: [4, 10, 10, 10] } },
  ]);

  // The cross before a student takes them off training.
  await page.evaluate(() => (window.__AOI_CALLS__ = []));
  await page.getByRole("button", { name: "アカネ（バニーガール）を育成対象から外す" }).click();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([{ id: 20019, target: null }]);
});

test("shows a slot's blueprints by tier, and pays T10 from a box or the universal ones", async ({
  page,
}) => {
  const tier = (tier, need, have) => ({
    tier,
    id: 100999 + tier,
    name: `T${tier}の設計図`,
    icon: `equipment_icon_hat_tier${tier}_piece`,
    need,
    have: { exact: have },
  });
  const universal = {
    tier: 0,
    id: 501000,
    name: "帽子の万能設計図",
    icon: "equipment_icon_hat_useall_piece",
    need: 10,
    have: { about: 51000 },
  };
  const tiers = [tier(2, 15, 10)];
  for (let t = 3; t <= 9; t++) tiers.push(tier(t, 0, 0));
  tiers.push(tier(10, 60, 17));
  const snapshot = {
    status: { state: "other_screen" },
    items,
    equipment,
    students,
    targets: [{ id: 20019, target }],
    plan: [],
    blueprints: [{ name: "帽子", icon: "equipment_icon_hat_tier1", tiers, universal, short: 43 }],
    plan_unknown: [],
    procure: [
      {
        kind: "equipment",
        id: 101009,
        name: "帽子のT10設計図",
        icon: "equipment_icon_hat_tier10_piece",
        short: 43,
        sources: [
          {
            type: "box",
            id: 150048,
            name: "T10装備設計図選択ボックス",
            icon: "item_icon_equipment_selection_10",
            held: 2,
            choices: 9,
          },
          {
            type: "universal",
            id: 501000,
            name: "帽子の万能設計図",
            icon: "equipment_icon_hat_useall_piece",
            held: 50990,
            rate: 50,
          },
        ],
      },
    ],
    plan_version: 1,
  };
  await open(page, snapshot);
  await page.getByRole("tab", { name: "育成" }).click();
  // The slot by its T1 piece, then T2 to T10: what is needed over what is held.
  const slot = page.locator("table.blueprints tbody");
  await expect(slot.getByRole("img", { name: "帽子", exact: true })).toBeVisible();
  await expect(slot.locator("th[scope=col]")).toHaveText([
    "T2",
    "T3",
    "T4",
    "T5",
    "T6",
    "T7",
    "T8",
    "T9",
    "T10",
  ]);
  const cells = slot.locator("tr").nth(1).locator("td");
  await expect(cells).toHaveText(["15 / 10", "", "", "", "", "", "", "", "60 / 17"]);
  // Below T10 the universal ones pay the rest: only T10 says it is short.
  await expect(cells.first()).not.toHaveClass(/short/);
  await expect(cells.last()).toHaveClass(/short/);
  // Under them, what T2 to T9 take of the universal ones over those held.
  await expect(slot.locator(".pay")).toHaveText("T2 - T9: 10 / 約51,000");
  await expect(slot.locator(".pay .enough")).toHaveCount(1);
  await expect(slot.getByRole("img", { name: "帽子の万能設計図" })).toBeVisible();
  // T10 asked in the procurement: from the T10 box, or 50 universal ones each.
  await expect(page.locator("table.procure tr.line")).toHaveText(["帽子のT10設計図4343"]);
  const chips = page.locator("table.procure .source");
  await expect(chips).toHaveText(["2", "50,990"]);
  await chips.last().click({ modifiers: ["Shift"] });
  await expect(chips.last()).toHaveText("48,840+43");
  await expect(page.locator("table.settle tbody tr")).toHaveText([
    "帽子の万能設計図50,990-2,15048,840",
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
    next: [
      { name: "29-2", slots: ["ヘアピン", "シューズ", "腕時計"] },
      { name: "29-5", slots: ["ネックレス", "バッグ", "ヘアピン"] },
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
  await expect(sections).toHaveText(["任務 Normal", "指名手配", "学園交流会", "特別依頼"]);
  await expect(page.locator(".work summary .chevron")).toHaveCount(4);
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
  await expect(page.locator(".farm:not(.next) tr")).toHaveText([
    "29-160回600APお守り・時計",
    "25-111回110AP帽子",
    "計71回710AP",
  ]);
  // Past the ratio, the missions that even it out most, best first.
  await expect(page.locator(".farm.next tr")).toHaveText([
    "29-2ヘアピン・シューズ・腕時計",
    "29-5ネックレス・バッグ・ヘアピン",
  ]);
  await expect(page.locator(".farm tfoot th")).toHaveCSS("border-top-style", "double");
  // Every cell of a mission's row fills the row, the slots it serves too.
  for (const row of await page.locator(".farm:not(.next) tbody tr").all()) {
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
    "その後に黄金比を整える周回",
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
    update: { current: "v0.1.29", latest: "v0.1.30", installs: true },
  });
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "アップデートを確認" }).click();
  await expect(page.getByText("v0.1.30 があります（今 v0.1.29）")).toBeVisible();
  await page.getByRole("button", { name: "更新して再起動" }).click();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([{ update: true }]);
  await expect(page.getByText("v0.1.30 に更新しています…")).toBeVisible();
});

test("where the app cannot replace itself, the update opens the release page", async ({ page }) => {
  await open(page, {
    status: { state: "no_source" },
    items,
    equipment,
    update: { current: "v0.1.29", latest: "v0.1.30", installs: false },
  });
  await expect(page.getByText("取り込み元なし")).toBeVisible();
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "アップデートを確認" }).click();
  await page.getByRole("button", { name: "入手ページを開く" }).click();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([{ update: true }]);
  // The page opens elsewhere: the notice stays, the button with it.
  await expect(page.getByText("v0.1.30 があります（今 v0.1.29）")).toBeVisible();
  await expect(page.getByRole("button", { name: "入手ページを開く" })).toBeVisible();
});

test("tells the app is up to date in a passing toast, or why it could not check", async ({
  page,
}) => {
  await page.clock.install();
  await open(page, {
    status: { state: "no_game" },
    items,
    equipment,
    update: { current: "v0.1.29", latest: null },
  });
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "アップデートを確認" }).click();
  // Up to date: a toast says so for a while, and nothing stays in the page.
  const toast = page.locator(".toast");
  await expect(toast).toHaveText("最新の版です（v0.1.29）");
  await expect(page.locator(".update")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "更新して再起動" })).toHaveCount(0);
  await page.clock.runFor(2500);
  await expect(toast).toBeVisible();
  await page.clock.runFor(1000);
  await expect(toast).toHaveCount(0);

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

test("the menu opens the link to a phone: a QR code to pair, then the phone connected or not", async ({
  page,
}) => {
  const qr = [
    [true, false],
    [false, true],
  ];
  await open(page, {
    status: { state: "no_source" },
    items,
    equipment,
    phone: { state: "pairing", qr },
  });
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "Androidとの通信" }).click();
  const dialog = page.getByRole("dialog", { name: "Androidとの通信" });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole("img", { name: "ペア設定の QR コード" })).toBeVisible();
  await expect(
    dialog.getByText("この QR コードを読み取るか、USB ケーブルでつないでください"),
  ).toBeVisible();
  // Already showing a QR code: none asked for anew.
  expect(await page.evaluate(() => window.__AOI_CALLS__ ?? [])).toEqual([]);
  await dialog.getByRole("button", { name: "閉じる" }).click();
  await expect(dialog).toBeHidden();

  await open(page, {
    status: { state: "no_source" },
    items,
    equipment,
    phone: { state: "unpaired" },
  });
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "Androidとの通信" }).click();
  // None paired: a QR code is asked for at once.
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([{ pair: true }]);

  const streaming = { state: "streaming", device: "Pixel 9", via: "wifi" };
  await open(page, { status: { state: "no_source" }, items, equipment, phone: streaming });
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "Androidとの通信" }).click();
  await expect(page.getByText("Pixel 9 の画面を Wi-Fi で受け取っています")).toBeVisible();
  await page.getByRole("button", { name: "切断" }).click();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([{ connect: false }]);

  await open(page, {
    status: { state: "no_source" },
    items,
    equipment,
    phone: { state: "stopped", device: "Pixel 9" },
  });
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "Androidとの通信" }).click();
  await page.getByRole("button", { name: "接続", exact: true }).click();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([{ connect: true }]);
});
