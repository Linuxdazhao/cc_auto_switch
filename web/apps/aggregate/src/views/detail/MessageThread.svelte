<script lang="ts">
  import Markdown from "./Markdown.svelte";
  import JsonBlock from "./JsonBlock.svelte";
  import ContentBlock from "./ContentBlock.svelte";

  let { messages }: { messages: any[] } = $props();

  const border: Record<string, string> = {
    user: "border-l-primary",
    assistant: "border-l-success",
    system: "border-l-muted-foreground",
  };
  const badge: Record<string, string> = {
    user: "bg-primary/15 text-primary",
    assistant: "bg-success/15 text-success",
    system: "bg-muted text-muted-foreground",
  };
</script>

<div class="space-y-2">
  {#each messages as m, i (i)}
    {@const role = m?.role || "unknown"}
    <div class="rounded-md border border-border border-l-[3px] {border[role] ?? 'border-l-border'} p-3">
      <span
        class="mb-2 inline-block rounded px-1.5 py-0.5 text-[10px] font-semibold uppercase {badge[role] ??
          'bg-muted text-muted-foreground'}">{role}</span
      >
      {#if typeof m.content === "string"}
        <Markdown text={m.content} />
      {:else if Array.isArray(m.content)}
        {#each m.content as block, j (j)}<ContentBlock {block} />{/each}
      {:else}
        <JsonBlock value={m.content ?? null} />
      {/if}
    </div>
  {/each}
</div>
