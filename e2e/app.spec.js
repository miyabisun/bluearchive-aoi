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
  await expect(page.getByLabel("名前で検索")).toBeFocused();
  await page.screenshot({ path: "test-results/wide-dark.png" });
});
