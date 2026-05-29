<script lang="ts">
  let {
    title,
    options,
    selected = $bindable([]),
  }: {
    title: string;
    options: string[];
    selected: string[];
  } = $props();
  let open = $state(true);

  function toggle(opt: string) {
    selected = selected.includes(opt)
      ? selected.filter((s) => s !== opt)
      : [...selected, opt];
  }
</script>

<section class="border-b border-border py-2">
  <button class="flex w-full items-center justify-between text-xs font-semibold uppercase tracking-wide text-muted-foreground"
          onclick={() => (open = !open)}>
    <span>{title}</span><span>{open ? "−" : "+"}</span>
  </button>
  {#if open}
    <div class="mt-2 flex flex-wrap gap-1">
      {#each options as opt}
        <button
          class="rounded-md border px-2 py-0.5 text-xs {selected.includes(opt) ? 'border-primary bg-primary/10 text-primary' : 'border-border text-muted-foreground'}"
          onclick={() => toggle(opt)}>{opt}</button>
      {/each}
    </div>
  {/if}
</section>
