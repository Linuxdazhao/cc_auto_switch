<script lang="ts">
  import JsonBlock from "./JsonBlock.svelte";

  let { record, raw }: { record: any; raw: boolean } = $props();

  const fmtMs = (n: number | null | undefined) => (n == null ? "—" : `${n}ms`);
  function fmtTokens(u: any): string {
    if (!u) return "—";
    const parts: string[] = [];
    if (u.input_tokens != null) parts.push(`in=${u.input_tokens}`);
    if (u.output_tokens != null) parts.push(`out=${u.output_tokens}`);
    if (u.cache_creation_input_tokens) parts.push(`cache_create=${u.cache_creation_input_tokens}`);
    if (u.cache_read_input_tokens) parts.push(`cache_read=${u.cache_read_input_tokens}`);
    return parts.join(" · ") || "—";
  }
</script>

{#if raw}
  <JsonBlock value={record} />
{:else}
  <dl class="grid grid-cols-[7rem_1fr] gap-x-4 gap-y-1.5 text-sm">
    <dt class="text-muted-foreground">Session</dt>
    <dd class="break-all font-mono text-xs">{record.session_id}</dd>
    <dt class="text-muted-foreground">Seq</dt>
    <dd>{record.seq}</dd>
    <dt class="text-muted-foreground">Request ID</dt>
    <dd class="break-all font-mono text-xs">{record.request_id || "—"}</dd>
    <dt class="text-muted-foreground">Model</dt>
    <dd>{record.model || "—"}</dd>
    <dt class="text-muted-foreground">Started</dt>
    <dd>{record.started_at}</dd>
    <dt class="text-muted-foreground">Ended</dt>
    <dd>{record.ended_at || "—"}</dd>
    <dt class="text-muted-foreground">Duration</dt>
    <dd>{fmtMs(record.duration_ms)}</dd>
    <dt class="text-muted-foreground">TTFT</dt>
    <dd>{fmtMs(record.ttft_ms)}</dd>
    <dt class="text-muted-foreground">Usage</dt>
    <dd>{fmtTokens(record.usage)}</dd>
    {#if record.partial}
      <dt class="text-muted-foreground">Partial</dt>
      <dd>yes</dd>
    {/if}
  </dl>
  {#if record.error}
    <div class="mt-3 mb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">Error</div>
    <JsonBlock value={record.error} />
  {/if}
{/if}
