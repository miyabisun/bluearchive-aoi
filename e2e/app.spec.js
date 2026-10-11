import { expect, test } from "@playwright/test";

const rows = Array.from({ length: 120 }, (_, i) => ({
  id: 1000 + i,
  name:
    i === 3
      ? "とても長い名前の期間限定でない普通のアイテム（ゲヘナ・トリニティ・ミレニアム共通）"
      : `アイテム${i}`,
  count: i === 5 ? { thousands: 51 } : { exact: i * 997 },
}));

/** Opens the page showing `snapshot` as the watcher's. */
async function open(page, snapshot) {
  await page.addInitScript((demo) => (window.__AOI_DEMO__ = demo), snapshot);
  await page.goto("/");
}

const overflow = (page) =>
  page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);

test("lists the tally in order and narrows it by name", async ({ page }) => {
  await open(page, { status: { state: "reading", fps: 27.4, dropped: 0 }, rows });
  await expect(page.getByRole("status")).toHaveText("集計中 27 fps");
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

test("guides to the item list before anything is read", async ({ page }) => {
  await open(page, { status: { state: "other_screen" }, rows: [] });
  await expect(page.getByRole("status")).toHaveText("対象外の画面");
  await expect(page.locator("[data-state=empty]")).toContainText("デフォルト");
  await expect(page.getByLabel("名前で検索")).toHaveCount(0);
});

test("shows why the capture failed", async ({ page }) => {
  await open(page, { status: { state: "failed", message: "アクセスが拒否されました。" }, rows });
  await expect(page.getByRole("status")).toHaveText("キャプチャ失敗");
  await expect(page.getByRole("alert")).toHaveText("アクセスが拒否されました。");
});

for (const scheme of ["dark", "light"]) {
  test(`fits a narrow window in ${scheme}`, async ({ page }) => {
    await page.emulateMedia({ colorScheme: scheme });
    await page.setViewportSize({ width: 320, height: 640 });
    await open(page, { status: { state: "reading", fps: 9.6, dropped: 2 }, rows });
    expect(await overflow(page)).toBe(0);
    await expect(page.locator("body")).toHaveCSS(
      "background-color",
      scheme === "dark" ? "rgb(25, 25, 25)" : "rgb(250, 246, 239)",
    );
    await page.screenshot({ path: `test-results/narrow-${scheme}.png`, fullPage: false });
  });
}

test("the search field takes focus from the keyboard", async ({ page }) => {
  await open(page, { status: { state: "reading", fps: 20, dropped: 0 }, rows });
  await page.keyboard.press("Tab");
  await expect(page.getByLabel("名前で検索")).toBeFocused();
  await page.screenshot({ path: "test-results/wide-dark.png" });
});
