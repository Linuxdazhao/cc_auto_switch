<script lang="ts">
  import ContentBlock from "./ContentBlock.svelte";
  import JsonBlock from "./JsonBlock.svelte";

  let { record, raw }: { record: any; raw: boolean } = $props();

  const response = $derived(record?.response ?? null);
  const reassembled = $derived(response?.body_reassembled ?? null);
  const content = $derived(Array.isArray(reassembled?.content) ? reassembled.content : null);
  const stopReason = $derived(reassembled?.stop_reason ?? null);
  const respUsage = $derived(reassembled?.usage ?? null);
  const rawSse = $derived(response?.raw_sse_text ?? null);

  const preClass =
    "max-h-[50vh] overflow-auto rounded-md border border-border bg-muted/40 p-3 text-xs font-mono whitespace-pre-wrap break-words";
</script>

{#if !response}
  <div class="text-sm text-muted-foreground">No response captured.</div>
{:else if raw}
  {#if reassembled}
    <JsonBlock value={reassembled} />
  {:else if rawSse}
    <pre class={preClass}>{rawSse}</pre>
  {:else}
    <div class="text-sm text-muted-foreground">Empty response body.</div>
  {/if}
{:else if content}
  <div class="my-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
    Content blocks ({content.length})
  </div>
  <div class="rounded-md border border-border border-l-[3px] border-l-success p-3">
    <span class="mb-2 inline-block rounded bg-success/15 px-1.5 py-0.5 text-[10px] font-semibold uppercase text-success">
      assistant
    </span>
    {#each content as block, i (i)}<ContentBlock {block} />{/each}
  </div>
  <dl class="mt-3 grid grid-cols-[7rem_1fr] gap-x-4 gap-y-1.5 text-sm">
    <dt class="text-muted-foreground">Status</dt>
    <dd>{response.status}</dd>
    <dt class="text-muted-foreground">Stop reason</dt>
    <dd>{stopReason || "—"}</dd>
    <dt class="text-muted-foreground">SSE frames</dt>
    <dd>{response.raw_sse_frames_count}</dd>
  </dl>
  {#if respUsage}
    <div class="mt-3 mb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">Response usage</div>
    <JsonBlock value={respUsage} />
  {/if}
{:else if reassembled}
  <div class="mb-1 text-xs text-muted-foreground">Non-Anthropic response shape — showing raw body.</div>
  <JsonBlock value={reassembled} />
{:else if rawSse}
  <div class="mb-1 text-xs text-muted-foreground">No reassembled body — showing raw SSE.</div>
  <pre class={preClass}>{rawSse}</pre>
{:else}
  <div class="text-sm text-muted-foreground">Empty response body.</div>
{/if}
