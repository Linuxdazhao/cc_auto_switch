<script lang="ts">
  import JsonBlock from "./JsonBlock.svelte";

  let { tools = [] }: { tools?: any[] } = $props();
</script>

{#if tools && tools.length}
  <details class="my-2 rounded-md border border-border">
    <summary class="cursor-pointer px-3 py-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
      Tools ({tools.length})
    </summary>
    <div class="space-y-2 p-2">
      {#each tools as tool, i (i)}
        <div class="rounded border border-border p-2">
          <strong class="font-mono text-sm">{tool.name}</strong>
          {#if tool.description}<div class="mt-1 text-xs text-muted-foreground">{tool.description}</div>{/if}
          {#if tool.input_schema}
            <details class="mt-1">
              <summary class="cursor-pointer text-xs text-muted-foreground">input_schema</summary>
              <JsonBlock value={tool.input_schema} />
            </details>
          {/if}
        </div>
      {/each}
    </div>
  </details>
{/if}
