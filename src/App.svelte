<script>
  import { onMount } from "svelte";
  import { exportAll, follow } from "./lib/snapshot.js";
  import {
    LISTS,
    filterRows,
    followTab,
    formatCount,
    sortStudents,
    statusView,
    studentCells,
  } from "./lib/view.js";

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
  };

  let snapshot = $state({
    status: { state: "no_game" },
    items: [],
    equipment: [],
    students: [],
  });
  let view = $state({ tab: "items", lastRead: null });
  let query = $state("");
  /** The student column the table is ordered by. */
  let sortBy = $state("level");
  /** The last export: `{ path }` or `{ error }`. */
  let exported = $state(null);

  const status = $derived(statusView(snapshot.status));
  const rows = $derived(snapshot[view.tab]);
  const shown = $derived.by(() => {
    const found = filterRows(rows, query);
    return view.tab === "students" ? sortStudents(found, sortBy) : found;
  });
  const heading = $derived(LISTS.find((l) => l.id === view.tab).name);
  /** How many students were read, to compare with how many the game holds. */
  const studentCount = $derived((snapshot.students ?? []).length);
  const nothingRead = $derived(LISTS.every((l) => (snapshot[l.id] ?? []).length === 0));

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
    {#if exported?.error}
      <p class="error" role="alert">書き出せませんでした: {exported.error}</p>
    {:else if exported}
      <p class="exported" aria-live="polite">書き出しました: {exported.path}</p>
    {/if}

    {#if rows.length === 0}
      <p class="empty" data-state="empty">{EMPTY[view.tab]}</p>
    {:else}
      <label class="search">
        <span>名前で検索</span>
        <input type="search" bind:value={query} autocomplete="off" />
      </label>
      {#if shown.length === 0}
        <p class="empty" data-state="no-match">一致する項目がありません</p>
      {:else if view.tab === "students"}
        <div class="table-scroll">
          <table data-state="success" class="students">
            <thead>
              <tr>
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
                  <td>{row.name}</td>
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
        <div class="table-scroll">
          <table data-state="success">
            <thead>
              <tr><th scope="col">{heading}</th><th scope="col" class="count">個数</th></tr>
            </thead>
            <tbody>
              {#each shown as row (row.id)}
                <tr>
                  <td>
                    {#if row.tier}<span class="tier">T{row.tier}</span>{/if}{row.name}
                  </td>
                  <td class="count">{formatCount(row.count)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
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

  .tabs
    display: flex
    padding: 0 var(--sp-1)
    button
      height: 40px
      padding: 0 var(--sp-3)
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

  .exported
    margin: 0 0 var(--sp-4)
    font-size: var(--fs-sm)
    color: var(--c-muted)
    overflow-wrap: anywhere

  .tier
    margin-right: var(--sp-2)
    color: var(--c-muted)
    font-size: var(--fs-sm)
    font-variant-numeric: tabular-nums

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
    td:first-child
      white-space: normal
      min-width: 8em

  .num
    text-align: right
    font-variant-numeric: tabular-nums

  .code
    font-variant-numeric: tabular-nums

  .count
    width: 7em
    text-align: right
    font-variant-numeric: tabular-nums
    white-space: nowrap
</style>
