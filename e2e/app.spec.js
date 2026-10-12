import { expect, test } from "@playwright/test";

const items = Array.from({ length: 120 }, (_, i) => ({
  id: 1000 + i,
  name:
    i === 3
      ? "とても長い名前の期間限定でない普通のアイテム（ゲヘナ・トリニティ・ミレニアム共通）"
      : `アイテム${i}`,
  tier: null,
  count: i === 5 ? { thousands: 51 } : { exact: i * 997 },
}));

const equipment = [
  { id: 1, name: "初級強化珠", tier: null, count: { exact: 1383 } },
  { id: 1000, name: "無地のキャップ", tier: 1, count: { exact: 249 } },
  { id: 101009, name: "ゲーミングヘルメットの設計図", tier: 10, count: { exact: 17 } },
  { id: 501000, name: "帽子の万能設計図", tier: null, count: { thousands: 51 } },
];

const reading = (list, fps = 27.4) => ({ state: "reading", list, fps, dropped: 0 });

/** Opens the page showing `snapshot` as the watcher's. */
async function open(page, snapshot) {
  await page.addInitScript((demo) => (window.__AOI_DEMO__ = demo), snapshot);
  await page.goto("/");
}

const overflow = (page) =>
  page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);

test("lists the item tally in order and narrows it by name", async ({ page }) => {
  await open(page, { status: reading("items"), items, equipment });
  await expect(page.getByRole("status")).toHaveText("アイテムを集計中 27 fps");
  await expect(page.getByRole("tab", { name: "アイテム" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await expect(page.locator("tbody tr")).toHaveCount(120);
  await expect(page.locator("tbody tr").nth(2)).toHaveText("アイテム21,994");
  await expect(page.locator("tbody tr").nth(5).locator("td.count")).toHaveText("51K");

  await page.getByLabel("名前で検索").fill("ｱｲﾃﾑ11");
  await expect(page.locator("tbody td:first-child")).toHaveText([
    "アイテム11",
    ...Array.from({ length: 10 }, (_, i) => `アイテム${110 + i}`),
  ]);
  await page.getByLabel("名前で検索").fill("ない名前");
  await expect(page.getByText("一致する項目がありません")).toBeVisible();
});

test("shows the equipment tab with tiers when the game shows the equipment list", async ({
  page,
}) => {
  await open(page, { status: reading("equipment"), items, equipment });
  await expect(page.getByRole("status")).toHaveText("装備を集計中 27 fps");
  await expect(page.getByRole("tab", { name: "装備" })).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("thead th").first()).toHaveText("装備");
  await expect(page.locator("tbody tr")).toHaveText([
    "初級強化珠1,383",
    "T1無地のキャップ249",
    "T10ゲーミングヘルメットの設計図17",
    "帽子の万能設計図51K",
  ]);

  await page.getByRole("tab", { name: "アイテム" }).click();
  await expect(page.locator("tbody tr")).toHaveCount(120);
});

test("guides to the list before anything is read", async ({ page }) => {
  await open(page, { status: { state: "other_screen" }, items: [], equipment });
  await expect(page.getByRole("status")).toHaveText("対象外の画面");
  await expect(page.locator("[data-state=empty]")).toContainText("アイテム一覧");
  await expect(page.getByLabel("名前で検索")).toHaveCount(0);
  await page.getByRole("tab", { name: "装備" }).click();
  await expect(page.locator("tbody tr")).toHaveCount(4);
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
  await expect(page.getByRole("tab", { name: "生徒" })).toBeFocused();
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

/** Whether nothing covers the middle of the first column heading. */
const headingShows = (page) =>
  page
    .locator("thead th")
    .first()
    .evaluate((th) => {
      const r = th.getBoundingClientRect();
      return th.contains(document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2));
    });

test("keeps the column headings in sight down to the last row", async ({ page }) => {
  await page.setViewportSize({ width: 640, height: 480 });
  await open(page, { status: reading("items"), items, equipment });
  await page.locator("tbody tr").last().scrollIntoViewIfNeeded();
  await expect(page.locator("tbody tr").last()).toBeInViewport();
  await expect(page.locator("thead th").first()).toBeInViewport();
  expect(await headingShows(page)).toBe(true);
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
  const names = page.locator("tbody td:first-child");
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
