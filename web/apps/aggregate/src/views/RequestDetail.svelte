<script lang="ts">
  import { ConversationView } from "@ccs/ui";
  import type { RequestDetail } from "@ccs/api";

  let { detail }: { detail: RequestDetail | null } = $props();

  // 从请求/响应体抽取对话消息；形态未知时兜底为空。
  const messages = $derived.by(() => {
    const body = detail?.request_body as { messages?: { role: string; content: unknown }[] } | undefined;
    const msgs = body?.messages ?? [];
    return msgs.map((m) => ({
      role: m.role,
      content: typeof m.content === "string" ? m.content : JSON.stringify(m.content, null, 2),
    }));
  });
</script>

{#if detail}
  <ConversationView {messages} />
{:else}
  <p class="text-muted-foreground">Select a request.</p>
{/if}
