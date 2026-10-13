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

/** Opens the page showing `snapshot` as the watcher's, with the app's icons. */
async function open(page, snapshot) {
  await page.route("**/icon/*", (route) => {
    const name = new URL(route.request().url()).pathname.slice("/icon/".length);
    const file = ICONS.map((dir) => `${dir}/${name}.webp`).find(existsSync);
    return file ? route.fulfill({ path: file }) : route.fulfill({ status: 404 });
  });
  await page.addInitScript((demo) => (window.__AOI_DEMO__ = demo), snapshot);
  await page.goto("/");
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

test("shows the equipment tab when the game shows the equipment list", async ({ page }) => {
  await open(page, { status: reading("equipment"), items, equipment });
  await expect(page.getByRole("status")).toHaveText("装備を集計中 27 fps");
  await expect(page.getByRole("tab", { name: "装備" })).toHaveAttribute("aria-selected", "true");
  await expect(page.locator(".tiles li")).toHaveText(["1,383", "249", "17", "51K"]);
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
  await expect(page.getByRole("button", { name: "書き出す" })).toBeFocused();
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
  await open(page, { status: reading("students"), items, equipment, students });
  await expect(page.getByRole("status")).toHaveText("生徒を集計中 27 fps");
  await expect(page.getByRole("tab", { name: "生徒" })).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("tbody tr").first().locator("td")).toHaveText([
    "",
    "ホシノ",
    "90",
    "★5",
    "★4 Lv60",
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
    "—",
    "2336",
    "T4/?/3",
    "—",
    "0/0/0",
    "20",
  ]);
  await page.getByLabel("名前で検索").fill("ｱｶﾈ");
  await expect(page.locator("tbody tr")).toHaveCount(1);
});

test("keeps a narrow window from scrolling sideways with the student table", async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 640 });
  await open(page, { status: reading("students"), items, equipment, students });
  expect(await overflow(page)).toBe(0);
  await page.screenshot({ path: "test-results/narrow-students.png" });
});

const written = String.raw`C:\Users\me\Documents\Aoi\aoi-20261012-153000.yaml`;

test("writes everything read to a file and tells where", async ({ page }) => {
  await open(page, { status: reading("students"), items, equipment, students, export: written });
  await page.getByRole("button", { name: "書き出す" }).click();
  await expect(page.getByText(`書き出しました: ${written}`)).toBeVisible();
});

test("tells why an export failed", async ({ page }) => {
  await open(page, {
    status: reading("items"),
    items,
    equipment,
    export: { error: "アクセスが拒否されました。" },
  });
  await page.getByRole("button", { name: "書き出す" }).click();
  await expect(page.getByRole("alert")).toHaveText(
    "書き出せませんでした: アクセスが拒否されました。",
  );
});

test("has nothing to export before anything is read", async ({ page }) => {
  await open(page, { status: { state: "no_game" }, items: [], equipment: [], students: [] });
  await expect(page.getByRole("button", { name: "書き出す" })).toBeDisabled();
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
  await open(page, { status: reading("students"), items, equipment, students });
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

test("orders students by level, rarity and bond, or by the column chosen", async ({ page }) => {
  const roster = [
    trained(1, "アル", { level: 80, bond: 50 }),
    trained(2, "ミカ", { bond: 43, weapon: { level: 30, stars: 1 } }),
    trained(3, "ヒナ", { rarity: 4, bond: 50 }),
    trained(4, "ホシノ", { bond: 34 }),
  ];
  await open(page, { status: reading("students"), items, equipment, students: roster });
  const names = page.locator("tbody td:nth-child(2)");
  await expect(names).toHaveText(["ミカ", "ホシノ", "ヒナ", "アル"]);
  await expect(page.locator("th", { hasText: "Lv" })).toHaveAttribute("aria-sort", "descending");

  await page.getByRole("button", { name: "固有武器" }).click();
  await expect(names).toHaveText(["ホシノ", "ヒナ", "アル", "ミカ"]);
  await expect(page.locator("th", { hasText: "固有武器" })).toHaveAttribute(
    "aria-sort",
    "descending",
  );
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

  await open(page, { status: reading("students"), items, equipment, students });
  await expect(tab).toHaveText("生徒 2");

  const roster = Array.from({ length: 42 }, (_, i) => trained(i + 1, `生徒${i + 1}`, {}));
  await open(page, { status: reading("students"), items, equipment, students: roster });
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

test("shows the credits on the shop tab once they are read", async ({ page }) => {
  await open(page, { status: reading("items"), items, equipment, credits: 244213276 });
  await expect(page.getByText(/^クレジット/)).toHaveCount(0);
  await page.getByRole("tab", { name: "ショップ" }).click();
  await expect(page.getByText("クレジット 244,213,276")).toBeVisible();

  await open(page, { status: reading("items"), items, equipment, credits: null });
  await page.getByRole("tab", { name: "ショップ" }).click();
  await expect(page.getByText(/^クレジット/)).toHaveCount(0);
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
      },
      {
        id: 10019,
        name: "アズサの神名文字",
        icon: "item_icon_secretstone_azusa",
        offers: [null, { left: 0, price: { price: 50, amount: 5, limit: 10 } }],
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
      },
    ],
  },
];

test("shows each shop's goods, shop by shop, when the game shows a shop", async ({ page }) => {
  await open(page, { status: reading("shops"), items, equipment, shops });
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
  await open(page, snapshot);
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
  await open(page, { status: reading("students"), items, equipment, students, targets: [] });
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
  const star = page.getByLabel("アカネ（バニーガール）の★の目標");
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
