<script lang="ts">
  import { onMount } from "svelte";
  import { createClient, type RequestSummary, type ProxyHealth } from "@ccs/api";
  import { DataTable, StatusBadge, Sheet, ConversationView } from "@ccs/ui";

  const client = createClient();
  let health = $state<ProxyHealth | null>(null);
  let requests = $state<RequestSummary[]>([]);
  let open = $state(false);
  let messages = $state<{ role: string; content: string }[]>([]);

  const cols = [
    { key: "started_at", label: "Time", sortable: true },
    { key: "model", label: "Model", sortable: true },
    { key: "status", label: "Status", sortable: true },
    { key: "duration_ms", label: "Duration", sortable: true },
  ] as const;

  onMount(async () => {
    health = await client.health();
    const d = await client.getSession(health.session_id);
    requests = d.requests;
  });

  async function openRow(r: RequestSummary) {
    if (!health) return;
    const detail = await client.getRequest(health.session_id, r.seq);
    const body = detail.request?.body as { messages?: { role: string; content: unknown }[] } | undefined;
    messages = (body?.messages ?? []).map((m) => ({
      role: m.role,
      content: typeof m.content === "string" ? m.content : JSON.stringify(m.content, null, 2),
    }));
    open = true;
  }
</script>

<div class="flex h-screen flex-col">
  <header class="border-b border-border px-4 py-3 text-sm">
    {#if health}<span class="font-mono">{health.provider} · {health.upstream} · {health.session_id.slice(0, 8)}</span>{/if}
  </header>
  <main class="flex-1 overflow-y-auto p-4">
    <DataTable columns={[...cols]} rows={requests} onRowClick={openRow}>
      {#snippet row(r: RequestSummary)}
        <td class="px-3 py-2 tabular-nums">{new Date(r.started_at).toLocaleTimeString()}</td>
        <td class="px-3 py-2">{r.model ?? "—"}</td>
        <td class="px-3 py-2"><StatusBadge status={r.status} /></td>
        <td class="px-3 py-2 tabular-nums">{r.duration_ms ?? "—"}ms</td>
      {/snippet}
    </DataTable>
  </main>
</div>

<Sheet.Root bind:open>
  <Sheet.Content side="right" class="w-[40rem] max-w-[90vw] overflow-y-auto">
    <Sheet.Header><Sheet.Title>Request detail</Sheet.Title></Sheet.Header>
    <div class="mt-4"><ConversationView {messages} /></div>
  </Sheet.Content>
</Sheet.Root>
