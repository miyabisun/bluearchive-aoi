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
const ICONS = [
  "src-tauri/data/schaledb/icons/*.webp",
  "src-tauri/data/schaledb/equipment-icons/*.webp",
  "src-tauri/data/schaledb/school-icons/*.png",
];

const reading = (list, fps = 27.4) => ({ state: "reading", list, fps, dropped: 0 });

/**
 * Opens the page showing `snapshot` as the watcher's, with the app's icons,
 * and the `tab` named, if any, chosen.
 */
async function open(page, snapshot, tab) {
  await page.route("**/icon/*", (route) => {
    const name = new URL(route.request().url()).pathname.slice("/icon/".length);
    const file = ICONS.map((path) => path.replace("*", name)).find(existsSync);
    return file ? route.fulfill({ path: file }) : route.fulfill({ status: 404 });
  });
  await page.addInitScript((demo) => (window.__AOI_DEMO__ = demo), snapshot);
  await page.goto("/");
  if (tab) await page.getByRole("tab", { name: tab }).click();
}

/** The names of the tiles shown, as their hover tells them. */
const tileNames = (page) =>
  page.locator(".tiles li").evaluateAll((tiles) => tiles.map((t) => t.title));

/** The colour the theme's `token` (as `--c-accent`) computes to. */
const colorOf = (page, token) =>
  page.evaluate((t) => {
    const probe = document.createElement("span");
    probe.style.color = `var(${t})`;
    document.body.append(probe);
    const rgb = getComputedStyle(probe).color;
    probe.remove();
    return rgb;
  }, token);

const overflow = (page) =>
  page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);

test("lists the item tally in order and narrows it by name", async ({ page }) => {
  await open(page, { status: reading("items"), items, equipment });
  await expect(page.getByRole("status")).toHaveText("アイテムを集計中");
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
  await expect(page.getByRole("status")).toHaveText("装備を集計中");
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

/** A tier's cell of a kind: its icon and how many are held, against what one student takes. */
const heldCell = (id, name, icon, held, need = null) => ({ id, name, icon, held, need });

/** The tables by kind: the secret notes, two schools, two ooparts and a slot. */
/** A set of choice boxes of the skill materials: whether of every school, the schools. */
const boxSet = (all, schools, first) => ({
  all,
  schools,
  bd: [0, 1, 2, 3].map((t) =>
    heldCell(first + 4 + t, `BD選択ボックス${t}`, `item_icon_exskillcd_selection_${t}`, {
      exact: first + t,
    }),
  ),
  notes: [0, 1, 2, 3].map((t) =>
    heldCell(first + t, `ノート選択ボックス${t}`, `item_icon_skillbook_selection_${t}`, null),
  ),
});

const kinds = {
  secret: [
    heldCell(9999, "秘伝ノート", "item_icon_skillbook_ultimate", { exact: 1 }, 3),
    heldCell(9998, "秘伝ノートの断片", "item_icon_skillbook_ultimate_piece", { exact: 11 }),
  ],
  skill_boxes: [
    boxSet(true, ["トリニティ", "ゲヘナ", "ミレニアム"], 100),
    boxSet(false, ["トリニティ", "ゲヘナ"], 200),
  ],
  print_boxes: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10].map((t) =>
    t === 1
      ? null
      : heldCell(
          150020 + t,
          `T${t}装備設計図選択ボックス`,
          `equipment_icon_selection_tier${t}_piece`,
          t === 4 ? { exact: 732 } : null,
        ),
  ),
  oopart_boxes: [0, 1, 2, 3].map((t) =>
    heldCell(
      150008 + t,
      `オーパーツ（${t + 1}）選択ボックス`,
      `item_icon_material_selection_${t}`,
      {
        exact: 670 - t,
      },
    ),
  ),
  schools: ["trinity", "gehenna"].map((key, s) => ({
    name: ["トリニティ", "ゲヘナ"][s],
    icon: `school_icon_${key}`,
    bd: [0, 1, 2, 3].map((t) =>
      heldCell(3020 + 10 * s + t, `BD${t}`, `item_icon_material_exskill_${key}_${t}`, {
        exact: 10 * t + s,
      }),
    ),
    notes: [0, 1, 2, 3].map((t) =>
      heldCell(
        4020 + 10 * s + t,
        `ノート${t}`,
        `item_icon_skillbook_${key}_${t}`,
        t === 3 ? null : { exact: 100 * t + s },
        t === 0 ? 50 : null,
      ),
    ),
  })),
  ooparts: [
    [0, 1, 2, 3].map((t) =>
      heldCell(100 + t, `ネブラ${t}`, `item_icon_material_nebra_${t}`, { exact: 1680 - t }),
    ),
    [
      heldCell(110, "ファイストス0", "item_icon_material_phaistos_0", { exact: 2291 }),
      null,
      null,
      null,
    ],
  ],
  enhancements: [
    {
      category: "Exp",
      raise: { exp: 5700780, about: true, rate: 1, exp_each: 741690, students: 7.686 },
    },
    {
      category: "WeaponExpGrowthA",
      raise: { exp: 220280, about: false, rate: 1.5, exp_each: 49605, students: 6.661 },
    },
  ].map((row, k) => ({
    ...row,
    tiers: [0, 1, 2, 3].map((t) =>
      heldCell(k * 10 + t + 1, `${["強化珠", "スプリング"][k]}${t}`, `equipment_icon_exp_${t}`, {
        exact: 100 * k + t,
      }),
    ),
  })),
  blueprints: [
    {
      name: "帽子",
      icon: "equipment_icon_hat_tier10",
      tiers: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10].map((t) =>
        heldCell(
          t === 1 ? 1000 : 101_000 + t - 1,
          t === 1 ? "無地のキャップ" : `帽子T${t}の設計図`,
          t === 1 ? "equipment_icon_hat_tier1" : `equipment_icon_hat_tier${t}_piece`,
          t === 9 ? null : { exact: 1000 + t },
        ),
      ),
      universal: heldCell(501_000, "帽子の万能設計図", "equipment_icon_hat_useall_piece", {
        thousands: 59,
      }),
    },
  ],
};

test("the item tab's sub-tabs: every item on its rarity, the skill materials, the ooparts", async ({
  page,
}) => {
  const item = (id, name, icon, n, rarity, eleph = false) => ({
    id,
    name,
    icon,
    tier: null,
    count: { exact: n },
    rarity,
    eleph,
  });
  const held = [
    item(10019, "アズサの神名文字", "item_icon_secretstone_azusa", 120, "N", true),
    item(3023, "最上級戦術教育BD（トリニティ）", "item_icon_material_exskill_trinity_3", 30, "SSR"),
    item(4021, "中級技術ノート（トリニティ）", "item_icon_skillbook_trinity_1", 100, "R"),
  ];
  const training = { ...students[0].training, rarity: 5, weapon: { stars: 2, level: 40 } };
  const azusa = { id: 10019, name: "アズサ", training };
  const snapshot = { status: reading("items"), items: held, equipment, students: [azusa], kinds };
  await open(page, snapshot);
  // Everything held: each tile on its rarity, an eleph on pink with its student's star on hover.
  await expect(page.getByRole("tab", { name: "全体" })).toHaveAttribute("aria-selected", "true");
  const tiles = page.locator(".tiles li");
  const face = (t) => t.map((li) => [li.dataset.face, getComputedStyle(li).backgroundColor]);
  expect(await tiles.evaluateAll(face)).toEqual([
    ["eleph", await colorOf(page, "--c-eleph")],
    ["SSR", await colorOf(page, "--c-tier-4")],
    ["R", await colorOf(page, "--c-tier-2")],
  ]);
  await expect(tiles.first()).toHaveAttribute("title", "アズサの神名文字\n星: 固有2");
  await page.getByLabel("名前で検索").fill("ノート");
  await expect(tiles).toHaveCount(1);

  // The skill materials: no search; the secret notes, the choice boxes, then a school's
  // BDs over its notes.
  await page.getByRole("tab", { name: "スキル素材" }).click();
  await expect(page.getByLabel("名前で検索")).toHaveCount(0);
  const skills = page.locator("table.skills");
  const secret = skills.locator("tbody").first();
  await expect(secret.locator("td")).toHaveText(["", "1", "11", ""]);
  await expect(secret.getByRole("img", { name: "秘伝ノート", exact: true })).toBeVisible();
  // Short of one student red.
  await expect(secret.locator("td span").first()).toHaveClass(/short/);
  const emblems = skills.locator("th.emblem img");
  expect(await emblems.evaluateAll((i) => i.map((e) => e.alt))).toEqual(["トリニティ", "ゲヘナ"]);
  const loaded = await emblems.evaluateAll((i) => i.map((e) => e.complete && e.naturalWidth > 0));
  expect(loaded).toEqual([true, true]);
  // The choice boxes above トリニティ as one set (the user's of 2026-10-31: 「3種類の物を合算」),
  // what each set holds on hover.
  await expect(skills.locator("th.set")).toHaveText(["選択"]);
  const boxes = skills.locator("tbody").nth(1).locator("tr");
  await expect(boxes.nth(0).locator("td")).toHaveText(["300", "302", "304", "306"]);
  await expect(boxes.nth(0).locator("td").first()).toHaveAttribute("title", "100 + 200");
  await expect(boxes.nth(1).locator("td")).toHaveText(["—", "—", "—", "—"]);
  await expect(boxes.nth(1).locator("td").first()).toHaveAttribute("title", "0 + 0");
  await expect(boxes.nth(0).getByRole("img", { name: "BD選択ボックス0" })).toBeVisible();
  const trinity = skills.locator("tbody").nth(2).locator("tr");
  await expect(trinity.nth(0).locator("td")).toHaveText(["0", "10", "20", "30"]);
  await expect(trinity.nth(1).locator("td")).toHaveText(["0", "100", "200", "—"]);
  await expect(trinity.nth(1).locator("td").first()).toHaveClass(/tier-1/);
  // The choice holds as new frames come, and when the tab is left and taken again.
  await page.evaluate(
    (s) => window.__AOI_PUSH__({ ...s, status: { ...s.status, fps: 9 } }),
    snapshot,
  );
  await expect(page.getByRole("tab", { name: "スキル素材" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await page.getByRole("tab", { name: "装備", exact: true }).click();
  await page.getByRole("tab", { name: "アイテム" }).click();
  await expect(page.getByRole("tab", { name: "スキル素材" })).toHaveAttribute(
    "aria-selected",
    "true",
  );
  await expect(skills).toBeVisible();

  // The ooparts: their choice boxes on top (the user's of 2026-10-29), then one a row,
  // its tiers on their colours.
  await page.getByRole("tab", { name: "オーパーツ" }).click();
  const ooparts = page.locator("table.ooparts tr");
  await expect(ooparts).toHaveCount(3);
  await expect(ooparts.nth(0).locator("td")).toHaveText(["670", "669", "668", "667"]);
  await expect(
    ooparts.nth(0).getByRole("img", { name: "オーパーツ（1）選択ボックス" }),
  ).toBeVisible();
  await expect(ooparts.nth(1).locator("td")).toHaveText(["1,680", "1,679", "1,678", "1,677"]);
  await expect(ooparts.nth(2).locator("td")).toHaveText(["2,291", "", "", ""]);
  expect(await overflow(page)).toBe(0);
});

test("the equipment tab's blueprints: a slot a row, its T1 pieces to T10 blueprints and universal ones", async ({
  page,
}) => {
  await open(page, { status: reading("equipment"), items, equipment, kinds }, "装備");
  await expect(page.locator(".tiles li")).toHaveCount(4);
  await page.getByRole("tab", { name: "設計図" }).click();
  await expect(page.getByLabel("名前で検索")).toHaveCount(0);
  const prints = page.locator("table.prints");
  const tiers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
  await expect(prints.locator("thead th")).toHaveText([
    "部位",
    ...tiers.map((t) => `T${t}`),
    "万能",
  ]);
  const hat = prints.locator("tbody tr").first().locator("td");
  const held = tiers.map((t) => (t === 9 ? "—" : (1000 + t).toLocaleString("en")));
  await expect(hat).toHaveText(["帽子", ...held, "59K"]);
  await expect(hat.nth(2)).toHaveAttribute("title", "帽子T2の設計図");
  await expect(hat.last()).toHaveAttribute("title", "帽子の万能設計図");
  // The blueprints' choice boxes at the bottom (the user's of 2026-10-29), each under the
  // tier it gives.
  const boxes = prints.locator("tbody tr").last().locator("td");
  await expect(boxes).toHaveText([
    "選択ボックス",
    "",
    "—",
    "—",
    "732",
    "—",
    "—",
    "—",
    "—",
    "—",
    "—",
    "",
  ]);
  await expect(boxes.nth(4)).toHaveAttribute("title", "T4装備設計図選択ボックス");
  await page.setViewportSize({ width: 360, height: 640 });
  expect(await overflow(page)).toBe(0);

  // The enhancement materials: a kind a row, its tiers on their colours, then how many
  // students what is held raises and how; no search.
  await page.getByRole("tab", { name: "強化素材" }).click();
  await expect(page.getByLabel("名前で検索")).toHaveCount(0);
  const rows = page.locator("table.enhancements tr");
  await expect(rows).toHaveCount(2);
  await expect(rows.nth(0).locator("td")).toHaveText([
    "0",
    "1",
    "2",
    "3",
    "7.6人分約5.7M EXP/ 0.74M EXP",
  ]);
  await expect(rows.nth(1).locator("td")).toHaveText([
    "100",
    "101",
    "102",
    "103",
    "6.6人分330.4K EXP/ 49.6K EXP",
  ]);
  // The count at the row's right end, in three lines: the students, the exp held with
  // the rate taken in, over what one takes (the user's of 2026-10-29).
  const raised = rows.nth(1).locator("td.raise");
  const lines = [raised.locator("strong"), ...(await raised.locator(".detail").all())];
  await expect(lines[1]).toHaveText("330.4K EXP");
  await expect(lines[2]).toHaveText("/ 49.6K EXP");
  const tops = await Promise.all(lines.map(async (l) => (await l.boundingBox()).y));
  expect(tops).toEqual([...tops].sort((a, b) => a - b));
  expect(new Set(tops).size).toBe(3);
  await expect(raised).toHaveCSS("text-align", "right");
  // Every row's count as wide, however long its terms.
  const widths = await page
    .locator("table.enhancements td.raise")
    .evaluateAll((cells) => cells.map((c) => c.getBoundingClientRect().width));
  expect(new Set(widths).size).toBe(1);
  // The icons at their own shape (SchaleDB's 146 by 116), the count at the right, 10px in.
  const cell = rows.nth(1).locator("td").first();
  const icon = await cell.locator("img").boundingBox();
  expect(icon.width / icon.height).toBeCloseTo(146 / 116, 1);
  const [box, count] = [await cell.boundingBox(), await cell.locator("span").boundingBox()];
  expect(Math.round(box.x + box.width - (count.x + count.width))).toBe(10);
  expect(Math.round(icon.x - box.x)).toBe(10);
  await expect(rows.nth(1).getByRole("img").first()).toHaveAttribute("title", "スプリング0");
  for (let t = 0; t < 4; t++) {
    const face = await colorOf(page, `--c-tier-${t + 1}`);
    await expect(rows.nth(0).locator("td").nth(t)).toHaveCSS("background-color", face);
  }
  expect(await overflow(page)).toBe(0);
});

test("the status pauses the reading when pressed, and resumes it", async ({ page }) => {
  const snapshot = { status: reading("items"), items, equipment };
  await open(page, snapshot);
  const badge = page.getByRole("button", { name: /アイテムを集計中/ });
  await expect(badge).toHaveAttribute("aria-pressed", "false");
  // Its lamp shows where the frames come from, its hover says so and the frame rate.
  await expect(badge).toHaveAttribute("title", /^取り込み元: ゲームのウィンドウ\n27 fps\n/);
  await badge.click();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([{ paused: true }]);
  // The app answers with the reading paused: the lamp red.
  await page.evaluate((s) => window.__AOI_PUSH__({ ...s, status: { state: "paused" } }), snapshot);
  const paused = page.getByRole("button", { name: "一時中断" });
  await expect(paused).toHaveAttribute("aria-pressed", "true");
  const lamp = (selector) =>
    page.locator(selector).evaluate((icon) => getComputedStyle(icon).color);
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
  expect(await lamp(".status .lamp")).toBe(red);
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
  // Then the sub-tabs, and the search over everything.
  for (const name of ["全体", "設計図", "強化素材"]) {
    await page.keyboard.press("Tab");
    await expect(page.getByRole("tab", { name })).toBeFocused();
  }
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
  // ホシノ's elephs as the item list holds them; アカネ's never read.
  const eleph = {
    id: 10005,
    name: "ホシノの神名文字",
    icon: "item_icon_secretstone_hoshino",
    tier: null,
    count: { exact: 148 },
    eleph: true,
  };
  const snapshot = { status: reading("students"), items: [...items, eleph], equipment, students };
  await open(page, snapshot, "生徒");
  await expect(page.getByRole("status")).toHaveText("生徒を集計中");
  await expect(page.getByRole("tab", { name: "生徒" })).toHaveAttribute("aria-selected", "true");
  await expect(page.locator("tbody tr").first().locator("td")).toHaveText([
    "",
    "ホシノ",
    "90",
    "",
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
    "",
    "2336",
    "T4/?/3",
    "—",
    "0/0/0",
    "20",
  ]);
  // The star counts on into the unique weapon's, its level and the student's elephs
  // held on hover (the user's of 2026-10-29); drawn as the game's stars: the
  // weapon's in blue, the student's in yellow, named to read by.
  const hoshino = page.locator("tbody tr").first().locator("td").nth(3);
  await expect(hoshino).toHaveAttribute(
    "title",
    "固有武器 Lv60" + String.fromCharCode(10) + "神名文字: 148",
  );
  await expect(page.locator("tbody tr").nth(1).locator("td").nth(3)).toHaveAttribute(
    "title",
    "神名文字: ?",
  );
  const weapon = hoshino.getByRole("img", { name: "固有4" });
  await expect(weapon.locator("svg.star")).toHaveCount(4);
  await expect(weapon.locator("use").first()).toHaveAttribute("href", "#star-weapon");
  const student = page.locator("tbody tr").nth(1).getByRole("img", { name: "★3" });
  await expect(student.locator("svg.star")).toHaveCount(3);
  await expect(student.locator("use").first()).toHaveAttribute("href", "#star-student");
  // Overlapping as the game's student's stars: left edges 0.833 of a star's ink apart,
  // the weapon's alike (the user's of 2026-10-22; the ink 28/32 of the box drawn).
  const pitch = async (row) => {
    const [a, b] = await row
      .locator("svg.star")
      .evaluateAll((s) => s.slice(0, 2).map((e) => e.getBoundingClientRect()));
    return (b.left - a.left) / ((a.height * 28) / 32);
  };
  expect(await pitch(student)).toBeCloseTo(0.833, 2);
  expect(await pitch(weapon)).toBeCloseTo(0.833, 2);
  // The colours are the game's, whatever the theme.
  const fill = (id) =>
    page.locator(`#${id} stop`).evaluateAll((s) => s.map((e) => e.getAttribute("stop-color")));
  expect(await fill("star-student-fill")).toEqual(["#fcc63a", "#fed92c"]);
  expect(await fill("star-weapon-fill")).toEqual(["#6ce3fe", "#79e4fc"]);
  // The stars are drawn: a box of the size asked for, painted.
  const box = await student.locator("svg.star").first().boundingBox();
  expect(box.width).toBeGreaterThan(10);
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
        rarity: "N",
        eleph: false,
      },
      {
        id: 10019,
        name: "アズサの神名文字",
        icon: "item_icon_secretstone_azusa",
        offers: [null, { left: 0, price: { price: 50, amount: 5, limit: 10 } }],
        sold: true,
        rarity: "N",
        eleph: true,
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
        rarity: null,
        eleph: false,
      },
    ],
  },
];

test("shows each shop's goods, shop by shop, when the game shows a shop", async ({ page }) => {
  await open(page, { status: reading("shops"), items, equipment, shops }, "ショップ");
  await expect(page.getByRole("status")).toHaveText("ショップを集計中");
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
  // A good on its rarity's face (初級 on tier-1), a student's eleph on pink.
  await expect(tiles.first()).toHaveCSS("background-color", await colorOf(page, "--c-tier-1"));
  await expect(tiles.nth(1)).toHaveCSS("background-color", await colorOf(page, "--c-eleph"));
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
  await expect(row.locator(".now")).toHaveText(["?", "", "2", "3", "3", "6", "4", "?", "3"]);
  // The star read drawn as the game's.
  await expect(row.locator(".now").nth(1).getByRole("img", { name: "★3" })).toBeVisible();
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
  const star = page.getByRole("button", { name: "アカネ（バニーガール）の星の目標" });
  // Never chosen, the target stays at the student's star, drawn as the game's.
  await expect(star.getByRole("img", { name: "★3" })).toBeVisible();
  await star.click();
  const menu = page.getByRole("menu", { name: "アカネ（バニーガール）の星の目標" });
  const choices = menu.getByRole("menuitemradio");
  await expect(choices).toHaveCount(8);
  // Every star drawn, none below the student's now; the target ticked.
  const names = await choices.evaluateAll((b) => b.map((e) => e.ariaLabel));
  expect(names).toEqual(["★1", "★2", "★3", "★4", "固有1", "固有2", "固有3", "固有4"]);
  await expect(menu.locator("[role=menuitemradio]:disabled")).toHaveCount(2);
  await expect(menu.getByRole("menuitemradio", { name: "★3" })).toHaveAttribute(
    "aria-checked",
    "true",
  );
  await expect(menu.getByRole("menuitemradio", { name: "固有2" }).getByRole("img")).toHaveCount(1);
  // Open through the frames that keep coming, and over the table rather than cut by it.
  for (let frame = 0; frame < 3; frame++) {
    await page.evaluate(
      (s) => window.__AOI_PUSH__({ ...s, status: { ...s.status, fps: 9 } }),
      snapshot,
    );
  }
  await expect(menu).toBeVisible();
  await expect(menu).toBeInViewport({ ratio: 1 });
  await menu.getByRole("menuitemradio", { name: "固有2" }).click();
  await expect(menu).toBeHidden();
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
  await expect(star.getByRole("img", { name: "固有2" })).toBeVisible();
  // From the keyboard: open on the target, Escape back to the button; a click outside closes.
  await star.focus();
  await page.keyboard.press("Enter");
  await expect(menu.getByRole("menuitemradio", { name: "固有2" })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(menu).toBeHidden();
  await expect(star).toBeFocused();
  await star.click();
  await expect(menu).toBeVisible();
  await page.locator("h2", { hasText: "必要素材" }).click();
  await expect(menu).toBeHidden();
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
      golden: { students: 3, slots: 103, all: 834, total: 24 },
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
    report_tiers: [0, 1, 2, 3].map((t) =>
      heldCell(
        10 + t,
        `レポート${t}`,
        `item_icon_expitem_${t}`,
        t === 2 ? null : { exact: 10 * t },
      ),
    ),
    report_exp: 6245925,
    report_about: false,
    exp_each: 1249185,
    report_students: 5.0,
  },
  schools: [
    {
      exp: 220280,
      about: false,
      rate: 1.5,
      exp_each: 49605,
      students: 6.661,
      share: 0.4593,
      first: true,
    },
  ],
};

test("tells what the daily plays give against what is held", async ({ page }) => {
  const snapshot = { status: { state: "other_screen" }, items, equipment, kinds, work };
  await open(page, snapshot);
  await page.getByRole("tab", { name: "お仕事" }).click();
  // The plays grouped by when they are played; each section folds as a shop
  // does, closed until opened.
  await expect(page.locator(".work .category")).toHaveText(["日課", "2倍キャンペーン", "月末"]);
  const sections = page.locator(".work summary");
  await expect(sections).toHaveText([
    "ショップ: 通常アイテム",
    "指名手配",
    "学園交流会",
    "任務 Normal",
    "任務 Hard",
    "特別依頼",
  ]);
  await expect(page.locator(".work summary .chevron")).toHaveCount(6);
  await expect(page.locator(".slots")).toBeHidden();
  for (const section of await sections.all()) await section.click();
  // Opened by hand, they stay open while the window takes a snapshot every frame.
  for (let i = 0; i < 3; i++) await page.evaluate((s) => window.__AOI_PUSH__(s), snapshot);
  await expect(page.locator(".work details[open]")).toHaveCount(6);
  // A slot: how many pieces its T10 blueprints and its universal ones make,
  // short of its golden ratio orange.
  await expect(page.locator(".slots tbody tr")).toHaveText([
    "帽子2.5個分 150 / 605.0個分 約2,150 / 430",
  ]);
  // The slot's golden ratio in its hover, off the table's lines, with why.
  await expect(page.locator(".slots .slot")).toHaveAttribute(
    "title",
    /^黄金比 3人\n全実装生徒の装備欄 834 のうち 103\n/,
  );
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
  // Each bounty's drops by tier with what is held.
  await expect(page.locator(".work h4")).toHaveText([
    "ハイウェイ",
    "黄金比までの周回（2倍キャンペーン中）",
    "その後に黄金比を整える周回",
  ]);
  // The normal items shop, the scrimmages and the commissions as the enhancement
  // materials' rows (the user's of 2026-10-29): a title over what is held, the
  // students it raises at the right end, its terms under it; "—" where none is counted.
  await expect(page.locator(".work table.raising th")).toHaveText([
    "クレジット",
    "強化珠",
    "トリニティ",
    "拠点防衛",
    "クレジット回収",
  ]);
  await expect(page.locator(".raises tbody")).toHaveText([
    "クレジット248,955,6865.9人分249M/ 41.5M",
    "強化珠01237.6人分約5.7M EXP/ 0.74M EXP",
  ]);
  await expect(page.locator(".raises .credits img")).toHaveAttribute("alt", "クレジット");
  await expect(page.locator(".schools tbody td.tier-1")).toHaveText(["—"]);
  await expect(page.locator(".commissions tbody")).toHaveText([
    "拠点防衛010—305.0人分6.2M EXP/ 1.2M EXP",
    "クレジット回収248,955,6865.9人分249M/ 41.5M",
  ]);
  // The counts lined up at the right end of each section, their terms under them.
  for (const kind of ["raises", "commissions"]) {
    const counts = page.locator(`.${kind} td.raise strong`);
    const [a, b] = await counts.evaluateAll((all) => all.map((c) => c.getBoundingClientRect()));
    expect(a.right).toBe(b.right);
    const terms = await page.locator(`.${kind} td.raise .detail`).first().boundingBox();
    expect(terms.y).toBeGreaterThan(a.top);
  }
  const table = await page.locator(".raises").boundingBox();
  const count = await page.locator(".raises td.raise strong").first().boundingBox();
  expect(table.x + table.width - (count.x + count.width)).toBeLessThan(16);
  // A title scrolls with its row, unlike the tables' headings that stick.
  const tiers = page.locator(".schools tbody tr").last();
  await tiers.evaluate((el) => el.scrollIntoView({ block: "start" }));
  const title = await page.locator(".schools th").boundingBox();
  expect(title.y + title.height).toBeLessThanOrEqual((await tiers.boundingBox()).y + 1);
  await page.locator(".table-scroll.work").evaluate((el) => (el.scrollTop = 0));
  // A school: the weapons its parts raise, the one to play first red.
  const red = await page.evaluate(() => {
    const probe = document.createElement("span");
    probe.style.color = "var(--c-danger)";
    document.body.append(probe);
    const rgb = getComputedStyle(probe).color;
    probe.remove();
    return rgb;
  });
  const school = page.locator(".schools tbody");
  await expect(school).toHaveText("トリニティ—6.6人分330.4K EXP/ 49.6K EXP");
  await expect(school.locator("strong")).toHaveCSS("color", red);
  await expect(school.locator("th")).toHaveCSS("color", red);
  // Why, on hover alone: the students its parts suit and the weapons per them.
  await expect(school).toHaveAttribute(
    "title",
    ["素材が合う生徒: 46%", "6.6人分 ÷ 46% = 14.5", "最も少ないので先に回します"].join(
      String.fromCharCode(10),
    ),
  );
  // Short of one student red, of two blue, enough for two green.
  await expect(page.locator(".work .tiers .short")).toHaveText(["51"]);
  await expect(page.locator(".work .tiers .fair")).toHaveText(["100"]);
  await expect(page.locator(".work .tiers .enough")).toHaveText(["200"]);
  const colours = await page
    .locator(".work .tiers :is(.short, .fair, .enough)")
    .evaluateAll((cells) => cells.map((c) => getComputedStyle(c).color));
  expect(new Set(colours).size).toBe(3);
  await expect(page.getByRole("img", { name: "ファイストス円盤の欠片" })).toBeVisible();
  // The commission raising more students is green, the fewer orange; the
  // reports first, as the game lists them.
  await expect(page.locator(".commissions tbody.more")).toContainText("クレジット回収");
  await expect(page.locator(".commissions tbody.fewer")).toContainText("拠点防衛");
  await expect(page.locator(".commissions tbody.fewer strong")).toHaveCSS(
    "color",
    await colorOf(page, "--c-warn"),
  );
  // The normal items shop: the credits and the orbs held in students, the fewer red.
  await expect(page.locator(".raises tbody.fewer")).toContainText("クレジット");
  await expect(page.locator(".raises tbody.fewer strong")).toHaveCSS("color", red);
  await expect(page.locator(".raises tbody.more strong")).toHaveCSS(
    "color",
    await colorOf(page, "--c-ok"),
  );
  // Narrow, the rows scroll in the tab rather than the page.
  await page.setViewportSize({ width: 360, height: 640 });
  expect(await overflow(page)).toBe(0);
});

test("the item tab's 通貨等: the credits and the reports in students, then the coins", async ({
  page,
}) => {
  const coin = (id, name, icon, held) => heldCell(id, name, icon, held);
  const coins = [
    coin(7, "総力戦コイン", "item_icon_raidcoin", { exact: 1160 }),
    coin(8, "戦術コイン", "item_icon_arenacoin", null),
  ];
  // 熟達証書 is no item: its count is its shop's.
  const shops = [
    {
      name: "熟達証書",
      read_at: "2026-10-29 12:00:00",
      resets: true,
      coins: [{ name: "熟達証書", icon: "currency_icon_mastercoin", held: { exact: 320 } }],
      goods: [],
    },
  ];
  const snapshot = {
    status: { state: "other_screen" },
    items,
    equipment,
    kinds: { ...kinds, coins },
    work,
    shops,
  };
  await open(page, snapshot, "アイテム");
  await page.getByRole("tab", { name: "通貨等" }).click();
  await expect(page.getByLabel("名前で検索")).toHaveCount(0);
  // As the commissions count them (the user's of 2026-10-29), the credits first; more
  // green, fewer orange.
  const rows = page.locator("table.raising tbody");
  await expect(rows).toHaveText([
    "クレジット248,955,6865.9人分249M/ 41.5M",
    "レポート010—305.0人分6.2M EXP/ 1.2M EXP",
  ]);
  await expect(rows.nth(0).locator("strong")).toHaveCSS("color", await colorOf(page, "--c-ok"));
  await expect(rows.nth(1).locator("strong")).toHaveCSS("color", await colorOf(page, "--c-warn"));
  const [a, b] = await page
    .locator("table.raising td.raise strong")
    .evaluateAll((all) => all.map((c) => c.getBoundingClientRect().right));
  expect(a).toBe(b);
  // The coins held as tiles, the item list's in its order, then 熟達証書 as its shop read it.
  const tiles = page.locator(".tiles li");
  expect(await tiles.evaluateAll((t) => t.map((e) => e.title))).toEqual([
    "総力戦コイン",
    "戦術コイン",
    "熟達証書",
  ]);
  await expect(tiles.locator(".count")).toHaveText(["1,160", "—", "320"]);
  await page.setViewportSize({ width: 360, height: 640 });
  expect(await overflow(page)).toBe(0);
});

test("reserves a hard mission's eleph, the day's AP it takes red in the hard missions' heading", async ({
  page,
}) => {
  const card = (id, name, more) => ({
    id,
    name,
    icon: "item_icon_secretstone_azusa",
    stages: ["18-3"],
    ap: 60,
    star: 7,
    held: { exact: 80 },
    done: false,
    reserved: false,
    ...more,
  });
  const hard = (reserved) => [
    card(10000, "アルの神名文字", { reserved: reserved.includes(10000) }),
    card(13010, "ユウカの神名文字", {
      stages: ["1-1", "4-3", "12-2"],
      ap: 180,
      star: 2,
      reserved: reserved.includes(13010),
    }),
    card(16000, "ハルカの神名文字", { star: 8, done: true }),
  ];
  const snapshot = { status: reading("shops"), items, equipment, work, hard: hard([]) };
  await open(page, snapshot, "お仕事");
  const heading = page.locator(".work summary", { hasText: "任務 Hard" });
  await expect(heading).toHaveText("任務 Hard");
  await heading.click();
  // A student's eleph a card: the missions dropping it in the place of "×5", the star
  // and the elephs short of 固有4 in the place of the price; 固有4 greyed, last.
  const cards = page.locator(".work details", { hasText: "任務 Hard" }).locator("li");
  await expect(cards.locator(".count")).toHaveText(["1個所", "3個所", "1個所"]);
  await expect(cards.locator(".offer")).toHaveText([/^\s*80文字$/, /^\s*80文字$/, /^\s*80文字$/]);
  // The star as the game draws it, before the elephs held.
  const named = await cards
    .locator(".offer [role=img]")
    .evaluateAll((s) => s.map((e) => e.ariaLabel));
  expect(named).toEqual(["固有3", "★2", "固有4"]);
  await expect(cards.last()).toHaveClass(/sold/);
  await expect(cards.first()).toHaveCSS("background-color", await colorOf(page, "--c-eleph"));
  const aru = page.getByRole("button", { name: /アルの神名文字/ });
  await expect(aru).toHaveAttribute("title", /任務 Hard: 18-3\n1日 60 AP$/);
  await aru.click();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([
    { card: { shop: "任務 Hard", id: 10000, coin: "AP" }, reserved: true },
  ]);
  // The app answers reserved: ringed and marked, and the hard missions' heading takes
  // a day's AP for it in red (not the campaign's: one hard mission is asked a week, so a
  // campaign's total means nothing); the fold stays open through the frames that keep coming.
  for (let frame = 0; frame < 3; frame++) {
    await page.evaluate((s) => window.__AOI_PUSH__(s), { ...snapshot, hard: hard([10000]) });
  }
  await expect(aru).toHaveAttribute("aria-pressed", "true");
  await expect(aru).toContainText("予約中");
  const taken = heading.locator(".held");
  await expect(taken).toHaveText("-60");
  await expect(taken.getByRole("img", { name: "AP" })).toBeVisible();
  await expect(taken.locator(".short")).toHaveCSS("color", await colorOf(page, "--c-danger"));
  // The only AP shown is the hard missions': none by the campaign's heading.
  await expect(page.locator(".work .category").nth(1)).toHaveText("2倍キャンペーン");
  await expect(
    page.locator(".work .held", { has: page.getByRole("img", { name: "AP" }) }),
  ).toHaveCount(1);
  await page.evaluate((s) => window.__AOI_PUSH__(s), {
    ...snapshot,
    hard: hard([10000, 13010]),
  });
  await expect(taken).toHaveText("-240");
  // Taken back, the heading is the hard missions' name alone again.
  await page.evaluate((s) => window.__AOI_PUSH__(s), snapshot);
  await expect(heading).toHaveText("任務 Hard");
  await expect(page.locator(".work details", { hasText: "任務 Hard" })).toHaveAttribute("open", "");
  expect(await overflow(page)).toBe(0);
});

test("reserves a month-end card by pressing it, kept as new frames come", async ({ page }) => {
  const coins = (need, short) => [
    { name: "総力戦コイン", icon: "", held: { exact: 600 }, need, short },
    { name: "総力戦レアコイン", icon: "", held: { exact: 40 }, need: 0, short: false },
  ];
  const card = (id, name, more) => ({
    id,
    name,
    icon: "",
    coin: 0,
    left: 16,
    price: 50,
    amount: 5,
    reserved: false,
    barred: false,
    held: null,
    eleph: true,
    rarity: "N",
    ...more,
  });
  const plan = (reserved) => [
    {
      name: "総力戦",
      read_at: "2026-10-16 07:00:00",
      coins: reserved ? coins(800, true) : coins(0, false),
      cards: [
        card(10019, "アズサの神名文字", { reserved, held: { exact: 120 } }),
        card(10020, "ハナコの神名文字", { left: 0 }),
        card(10021, "マキの神名文字", { left: 0, barred: true }),
      ],
    },
    { name: "大決戦", read_at: null, coins: [], cards: [] },
    { name: "指名手配", read_at: null, coins: [], cards: [] },
  ];
  const trained = { rarity: 5, weapon: { stars: 2, level: 40 } };
  const snapshot = {
    status: reading("shops"),
    items,
    equipment,
    students: [{ id: 10019, name: "アズサ", training: trained }],
    work,
    month_end: plan(false),
  };
  await open(page, snapshot, "お仕事");
  await page.locator(".work summary", { hasText: "ショップ: 総力戦" }).click();
  // A card a good and a coin: the times left at the price, bought up, or its student done.
  const cards = page.locator(".plan li");
  await expect(cards).toHaveText(["×5 16回@50", "×5 売切れ", /^×5\s*$/]);
  // The student at 固有4: the weapon's four stars in the place of the times left.
  await expect(cards.last().getByRole("img", { name: "固有4" }).locator("svg.star")).toHaveCount(4);
  // The elephs on pink, as the shop tab paints them, though their rarity is 初級.
  for (let i = 0; i < 3; i++) {
    await expect(cards.nth(i)).toHaveCSS("background-color", await colorOf(page, "--c-eleph"));
  }
  for (const [i, name] of ["アズサ", "ハナコ", "マキ"].entries()) {
    await expect(cards.nth(i).getByRole("button")).toHaveAccessibleName(
      new RegExp(`${name}の神名文字`),
    );
  }
  const azusa = page.getByRole("button", { name: /アズサの神名文字/ });
  const coinsLeft = page
    .locator(".work details", { hasText: "ショップ: 総力戦" })
    .locator(".held > span");
  // Nothing reserved: the coins held, in the text's colour.
  await expect(coinsLeft).toHaveText(["600", "40"]);
  const onSurface = await colorOf(page, "--c-on-surface");
  await expect(coinsLeft.first().locator("span")).toHaveCSS("color", onSurface);
  await expect(azusa).toHaveAttribute("aria-pressed", "false");
  // Its hover tells the student's star and how many of the eleph are held.
  await expect(azusa).toHaveAttribute(
    "title",
    ["アズサの神名文字", "星: 固有2", "所持: 120"].join("\n"),
  );
  await azusa.click();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([
    { card: { shop: "総力戦", id: 10019, coin: "総力戦コイン" }, reserved: true },
  ]);
  // The app answers reserved: ringed and marked, the coin short in red; the
  // section stays open through the frames that keep coming.
  for (let frame = 0; frame < 3; frame++) {
    await page.evaluate((s) => window.__AOI_PUSH__(s), { ...snapshot, month_end: plan(true) });
  }
  await expect(azusa).toHaveAttribute("aria-pressed", "true");
  await expect(azusa).toContainText("予約中");
  // The mark sits over the card's top right corner, the card no taller than the others.
  const box = await azusa.boundingBox();
  const other = await page.getByRole("button", { name: /ハナコの神名文字/ }).boundingBox();
  expect(box.height).toBe(other.height);
  const mark = await azusa.getByText("予約中").boundingBox();
  expect(Math.abs(mark.y - box.y)).toBeLessThanOrEqual(1);
  expect(Math.abs(mark.x + mark.width - (box.x + box.width))).toBeLessThanOrEqual(1);
  await expect(azusa).toHaveCSS("border-top-color", await colorOf(page, "--c-accent"));
  // The coins left once the cards reserved are paid: 600 less 16 × 50, short red.
  await expect(coinsLeft).toHaveText(["-200", "40"]);
  await expect(page.locator(".work .held .short")).toHaveText(["-200"]);
  await expect(coinsLeft.first().locator("span")).toHaveCSS(
    "color",
    await colorOf(page, "--c-danger"),
  );
  // Enough for what is reserved: blue, so the count left is not taken for the game's.
  const enough = plan(true).map((s, i) => (i === 0 ? { ...s, coins: coins(400, false) } : s));
  await page.evaluate((s) => window.__AOI_PUSH__(s), { ...snapshot, month_end: enough });
  await expect(coinsLeft).toHaveText(["200", "40"]);
  const blue = await colorOf(page, "--c-accent");
  await expect(coinsLeft.first().locator("span")).toHaveCSS("color", blue);
  await expect(coinsLeft.last().locator("span")).toHaveCSS("color", onSurface);
  await page.evaluate((s) => window.__AOI_PUSH__(s), { ...snapshot, month_end: plan(true) });
  await expect(coinsLeft.first()).toHaveAttribute("title", "総力戦コイン: 所持 600 − 予約 800");
  // A shop never read says so.
  await page.locator(".work summary", { hasText: "ショップ: 大決戦" }).click();
  await expect(
    page.locator(".work details", { hasText: "ショップ: 大決戦" }).getByText("まだ読んでいません"),
  ).toBeVisible();
  // The shop 指名手配 opens apart from the daily 指名手配.
  await page.locator(".work summary", { hasText: "ショップ: 指名手配" }).click();
  await expect(page.locator(".work .tiers").first()).toBeHidden();
});

test("keeps the tab chosen when the game shows another list", async ({ page }) => {
  const snapshot = { status: reading("items"), items, equipment, students };
  await open(page, snapshot, "生徒");
  await page.evaluate(
    (s) => window.__AOI_PUSH__({ ...s, status: { ...s.status, list: "equipment" } }),
    snapshot,
  );
  await expect(page.getByRole("status")).toHaveText("装備を集計中");
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
  await expect(toast).toHaveText("最新のバージョンです（v0.1.29）");
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
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([{ pair: true, usb: true }]);

  const streaming = { state: "streaming", device: "Pixel 9", via: "wifi" };
  await open(page, { status: { state: "no_source" }, items, equipment, phone: streaming });
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "Androidとの通信" }).click();
  await expect(page.getByText("Pixel 9 の画面を Wi-Fi で受け取っています")).toBeVisible();
  await page.getByRole("button", { name: "切断" }).click();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([{ connect: false }]);
  // Another phone is paired over Wi-Fi alone: the one plugged in would take it.
  await page.getByRole("button", { name: "別のスマホとペア設定" }).click();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([
    { connect: false },
    { pair: true, usb: false },
  ]);

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

test("the menu waits for a streaming app: a QR code for Moblin or another app, then the screen coming in", async ({
  page,
}) => {
  await open(page, { status: { state: "no_source" }, items, equipment, cast: { state: "off" } });
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "配信アプリとの連携" }).click();
  const dialog = page.getByRole("dialog", { name: "配信アプリとの連携" });
  await expect(dialog).toBeVisible();
  // Not waiting: it starts at once.
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([{ wait: true }]);
  await dialog.getByRole("button", { name: "閉じる" }).click();

  const url = "rtmp://192.168.1.20:1935/aoi/Ab3";
  const waiting = {
    state: "waiting",
    url,
    moblin: [
      [true, false],
      [false, true],
    ],
    plain: [[true]],
    problem: "キーの違う配信が届いたので断りました。QR コードを読み取り直してください",
  };
  await open(page, { status: { state: "no_source" }, items, equipment, cast: waiting });
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "配信アプリとの連携" }).click();
  await expect(dialog.getByRole("img", { name: "Moblin に取り込む QR コード" })).toBeVisible();
  await expect(dialog.getByText(url)).toBeVisible();
  await expect(
    dialog.getByText("キーの違う配信が届いたので断りました。QR コードを読み取り直してください"),
  ).toBeVisible();
  await dialog.getByRole("tab", { name: "ほかのアプリ" }).click();
  await expect(dialog.getByRole("img", { name: "配信先の URL の QR コード" })).toBeVisible();
  await expect(dialog.getByText("配信アプリの配信先（RTMP）に上の URL を入れる")).toBeVisible();
  // Already waiting: no new key.
  expect(await page.evaluate(() => window.__AOI_CALLS__ ?? [])).toEqual([]);
  await dialog.getByRole("button", { name: "やめる" }).click();
  expect(await page.evaluate(() => window.__AOI_CALLS__)).toEqual([{ wait: false }]);

  const receiving = { state: "receiving", from: "192.168.1.20", sideways: true };
  await open(page, { status: { state: "no_source" }, items, equipment, cast: receiving });
  await page.getByRole("button", { name: "メニュー" }).click();
  await page.getByRole("menuitem", { name: "配信アプリとの連携" }).click();
  await expect(dialog.getByText("192.168.1.20 から画面を受け取っています")).toBeVisible();
  await expect(dialog.getByText("縦向き（Portrait）の配信にすると大きく届きます")).toBeVisible();
});

test("a double click on a fold's title opens and closes it and selects no text", async ({
  page,
}) => {
  await open(page, { status: { state: "other_screen" }, items, equipment, work });
  await page.getByRole("tab", { name: "お仕事" }).click();
  await page.locator(".work summary h3", { hasText: "任務 Normal" }).dblclick();
  expect(await page.evaluate(() => window.getSelection()?.toString())).toBe("");
});
