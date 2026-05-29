<script lang="ts">
  import Markdown from "./Markdown.svelte";
  import JsonBlock from "./JsonBlock.svelte";
  import ContentBlock from "./ContentBlock.svelte";

  // An Anthropic content block. Loosely typed — the dispatcher branches on `type`.
  let { block }: { block: any } = $props();
</script>

{#if block?.type === "text"}
  <Markdown text={block.text || ""} />
{:else if block?.type === "tool_use"}
  <details class="my-1.5 rounded-md border border-border bg-card" open>
    <summary class="cursor-pointer px-3 py-2 text-xs font-semibold">
      🔧 <span class="font-mono">{block.name}</span>{#if block.id}<span
          class="font-normal text-muted-foreground"> · {block.id}</span
        >{/if}
    </summary>
    <div class="border-t border-border p-2"><JsonBlock value={block.input ?? {}} /></div>
  </details>
{:else if block?.type === "tool_result"}
  <details class="my-1.5 rounded-md border border-border bg-card" open>
    <summary class="cursor-pointer px-3 py-2 text-xs font-semibold">
      ↩ tool_result{#if block.tool_use_id}<span class="font-normal text-muted-foreground">
          · {block.tool_use_id}</span
        >{/if}{#if block.is_error}<span class="font-normal text-danger"> · error</span>{/if}
    </summary>
    <div class="border-t border-border p-2">
      {#if typeof block.content === "string"}
        <Markdown text={block.content} />
      {:else if Array.isArray(block.content)}
        {#each block.content as child, i (i)}<ContentBlock block={child} />{/each}
      {:else}
        <JsonBlock value={block.content ?? null} />
      {/if}
    </div>
  </details>
{:else if block?.type === "thinking"}
  <div class="my-1.5 border-l-2 border-muted-foreground/40 pl-3 italic text-muted-foreground">
    <Markdown text={block.thinking || ""} />
  </div>
{:else if block?.type === "image"}
  <div class="my-1.5 rounded-md border border-dashed border-border px-3 py-2 text-xs text-muted-foreground">
    [image: {block.source?.media_type || "unknown"}]
  </div>
{:else}
  <JsonBlock value={block} />
{/if}
