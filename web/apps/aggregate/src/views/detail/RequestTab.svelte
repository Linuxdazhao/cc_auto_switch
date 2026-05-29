<script lang="ts">
  import SystemSection from "./SystemSection.svelte";
  import ToolsSection from "./ToolsSection.svelte";
  import MessageThread from "./MessageThread.svelte";
  import JsonBlock from "./JsonBlock.svelte";

  let { record, raw }: { record: any; raw: boolean } = $props();

  const body = $derived(record?.request?.body ?? null);
  const isAnthropic = $derived(!!body && Array.isArray(body.messages));
</script>

{#if raw}
  <JsonBlock value={body} />
{:else if isAnthropic}
  <SystemSection system={body.system} />
  <ToolsSection tools={body.tools || []} />
  <div class="my-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
    Messages ({body.messages.length})
  </div>
  <MessageThread messages={body.messages} />
{:else}
  <div class="mb-1 text-xs text-muted-foreground">Non-Anthropic shape — showing raw body.</div>
  <JsonBlock value={body} />
{/if}
