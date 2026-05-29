<script lang="ts" generics="T extends Record<string, any>">
  import { sortRows, type SortDir } from "../sort";
  import type { Snippet } from "svelte";

  type Column = { key: keyof T & string; label: string; sortable?: boolean };
  let {
    columns,
    rows,
    row,
    onRowClick,
  }: {
    columns: Column[];
    rows: T[];
    row: Snippet<[T]>;
    onRowClick?: (r: T) => void;
  } = $props();

  let sortKey = $state<(keyof T & string) | null>(null);
  let sortDir = $state<SortDir>("desc");
  const sorted = $derived(sortKey ? sortRows(rows, sortKey, sortDir) : rows);

  function toggle(key: keyof T & string) {
    if (sortKey === key) sortDir = sortDir === "asc" ? "desc" : "asc";
    else {
      sortKey = key;
      sortDir = "asc";
    }
  }
</script>

<table class="w-full text-sm">
  <thead class="border-b border-border text-left text-muted-foreground">
    <tr>
      {#each columns as col}
        <th class="px-3 py-2 font-medium">
          {#if col.sortable}
            <button class="hover:text-foreground" onclick={() => toggle(col.key)}>
              {col.label}{sortKey === col.key
                ? sortDir === "asc"
                  ? " ↑"
                  : " ↓"
                : ""}
            </button>
          {:else}{col.label}{/if}
        </th>
      {/each}
    </tr>
  </thead>
  <tbody>
    {#each sorted as r (r)}
      <tr
        class="border-b border-border/50 hover:bg-muted/50 {onRowClick
          ? 'cursor-pointer'
          : ''}"
        onclick={() => onRowClick?.(r)}
      >
        {@render row(r)}
      </tr>
    {/each}
  </tbody>
</table>
