<script lang="ts">
  import { DataTable, StatusBadge, Input } from "@ccs/ui";
  import { state as store, openRequest, type RequestRow } from "../store.svelte";

  const cols: { key: keyof RequestRow & string; label: string; sortable?: boolean }[] = [
    { key: "started_at", label: "Time", sortable: true },
    { key: "upstream", label: "Upstream", sortable: true },
    { key: "model", label: "Model", sortable: true },
    { key: "input_tokens", label: "Tokens", sortable: true },
    { key: "status", label: "Status", sortable: true },
    { key: "duration_ms", label: "Duration", sortable: true },
  ];

  const filtered = $derived(
    store.requests.filter((r) => {
      const q = store.search.toLowerCase();
      if (q && !`${r.model} ${r.upstream}`.toLowerCase().includes(q)) return false;
      if (store.filters.upstreams.length && !store.filters.upstreams.includes(r.upstream ?? "")) return false;
      if (store.filters.models.length && !store.filters.models.includes(r.model ?? "")) return false;
      return true;
    }),
  );

  function fmtTime(s: string) {
    return new Date(s).toLocaleTimeString();
  }
</script>

<div class="mb-3"><Input placeholder="Search model / upstream…" bind:value={store.search} /></div>
<DataTable columns={cols} rows={filtered} onRowClick={(r: RequestRow) => openRequest(r.session_id, r.seq)}>
  {#snippet row(r: RequestRow)}
    <td class="px-3 py-2 tabular-nums">{fmtTime(r.started_at)}</td>
    <td class="px-3 py-2">{r.upstream ?? "—"}</td>
    <td class="px-3 py-2">{r.model ?? "—"}</td>
    <td class="px-3 py-2 tabular-nums">{(r.input_tokens ?? 0) + (r.output_tokens ?? 0)}</td>
    <td class="px-3 py-2"><StatusBadge status={r.status} /></td>
    <td class="px-3 py-2 tabular-nums">{r.duration_ms ?? "—"}ms</td>
  {/snippet}
</DataTable>
