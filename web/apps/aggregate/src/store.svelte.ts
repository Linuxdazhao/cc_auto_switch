import { createClient, type SessionSummary, type RequestSummary, type Stats, type AggregateMeta } from "@ccs/api";

const client = createClient();

export const state = $state({
  view: "requests" as "requests" | "sessions",
  loading: false,
  meta: { upstreams: [], models: [], cwds: [] } as AggregateMeta,
  stats: null as Stats | null,
  sessions: [] as SessionSummary[],
  requests: [] as RequestSummary[],
  filters: { upstreams: [] as string[], models: [] as string[], cwds: [] as string[], window: "1h" as string },
  search: "",
});

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
    const sess = await client.listSessions({ limit: 500 });
    state.sessions = sess.items;
    // requests 列表由 loadRequests() 单独填充（逐会话取详情）
  } finally {
    state.loading = false;
  }
}

export async function loadRequests(): Promise<void> {
  const sess = await client.listSessions({ limit: 200 });
  const all: RequestSummary[] = [];
  for (const s of sess.items) {
    const detail = await client.getSession(s.session_id);
    all.push(...detail.requests);
  }
  state.requests = all;
}

export { client };
