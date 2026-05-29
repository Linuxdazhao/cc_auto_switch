<script lang="ts">
  let { value }: { value: unknown } = $props();
  const text = $derived.by(() => {
    try {
      return JSON.stringify(value, null, 2);
    } catch {
      return String(value);
    }
  });
</script>

<pre
  class="max-h-[50vh] overflow-auto rounded-md border border-border bg-muted/40 p-3 text-xs font-mono leading-relaxed whitespace-pre-wrap break-words"><code
  >{text}</code></pre>
