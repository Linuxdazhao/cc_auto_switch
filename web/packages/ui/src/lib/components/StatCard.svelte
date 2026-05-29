<script lang="ts">
  let { label, value, hint }: { label: string; value: string | number; hint?: string } = $props();
</script>

<div class="rounded-lg border border-border bg-card p-4">
  <div class="text-xs uppercase tracking-wide text-muted-foreground">{label}</div>
  <div class="mt-1 text-2xl font-semibold tabular-nums">{value}</div>
  {#if hint}<div class="mt-1 text-xs text-muted-foreground">{hint}</div>{/if}
</div>
