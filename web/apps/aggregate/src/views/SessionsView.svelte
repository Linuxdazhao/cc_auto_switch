<script lang="ts">
  import { DataTable, Sheet } from "@ccs/ui";
  import type { SessionSummary, RequestDetail as RequestDetailData } from "@ccs/api";
  import { state as store, client } from "../store.svelte";
  import RequestDetail from "./RequestDetail.svelte";

  let open = $state(false);
  let detail = $state<RequestDetailData | null>(null);

  const cols: { key: keyof SessionSummary & string; label: string; sortable?: boolean }[] = [
    { key: "started_at", label: "Started", sortable: true },
    { key: "session_id", label: "Session", sortable: false },
    { key: "upstream", label: "Upstream", sortable: true },
    { key: "alias", label: "Alias", sortable: true },
    { key: "request_count", label: "Requests", sortable: true },
    { key: "duration_ms", label: "Duration", sortable: true },
  ];

  async function openSession(s: SessionSummary) {
    const d = await client.getSession(s.session_id);
    detail = d.requests[0] ? await client.getRequest(s.session_id, d.requests[0].seq) : null;
    open = true;
  }
</script>

<DataTable columns={cols} rows={store.sessions} onRowClick={openSession}>
  {#snippet row(s: SessionSummary)}
    <td class="px-3 py-2 tabular-nums">{new Date(s.started_at).toLocaleString()}</td>
    <td class="px-3 py-2 font-mono text-xs">{s.session_id.slice(0, 8)}</td>
    <td class="px-3 py-2">{s.upstream ?? "—"}</td>
    <td class="px-3 py-2">{s.alias ?? "—"}</td>
    <td class="px-3 py-2 tabular-nums">{s.request_count}</td>
    <td class="px-3 py-2 tabular-nums">{s.duration_ms ?? "—"}ms</td>
  {/snippet}
</DataTable>

<Sheet.Root bind:open>
  <Sheet.Content side="right" class="w-[40rem] max-w-[90vw] overflow-y-auto">
    <Sheet.Header><Sheet.Title>Request detail</Sheet.Title></Sheet.Header>
    <div class="mt-4"><RequestDetail {detail} /></div>
  </Sheet.Content>
</Sheet.Root>
