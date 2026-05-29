export interface RequestSummary {
  seq: number;
  started_at: string;
  upstream: string | null;
  model: string | null;
  input_tokens: number | null;
  output_tokens: number | null;
  status: number | null;
  duration_ms: number | null;
  has_error?: boolean;
  cwd?: string | null;
}

export interface SessionSummary {
  session_id: string;
  started_at: string;
  upstream: string | null;
  alias: string | null;
  request_count: number;
  duration_ms: number | null;
}

export interface SessionDetail extends SessionSummary {
  requests: RequestSummary[];
}

export interface ProxyHealth {
  provider: string;
  upstream: string;
  session_id: string;
}

export interface AggregateMeta {
  upstreams: string[];
  models: string[];
  cwds: string[];
}

export interface Stats {
  total_requests: number;
  total_tokens: number;
  input_tokens: number;
  output_tokens: number;
  error_count: number;
}

/** /api/requests/{sid}/{seq} 返回完整记录；形态由后端决定，按需读取字段。 */
export type RequestDetail = Record<string, unknown> & {
  seq: number;
  session_id: string;
  request_body?: unknown;
  response_body?: unknown;
  request_headers?: Record<string, string>;
  response_headers?: Record<string, string>;
};
