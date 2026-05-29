<script lang="ts">
  import ContentBlock from "./ContentBlock.svelte";

  let { system = null }: { system?: unknown } = $props();

  const blocks = $derived.by(() => {
    if (system == null) return [] as any[];
    if (typeof system === "string") return [{ type: "text", text: system }];
    if (Array.isArray(system)) return system;
    return [] as any[];
  });
</script>

{#if blocks.length}
  <details class="my-2 rounded-md border border-border" open>
    <summary class="cursor-pointer px-3 py-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
      System
    </summary>
    <div class="p-2">
      {#each blocks as b, i (i)}<ContentBlock block={b} />{/each}
    </div>
  </details>
{/if}
