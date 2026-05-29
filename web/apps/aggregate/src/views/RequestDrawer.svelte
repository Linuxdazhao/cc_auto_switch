<script lang="ts">
  import { state as store, closeDetail } from "../store.svelte";
  import OverviewTab from "./detail/OverviewTab.svelte";
  import RequestTab from "./detail/RequestTab.svelte";
  import ResponseTab from "./detail/ResponseTab.svelte";

  const d = $derived(store.selected);
  const tabs = ["overview", "request", "response"] as const;
  let activeTab = $state<"overview" | "request" | "response">("overview");
  let viewMode = $state<"structured" | "raw">("structured");
</script>

{#if d}
  <div
    class="fixed inset-y-0 right-0 z-40 flex w-[56rem] max-w-[62vw] flex-col border-l border-border bg-background shadow-2xl"
  >
    <!-- Header: title + global Structured/Raw toggle + close -->
    <div class="flex items-center justify-between border-b border-border px-4 py-3">
      <h3 class="text-sm font-semibold">Request {d.seq}</h3>
      <div class="flex items-center gap-2">
        <button
          class="rounded-md border px-2 py-1 text-xs {viewMode === 'raw'
            ? 'border-primary bg-primary/10 text-primary'
            : 'border-border hover:bg-muted'}"
          onclick={() => (viewMode = viewMode === "structured" ? "raw" : "structured")}
          >{viewMode === "structured" ? "Structured" : "Raw"}</button
        >
        <button
          class="rounded-md border border-border px-2 py-1 text-xs hover:bg-muted"
          onclick={closeDetail}>✕</button
        >
      </div>
    </div>

    <!-- Overview / Request / Response tabs -->
    <div class="flex gap-1 border-b border-border px-3 py-2">
      {#each tabs as t}
        <button
          class="rounded-md px-3 py-1 text-xs capitalize {activeTab === t
            ? 'bg-primary/10 text-primary'
            : 'text-muted-foreground hover:bg-muted'}"
          onclick={() => (activeTab = t)}>{t}</button
        >
      {/each}
    </div>

    <div class="flex-1 overflow-y-auto p-4">
      {#if store.detailLoading}
        <div class="text-sm text-muted-foreground">Loading…</div>
      {:else if activeTab === "overview"}
        <OverviewTab record={d} raw={viewMode === "raw"} />
      {:else if activeTab === "request"}
        <RequestTab record={d} raw={viewMode === "raw"} />
      {:else}
        <ResponseTab record={d} raw={viewMode === "raw"} />
      {/if}
    </div>
  </div>
{/if}
