<script lang="ts">
  import { FilterGroup, StatCard } from "@ccs/ui";
  import { state, loadAll } from "../store.svelte";
  const windows = ["1h", "24h", "7d", "all"];
  async function setWindow(w: string) {
    state.filters.window = w;
    await loadAll();
  }
</script>

<aside class="w-64 shrink-0 overflow-y-auto border-r border-border p-3">
  <FilterGroup title="Upstreams" options={state.meta.upstreams} bind:selected={state.filters.upstreams} />
  <FilterGroup title="Models" options={state.meta.models} bind:selected={state.filters.models} />
  <FilterGroup title="Working dirs" options={state.meta.cwds} bind:selected={state.filters.cwds} />
  <section class="border-b border-border py-2">
    <div class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Time</div>
    <div class="mt-2 flex gap-1">
      {#each windows as w}
        <button class="rounded-md border px-2 py-0.5 text-xs {state.filters.window === w ? 'border-primary bg-primary/10 text-primary' : 'border-border text-muted-foreground'}"
                onclick={() => setWindow(w)}>{w}</button>
      {/each}
    </div>
  </section>
  {#if state.stats}
    <section class="mt-3 space-y-2">
      <StatCard label="Requests" value={state.stats.total_requests} />
      <StatCard label="Tokens" value={state.stats.total_tokens} />
      <StatCard label="Errors" value={state.stats.error_count} />
    </section>
  {/if}
</aside>
