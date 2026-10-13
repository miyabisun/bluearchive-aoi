<script>
  import { onMount } from "svelte";
  import { SvelteSet } from "svelte/reactivity";
  import { exportAll, follow, iconUrl, setTarget } from "./lib/snapshot.js";
  import {
    LISTS,
    filterRows,
    filterShops,
    followTab,
    formatCount,
    goodTile,
    groupPlan,
    needCells,
    shopStale,
    sortStudents,
    starName,
    starOf,
    statusView,
    studentCells,
    tierCell,
    unknownText,
  } from "./lib/view.js";

  /**
   * The columns of a target: where each reads and changes, and its highest
   * value; the star is chosen from a list.
   */
  const TARGET_COLUMNS = [
    { label: "Lv", max: 90, at: ["level"] },
    { label: "★", star: true },
    { label: "EX", max: 5, at: ["skills", 0] },
    { label: "N", max: 10, at: ["skills", 1] },
    { label: "P", max: 10, at: ["skills", 2] },
    { label: "S", max: 10, at: ["skills", 3] },
    { label: "装備1", max: 10, at: ["equipment", 0] },
    { label: "装備2", max: 10, at: ["equipment", 1] },
    { label: "装備3", max: 10, at: ["equipment", 2] },
  ];

  /** A column's value in a target or in a student's training (an equipment slot's tier; "?" unread). */
  const valueAt = (from, /** @type {(string | number)[]} */ at) => {
    const [key, i] = at;
    const value = i === undefined ? from[key] : from[key][i];
    return value?.tier ?? value ?? "?";
  };

  /** Sets a target's column to `value`, kept within the column's range. */
  function retarget(row, column, value) {
    const [key, i] = column.at;
    const n = Math.min(Math.max(Math.round(Number(value)) || 1, 1), column.max);
    const target = structuredClone($state.snapshot(row.target));
    if (i === undefined) target[key] = n;
    else target[key][i] = n;
    setTarget(row.id, target);
  }

  /** The stars a target can be: ★1 to ★5, then the unique weapon's 2 to 4. */
  const STARS = [1, 2, 3, 4, 5, 6, 7, 8];

  /** Sets a target's star (kept as the student's while never chosen). */
  function restar(row, value) {
    const target = structuredClone($state.snapshot(row.target));
    target.star = Number(value);
    setTarget(row.id, target);
  }

  /** The student table's columns after the name, each sorting by its key. */
  const STUDENT_COLUMNS = [
    { key: "level", label: "Lv", num: true },
    { key: "rarity", label: "★" },
    { key: "weapon", label: "固有武器" },
    { key: "skills", label: "スキル" },
    { key: "equipment", label: "装備" },
    { key: "gear", label: "愛用品" },
    { key: "ability", label: "能力解放" },
    { key: "bond", label: "絆", num: true },
  ];

  /** What each tab tells before anything of its list is read. */
  const EMPTY = {
    items:
      "ブルーアーカイブのアイテム一覧を「デフォルト」の昇順で開いてスクロールすると、見えたアイテムの個数がここに並びます。",
    equipment:
      "ブルーアーカイブの装備一覧を「デフォルト」の昇順で開いてスクロールすると、見えた装備の個数がここに並びます。",
    students:
      "ブルーアーカイブの生徒詳細画面（基本情報）を開いて「＞」で送ると、見えた生徒の育成状況がここに並びます。",
    plan: "生徒タブで「育成」に印を付けた生徒の目標と、そこまでに要る素材の過不足がここに並びます。",
    shops:
      "ブルーアーカイブのショップで総力戦・大決戦・カケラ(文字)・合同火力演習・指名手配・熟達証書を開いて下まで送ると、品ごとにあと何回買えるかがここに並びます。",
  };

  let snapshot = $state({
    status: { state: "no_game" },
    items: [],
    equipment: [],
    students: [],
    shops: [],
    targets: [],
    plan: [],
    plan_unknown: [],
    credits: /** @type {number | null} */ (null),
    /** Why what was read could not be kept or brought back. */
    keep_error: /** @type {string | null} */ (null),
  });
  let view = $state({ tab: "items", lastRead: null });
  let query = $state("");
  /** The student column the table is ordered by. */
  let sortBy = $state("level");
  /** The last export: `{ path }` or `{ error }`. */
  let exported = $state(null);
  /**
   * The shops opened by hand, by name. Kept here, not in the page: every frame
   * read brings a new snapshot, which would close them again.
   */
  const opened = new SvelteSet();

  const status = $derived(statusView(snapshot.status));
  const rows = $derived(snapshot[view.tab] ?? []);
  const shown = $derived.by(() => {
    if (view.tab === "shops") return filterShops(rows, query);
    const found = filterRows(rows, query);
    return view.tab === "students" ? sortStudents(found, sortBy) : found;
  });
  /** How many students were read, to compare with how many the game holds. */
  const studentCount = $derived((snapshot.students ?? []).length);
  const nothingRead = $derived(LISTS.every((l) => (snapshot[l.id] ?? []).length === 0));
  /** The students marked for training, with their names and training as read. */
  const targeted = $derived(new Set((snapshot.targets ?? []).map((t) => t.id)));
  const targetRows = $derived(
    (snapshot.targets ?? []).map((t) => {
      const student = (snapshot.students ?? []).find((s) => s.id === t.id);
      return { ...t, name: student?.name ?? "", training: student?.training };
    }),
  );

  async function save() {
    try {
      exported = { path: await exportAll() };
    } catch (error) {
      exported = { error: String(error) };
    }
  }

  onMount(() => {
    follow((next) => {
      snapshot = next;
      view = followTab(view, next.status);
    });
  });
</script>

<div class="top">
  <header class="app-header">
    <h1>Aoi</h1>
    <p class="status" data-tone={status.tone} role="status">
      <span class="dot" aria-hidden="true"></span>{status.label}
    </p>
  </header>
  <div class="bar">
    <div class="tabs" role="tablist">
      {#each LISTS as list (list.id)}
        <button
          type="button"
          role="tab"
          id="tab-{list.id}"
          aria-selected={view.tab === list.id}
          aria-controls="list"
          onclick={() => (view = { ...view, tab: list.id })}
          >{list.name}{#if list.id === "students" && studentCount > 0}<span class="tally"
              >&nbsp;{studentCount}</span
            >{/if}</button
        >
      {/each}
    </div>
    <button type="button" class="export" disabled={nothingRead} onclick={save}>書き出す</button>
  </div>
</div>

<main class="content">
  <div id="list" role="tabpanel" aria-labelledby="tab-{view.tab}">
    {#if snapshot.status.state === "failed"}
      <p class="error" role="alert">{snapshot.status.message}</p>
    {/if}
    {#if snapshot.keep_error}
      <p class="error" role="alert">{snapshot.keep_error}</p>
    {/if}
    {#if exported?.error}
      <p class="error" role="alert">書き出せませんでした: {exported.error}</p>
    {:else if exported}
      <p class="exported" aria-live="polite">書き出しました: {exported.path}</p>
    {/if}

    {#if view.tab === "shops" && snapshot.credits != null}
      <p class="credits">
        クレジット <span class="amount">{formatCount({ exact: snapshot.credits })}</span>
      </p>
    {/if}

    {#if view.tab === "plan"}
      {#if targetRows.length === 0}
        <p class="empty" data-state="empty">{EMPTY.plan}</p>
      {:else}
        <div class="table-scroll">
          <table class="students targets">
            <thead>
              <tr>
                <th scope="col">生徒</th>
                {#each TARGET_COLUMNS as column (column.label)}
                  <th scope="col">{column.label}</th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each targetRows as row (row.id)}
                <tr>
                  <td class="name">{row.name}</td>
                  {#each TARGET_COLUMNS as column (column.label)}
                    {#if column.star}
                      {@const now = row.training ? starOf(row.training) : null}
                      {@const to = row.target.star ?? now}
                      <td>
                        <span class="now">{now === null ? "?" : starName(now)}→</span><select
                          value={to ?? ""}
                          aria-label="{row.name}の★の目標"
                          onchange={(event) => restar(row, event.currentTarget.value)}
                        >
                          {#if to === null}<option value="">?</option>{/if}
                          {#each STARS as star (star)}
                            <option value={star} disabled={now !== null && star < now}
                              >{starName(star)}</option
                            >
                          {/each}
                        </select>
                      </td>
                    {:else}
                      <td>
                        <span class="now"
                          >{row.training ? valueAt(row.training, column.at) : "?"}→</span
                        ><input
                          type="number"
                          min="1"
                          max={column.max}
                          value={valueAt(row.target, column.at)}
                          aria-label="{row.name}の{column.label}の目標"
                          onchange={(event) => retarget(row, column, event.currentTarget.value)}
                        />
                      </td>
                    {/if}
                  {/each}
                </tr>
              {/each}
            </tbody>
          </table>
          {#each unknownText(snapshot.plan_unknown ?? []) as line (line)}
            <p class="unknown">{line} は数えられません</p>
          {/each}
          {#if (snapshot.plan ?? []).length === 0}
            <p class="empty" data-state="reached">目標までに要る素材はありません</p>
          {:else}
            {@const plan = groupPlan(snapshot.plan)}
            <table class="plan" data-state="success">
              <thead>
                <tr>
                  <th scope="col">要るもの</th>
                  <th scope="col" class="fit">必要</th>
                  <th scope="col" class="fit">所持</th>
                  <th scope="col" class="fit">過不足</th>
                </tr>
              </thead>
              <tbody>
                {#each plan.rest as line (`${line.kind}-${line.id}`)}
                  {@const cells = needCells(line)}
                  <tr>
                    <td>{cells.name}</td>
                    <td class="fit">{cells.need}</td>
                    <td class="fit">{cells.have}</td>
                    <td class="fit" class:enough={cells.enough} class:short={!cells.enough}
                      >{cells.short}</td
                    >
                  </tr>
                {/each}
              </tbody>
            </table>
            {#if plan.families.length > 0}
              <!-- A kind's tiers on one line, told by their icons and colours. -->
              <table class="tiers" data-state="success">
                <thead class="hidden">
                  <tr>
                    {#each ["初級", "中級", "上級", "最上級"] as tier (tier)}
                      <th scope="col">{tier}</th>
                    {/each}
                  </tr>
                </thead>
                <tbody>
                  {#each plan.families as family (family.id)}
                    <tr>
                      {#each family.tiers as line, i (i)}
                        <td class="tier-{i + 1}">
                          {#if line}
                            {@const cell = tierCell(line)}
                            <img
                              src={iconUrl(line.icon)}
                              alt={line.name}
                              title={line.name}
                              width="40"
                              height="40"
                            /><span class:enough={cell.enough} class:short={!cell.enough}
                              >{cell.text}</span
                            >
                          {/if}
                        </td>
                      {/each}
                    </tr>
                  {/each}
                </tbody>
              </table>
            {/if}
          {/if}
        </div>
      {/if}
    {:else if rows.length === 0}
      <p class="empty" data-state="empty">{EMPTY[view.tab]}</p>
    {:else}
      <label class="search">
        <span>名前で検索</span>
        <input type="search" bind:value={query} autocomplete="off" />
      </label>
      {#if shown.length === 0}
        <p class="empty" data-state="no-match">一致する項目がありません</p>
      {:else if view.tab === "shops"}
        <div class="table-scroll">
          {#each shown as shop (shop.name)}
            <!-- Closed until opened, and open while a search narrows the goods. -->
            <details class="shop" open={query.trim() !== "" || opened.has(shop.name)}>
              <!-- Opened and closed here, not by the page: the snapshot of the
                   next frame may come before the page's toggle event. -->
              <summary
                onclick={(event) => {
                  event.preventDefault();
                  if (query.trim() !== "") return;
                  if (opened.has(shop.name)) opened.delete(shop.name);
                  else opened.add(shop.name);
                }}
              >
                <h2>
                  {shop.name}<span class="read-at"
                    >{shop.read_at.slice(5, 16).replace("-", "/")}</span
                  >{#if shopStale(shop)}<span class="stale">前の月の値</span>{/if}
                </h2>
                <!-- The coins held, as the shop shows them over its cards. -->
                <span class="held">
                  {#each shop.coins as coin (coin.name)}
                    <span title={coin.name}
                      ><img
                        src={iconUrl(coin.icon)}
                        alt={coin.name}
                        width="20"
                        height="20"
                      />{coin.held == null ? "?" : formatCount({ exact: coin.held })}</span
                    >
                  {/each}
                </span>
              </summary>
              <!-- Each good by its icon; under it, what each coin buys, the plain coin left. -->
              <ul class="tiles goods" class:pair={shop.coins.length > 1} data-state="success">
                {#each shop.goods as good (good.id)}
                  {@const tile = goodTile(good)}
                  <li title={good.name}>
                    <span class="item"
                      ><img
                        src={iconUrl(good.icon)}
                        alt={good.name}
                        width="64"
                        height="64"
                        loading="lazy"
                      />{#if tile.amount}<span class="count">{tile.amount}</span>{/if}</span
                    >
                    <span class="offers">
                      {#each tile.offers as offer, i (i)}
                        <span class="offer" class:sold={offer?.sold}
                          >{#if offer}<img
                              src={iconUrl(shop.coins[i].icon)}
                              alt={shop.coins[i].name}
                              title={shop.coins[i].name}
                              width="18"
                              height="18"
                            />{offer.text}{/if}</span
                        >
                      {/each}
                    </span>
                  </li>
                {/each}
              </ul>
            </details>
          {/each}
        </div>
      {:else if view.tab === "students"}
        <div class="table-scroll">
          <table data-state="success" class="students">
            <thead>
              <tr>
                <th scope="col" class="mark">育成</th>
                <th scope="col">生徒</th>
                {#each STUDENT_COLUMNS as column (column.key)}
                  <th
                    scope="col"
                    class:num={column.num}
                    aria-sort={sortBy === column.key ? "descending" : undefined}
                  >
                    <button type="button" class="sort" onclick={() => (sortBy = column.key)}
                      >{column.label}</button
                    >{#if sortBy === column.key}<span class="sorted" aria-hidden="true">▼</span
                      >{/if}
                  </th>
                {/each}
              </tr>
            </thead>
            <tbody>
              {#each shown as row (row.id)}
                {@const cells = studentCells(row.training)}
                <tr>
                  <td class="mark">
                    <input
                      type="checkbox"
                      checked={targeted.has(row.id)}
                      aria-label="{row.name}を育成対象にする"
                      onchange={() => setTarget(row.id, targeted.has(row.id) ? null : {})}
                    />
                  </td>
                  <td class="name">{row.name}</td>
                  <td class="num">{cells.level}</td>
                  <td>{cells.rarity}</td>
                  <td>{cells.weapon}</td>
                  <td class="code">{cells.skills}</td>
                  <td class="code" title={cells.equipmentLevels}>{cells.equipment}</td>
                  <td>{cells.gear}</td>
                  <td class="code">{cells.ability}</td>
                  <td class="num">{cells.bond}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <!-- Each item by its icon, the count at its corner and the name on hover. -->
        <div class="table-scroll">
          <ul class="tiles" data-state="success">
            {#each shown as row (row.id)}
              <li title={row.name}>
                <img
                  src={iconUrl(row.icon)}
                  alt={row.name}
                  width="64"
                  height="64"
                  loading="lazy"
                /><span class="count">{formatCount(row.count)}</span>
              </li>
            {/each}
          </ul>
        </div>
      {/if}
    {/if}
  </div>
</main>

<style lang="sass">
  // The page itself never scrolls: the top and the search field stay, and the
  // table scrolls in its own box under them, its headings held at the box's top.
  .top
    flex: none
    background: var(--c-wash-base)
    border-bottom: 1px solid var(--c-border)

  .app-header
    display: flex
    align-items: center
    justify-content: space-between
    gap: var(--sp-3)
    height: 48px
    padding: 0 var(--sp-3)

  .bar
    display: flex
    align-items: center
    padding-right: var(--sp-3)

  // Narrower than its tabs, the row scrolls sideways rather than wrap a name.
  .tabs
    display: flex
    min-width: 0
    overflow-x: auto
    scrollbar-width: thin
    padding: 0 var(--sp-1)
    button
      flex: none
      height: 40px
      padding: 0 var(--sp-3)
      white-space: nowrap
      border: 0
      border-bottom: 2px solid transparent
      background: none
      color: var(--c-muted)
      font: inherit
      font-size: var(--fs-sm)
      cursor: pointer
      &[aria-selected="true"]
        border-bottom-color: var(--c-accent)
        color: var(--c-on-surface)
        font-weight: 600

  .tally
    color: var(--c-muted)
    font-weight: 400
    font-variant-numeric: tabular-nums

  .export
    margin-left: auto
    height: 28px
    padding: 0 var(--sp-3)
    border: 1px solid var(--c-border)
    border-radius: var(--radius-sm)
    background: none
    color: var(--c-on-surface)
    font: inherit
    font-size: var(--fs-sm)
    white-space: nowrap
    cursor: pointer
    &:disabled
      color: var(--c-muted)
      cursor: default

  .credits
    margin: 0 0 var(--sp-3)
    font-size: var(--fs-sm)
    color: var(--c-muted)
    .amount
      color: var(--c-on-surface)
      font-variant-numeric: tabular-nums

  .exported
    margin: 0 0 var(--sp-4)
    font-size: var(--fs-sm)
    color: var(--c-muted)
    overflow-wrap: anywhere

  h1
    margin: 0
    font-size: var(--fs-title)
    font-weight: 600

  .status
    display: inline-flex
    align-items: center
    gap: var(--sp-2)
    margin: 0
    padding: 2px var(--sp-3)
    border: 1px solid var(--c-border)
    border-radius: 9999px
    font-size: var(--fs-sm)
    color: var(--c-muted)
    white-space: nowrap

  .dot
    width: 8px
    height: 8px
    border-radius: 9999px
    background: currentColor

  .status[data-tone="active"]
    color: var(--c-on-surface)
    .dot
      background: var(--c-accent)

  .status[data-tone="danger"]
    color: var(--c-danger)

  .content
    flex: 1
    min-height: 0
    display: flex
    flex-direction: column
    width: 100%
    max-width: 720px
    margin: 0 auto
    padding: var(--sp-4) var(--sp-3) 0
    box-sizing: border-box

  #list
    flex: 1
    min-height: 0
    display: flex
    flex-direction: column

  .error
    margin: 0 0 var(--sp-4)
    padding: var(--sp-3)
    border-radius: var(--radius-sm)
    background: var(--c-danger-subtle)
    color: var(--c-danger)
    font-size: var(--fs-sm)
    overflow-wrap: anywhere

  .empty
    margin: var(--sp-5) 0
    color: var(--c-muted)

  .search
    display: flex
    flex-direction: column
    gap: var(--sp-1)
    margin-bottom: var(--sp-4)
    span
      font-size: var(--fs-caption)
      color: var(--c-muted)
    input
      padding: var(--sp-2) var(--sp-3)
      border: 1px solid var(--c-border)
      border-radius: var(--radius-sm)
      background: var(--c-surface)
      color: inherit
      font: inherit
      &:focus
        border-color: var(--c-accent)

  table
    width: 100%
    // Separate borders stay on the held headings; collapsed ones do not.
    border-collapse: separate
    border-spacing: 0

  th, td
    padding: var(--sp-2) var(--sp-1)
    border-bottom: 1px solid var(--c-border)
    text-align: left
    overflow-wrap: anywhere

  th
    position: sticky
    top: 0
    background: var(--c-surface)
    font-size: var(--fs-sm)
    font-weight: 500
    color: var(--c-muted)

  .table-scroll
    flex: 1
    min-height: 0
    overflow: auto

  .students
    th, td
      padding: var(--sp-2)
      white-space: nowrap
    .sort
      padding: 0
      border: 0
      background: none
      color: inherit
      font: inherit
      cursor: pointer
    th[aria-sort] .sort
      color: var(--c-on-surface)
      font-weight: 600
    .sorted
      margin-left: 2px
      font-size: var(--fs-caption)
    td.name
      white-space: normal
      min-width: 8em

  .num
    text-align: right
    font-variant-numeric: tabular-nums

  .code
    font-variant-numeric: tabular-nums

  .shop
    border-bottom: 1px solid var(--c-border)
    &[open]
      padding-bottom: var(--sp-4)
    summary
      display: flex
      flex-wrap: wrap
      align-items: center
      gap: var(--sp-1) var(--sp-3)
      padding: var(--sp-2) 0
      cursor: pointer
    .tiles
      margin-top: var(--sp-1)

  .held
    display: inline-flex
    gap: var(--sp-3)
    margin-left: auto
    font-size: var(--fs-sm)
    font-variant-numeric: tabular-nums
    span
      display: inline-flex
      align-items: center
      gap: 2px
    img
      object-fit: contain

  // A good's tile: its icon over what each coin buys, two coins side by side.
  .tiles.goods
    grid-template-columns: repeat(auto-fill, minmax(112px, 1fr))
    &.pair
      grid-template-columns: repeat(auto-fill, minmax(176px, 1fr))
    li
      flex-direction: column
      align-items: center
      gap: var(--sp-1)
    .item
      position: relative
    .offers
      display: flex
      gap: var(--sp-2)
      width: 100%
      justify-content: center
    .offer
      display: inline-flex
      flex: 1
      justify-content: center
      align-items: center
      gap: 2px
      font-size: var(--fs-caption)
      font-variant-numeric: tabular-nums
      white-space: nowrap
      img
        width: 18px
        height: 18px

  h2
    display: inline-flex
    align-items: baseline
    gap: var(--sp-2)
    margin: 0
    font-size: var(--fs-md)
    font-weight: 600

  .read-at
    color: var(--c-muted)
    font-size: var(--fs-sm)
    font-weight: 400
    font-variant-numeric: tabular-nums

  .stale
    font-size: var(--fs-sm)

  .sold
    color: var(--c-muted)

  .mark
    width: 1%
    text-align: center

  // The current value over the target's field, so nine targets fit 720px.
  .targets
    margin-bottom: var(--sp-4)
    td:not(.name)
      padding: var(--sp-1)
    .now
      display: block
      color: var(--c-muted)
      font-size: var(--fs-caption)
      font-variant-numeric: tabular-nums
    input, select
      box-sizing: border-box
      width: 3em
      padding: 2px var(--sp-1)
      border: 1px solid var(--c-border)
      border-radius: var(--radius-sm)
      background: var(--c-surface)
      color: inherit
      font: inherit
      font-variant-numeric: tabular-nums
      &:focus
        border-color: var(--c-accent)
    // As wide as its longest star.
    select
      width: auto

  // Narrower than its numbers, the table scrolls sideways rather than squeeze the names.
  .plan :is(th, td):first-child
    min-width: 8em

  .unknown
    margin: 0 0 var(--sp-2)
    color: var(--c-muted)
    font-size: var(--fs-sm)

  .enough
    color: var(--c-ok)

  .short
    color: var(--c-danger)
    font-weight: 600

  // Each tier on its colour, from the grey of the first to the purple of the last.
  .tiers
    margin-top: var(--sp-4)
    td
      width: 25%
      padding: var(--sp-1) var(--sp-2)
      border-bottom: 2px solid var(--c-surface)
      font-variant-numeric: tabular-nums
      white-space: nowrap
    @for $i from 1 through 4
      .tier-#{$i}
        background: var(--c-tier-#{$i})
    img
      vertical-align: middle
      margin-right: var(--sp-2)

  .hidden
    position: absolute
    width: 1px
    height: 1px
    overflow: hidden
    clip-path: inset(50%)

  // As narrow as their contents, leaving the names the room.
  .fit
    width: 1%
    padding-left: var(--sp-3)
    text-align: right
    font-variant-numeric: tabular-nums
    white-space: nowrap

  .tiles
    display: grid
    grid-template-columns: repeat(auto-fill, minmax(64px, 1fr))
    gap: var(--sp-2)
    margin: 0
    padding: 0
    list-style: none
    li
      position: relative
      display: flex
      justify-content: center
      padding: var(--sp-1)
      border-radius: var(--radius-sm)
      background: var(--c-wash-base)
    img
      display: block
      width: 64px
      height: 64px
      object-fit: contain
    .count
      position: absolute
      right: 2px
      bottom: 2px
      padding: 0 var(--sp-1)
      border-radius: var(--radius-sm)
      background: var(--c-surface)
      font-size: var(--fs-caption)
      font-variant-numeric: tabular-nums
      white-space: nowrap
</style>
