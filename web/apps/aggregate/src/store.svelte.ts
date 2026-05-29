import { createClient, type SessionSummary, type RequestSummary, type Stats, type AggregateMeta, type RequestDetail } from "@ccs/api";

const client = createClient();

/** A request row in the flat Requests view: the per-session RequestSummary
 * enriched with the owning session's upstream (the API summary has no upstream). */
export type RequestRow = RequestSummary & { upstream: string | null };

export const state = $state({
  view: "requests" as "requests" | "sessions",
  loading: false,
  meta: { models: [], cwds: [] } as AggregateMeta,
  /** Derived from stats (the backend has no `/api/meta` upstreams field). */
  upstreams: [] as string[],
  stats: null as Stats | null,
  sessions: [] as SessionSummary[],
  requests: [] as RequestRow[],
  filters: { upstreams: [] as string[], models: [] as string[], cwds: [] as string[], window: "all" as string },
  search: "",
  /** Full record shown in the right-hand detail drawer (null = closed). */
  selected: null as RequestDetail | null,
  detailLoading: false,
});

/** Open the detail drawer for a single captured request. */
export async function openRequest(sid: string, seq: number): Promise<void> {
  state.detailLoading = true;
  try {
    state.selected = await client.getRequest(sid, seq);
  } finally {
    state.detailLoading = false;
  }
}

/** Open the first request of a session in the detail drawer. */
export async function openSession(sid: string): Promise<void> {
  const d = await client.getSession(sid);
  if (d.requests[0]) await openRequest(sid, d.requests[0].seq);
}

export function closeDetail(): void {
  state.selected = null;
}

function sinceFromWindow(w: string): string | undefined {
  if (w === "all") return undefined;
  const now = Date.now();
  const ms = w === "1h" ? 3.6e6 : w === "24h" ? 8.64e7 : w === "7d" ? 6.048e8 : 0;
  return ms ? new Date(now - ms).toISOString() : undefined;
}

export async function loadAll(): Promise<void> {
  state.loading = true;
  try {
    state.meta = await client.meta();
    state.stats = await client.stats(sinceFromWindow(state.filters.window));
    state.upstreams = state.stats.upstreams.map((u) => u.upstream);
    state.sessions = await client.listSessions({ limit: 500 });
    // requests 列表由 loadRequests() 单独填充（逐会话取详情）
  } finally {
    state.loading = false;
  }
}

export async function loadRequests(): Promise<void> {
  const sessions = await client.listSessions({ limit: 200 });
  const all: RequestRow[] = [];
  for (const s of sessions) {
    const detail = await client.getSession(s.session_id);
    for (const r of detail.requests) {
      all.push({ ...r, upstream: s.upstream });
    }
  }
  state.requests = all;
}

export { client };
