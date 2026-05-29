// These types mirror the actual JSON emitted by the Rust backends:
//   - aggregate daemon : src/daemon/aggregate/routes.rs
//   - single proxy     : ccs-proxy/src/api/routes.rs
// Field names/shapes are authoritative from those handlers — keep them in sync.

/** One request row. Matches `ccs_proxy::store::RequestSummary`. */
export interface RequestSummary {
  seq: number;
  session_id: string;
  request_id: string | null;
  started_at: string;
  duration_ms: number | null;
  model: string | null;
  status: number | null;
  input_tokens: number | null;
  output_tokens: number | null;
  has_error: boolean;
}

/** Full session metadata. Matches `ccs_proxy::store::SessionMeta`. */
export interface SessionMeta {
  session_id: string;
  provider: string;
  upstream: string;
  proxy_port: number;
  api_port: number;
  started_at: string;
  ended_at: string | null;
  request_count: number;
  schema_version: number;
  cwd: string | null;
  models: string[];
}

/**
 * A session row in a list. The aggregate `/api/sessions` returns this exact
 * shape (subset of SessionMeta + `aliases`); the single proxy returns full
 * SessionMeta (a superset), so the extra fields are optional here.
 */
export interface SessionSummary {
  session_id: string;
  provider: string;
  upstream: string | null;
  started_at: string;
  ended_at: string | null;
  request_count: number;
  cwd: string | null;
  models: string[];
  /** Present only on the aggregate endpoint. */
  aliases?: string[];
  proxy_port?: number;
  api_port?: number;
  schema_version?: number;
}

/** `GET /api/sessions/{sid}` — both backends return `{meta, requests}`; the
 * aggregate additionally wraps `upstream` + `aliases`. */
export interface SessionDetail {
  meta: SessionMeta;
  requests: RequestSummary[];
  upstream?: string;
  aliases?: string[];
}

/** Single-proxy `GET /api/health` (ccs-proxy/src/api/routes.rs). */
export interface ProxyHealth {
  status: string;
  provider: string;
  upstream: string;
  session_id: string;
  started_at: string;
  request_count: number;
  store: string;
}

/** One proxy entry in the aggregate health payload. */
export interface AggregateProxy {
  upstream: string;
  request_count: number;
  aliases: string[];
}

/** Aggregate `GET /api/health` (src/daemon/aggregate/routes.rs). */
export interface AggregateHealth {
  status: string;
  pid: number;
  uptime_s: number;
  proxy_count: number;
  total_requests: number;
  proxies: AggregateProxy[];
}

/** Aggregate `GET /api/meta`. Note: no `upstreams` — derive those from stats. */
export interface AggregateMeta {
  models: string[];
  cwds: string[];
}

/** Per-upstream stats row in `GET /api/stats`. */
export interface UpstreamStats {
  upstream: string;
  aliases: string[];
  total_requests: number;
  total_input_tokens: number;
  total_output_tokens: number;
  avg_duration_ms: number;
  error_count: number;
  error_rate: number;
  session_count: number;
}

/** Roll-up totals in `GET /api/stats`. */
export interface StatsTotals {
  total_requests: number;
  total_input_tokens: number;
  total_output_tokens: number;
  avg_duration_ms: number;
  error_count: number;
}

/** Aggregate `GET /api/stats`. */
export interface Stats {
  upstreams: UpstreamStats[];
  totals: StatsTotals;
  since: string | null;
}

/** `GET /api/requests/{sid}/{seq}` — full `ccs_proxy::capture::CaptureRecord`.
 * The conversation lives at `request.body.messages`. */
export interface RequestDetail {
  seq: number;
  session_id: string;
  request_id: string | null;
  started_at: string;
  ended_at: string | null;
  duration_ms: number | null;
  ttft_ms: number | null;
  request: {
    method: string;
    path: string;
    headers: Record<string, string>;
    body: unknown;
  };
  response?: {
    status: number;
    headers: Record<string, string>;
    body_reassembled?: unknown;
    raw_sse_text?: string | null;
    raw_sse_frames_count: number;
  } | null;
  usage?: {
    input_tokens: number;
    output_tokens: number;
    cache_creation_input_tokens: number;
    cache_read_input_tokens: number;
  } | null;
  model: string | null;
  error?: unknown;
  partial: boolean;
  schema_version: number;
  [key: string]: unknown;
}
