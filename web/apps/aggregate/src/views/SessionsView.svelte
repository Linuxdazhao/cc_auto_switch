<script lang="ts">
  import { DataTable } from "@ccs/ui";
  import type { SessionSummary } from "@ccs/api";
  import { state as store, openSession } from "../store.svelte";

  const cols: { key: keyof SessionSummary & string; label: string; sortable?: boolean }[] = [
    { key: "started_at", label: "Started", sortable: true },
    { key: "session_id", label: "Session", sortable: false },
    { key: "upstream", label: "Upstream", sortable: true },
    { key: "aliases", label: "Aliases", sortable: false },
    { key: "request_count", label: "Requests", sortable: true },
    { key: "models", label: "Models", sortable: false },
  ];
</script>

<DataTable columns={cols} rows={store.sessions} onRowClick={(s: SessionSummary) => openSession(s.session_id)}>
  {#snippet row(s: SessionSummary)}
    <td class="px-3 py-2 tabular-nums">{new Date(s.started_at).toLocaleString()}</td>
    <td class="px-3 py-2 font-mono text-xs">{s.session_id.slice(0, 8)}</td>
    <td class="px-3 py-2">{s.upstream ?? "—"}</td>
    <td class="px-3 py-2">{(s.aliases ?? []).join(", ") || "—"}</td>
    <td class="px-3 py-2 tabular-nums">{s.request_count}</td>
    <td class="px-3 py-2">{(s.models ?? []).join(", ") || "—"}</td>
  {/snippet}
</DataTable>
