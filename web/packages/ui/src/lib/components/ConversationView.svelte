<script lang="ts">
  import { renderMarkdown } from "../markdown";
  type Message = { role: string; content: string };
  let { messages }: { messages: Message[] } = $props();
  const roleClass: Record<string, string> = {
    user: "bg-muted",
    assistant: "bg-primary/5 border border-primary/20",
    system: "bg-warning/10",
    tool: "bg-card border border-border",
  };
</script>

<div class="space-y-3">
  {#each messages as m}
    <div class="rounded-lg p-3 {roleClass[m.role] ?? 'bg-card'}">
      <div class="mb-1 text-xs font-semibold uppercase text-muted-foreground">{m.role}</div>
      <div class="prose prose-sm max-w-none dark:prose-invert">{@html renderMarkdown(m.content)}</div>
    </div>
  {/each}
</div>
