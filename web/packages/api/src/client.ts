import type {
  SessionSummary, SessionDetail, RequestDetail,
  ProxyHealth, AggregateMeta, Stats,
} from "./types";

export interface ClientOptions {
  baseUrl?: string;
  fetch?: typeof fetch;
}

export interface ListSessionsParams {
  limit?: number;
  query?: Record<string, string>;
}

export interface ApiClient {
  health(): Promise<ProxyHealth>;
  meta(): Promise<AggregateMeta>;
  stats(since?: string): Promise<Stats>;
  listSessions(params?: ListSessionsParams): Promise<{ items: SessionSummary[]; total?: number }>;
  getSession(sid: string): Promise<SessionDetail>;
  getRequest(sid: string, seq: number): Promise<RequestDetail>;
}

export function createClient(opts: ClientOptions = {}): ApiClient {
  const base = opts.baseUrl ?? "";
  const f = opts.fetch ?? fetch;

  async function get<T>(path: string): Promise<T> {
    const resp = await f(`${base}${path}`, { headers: { Accept: "application/json" } });
    if (!resp.ok) throw new Error(`request ${path} failed: ${resp.status}`);
    return (await resp.json()) as T;
  }

  function qs(params: Record<string, string | number | undefined>): string {
    const parts = Object.entries(params)
      .filter(([, v]) => v !== undefined && v !== "")
      .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(String(v))}`);
    return parts.length ? `?${parts.join("&")}` : "";
  }

  return {
    health: () => get("/api/health"),
    meta: () => get("/api/meta"),
    stats: (since) => get(`/api/stats${qs({ since })}`),
    listSessions: (p = {}) =>
      get(`/api/sessions${qs({ limit: p.limit, ...(p.query ?? {}) })}`),
    getSession: (sid) => get(`/api/sessions/${encodeURIComponent(sid)}`),
    getRequest: (sid, seq) => get(`/api/requests/${encodeURIComponent(sid)}/${seq}`),
  };
}

export interface SseEvent {
  event: string;
  data: unknown;
}

export function parseSseEvent(chunk: string): SseEvent | null {
  const lines = chunk.split("\n");
  let event = "message";
  let data = "";
  for (const line of lines) {
    if (line.startsWith(":")) continue;
    if (line.startsWith("event:")) event = line.slice(6).trim();
    else if (line.startsWith("data:")) data += line.slice(5).trim();
  }
  if (!data) return null;
  try {
    return { event, data: JSON.parse(data) };
  } catch {
    return { event, data };
  }
}
