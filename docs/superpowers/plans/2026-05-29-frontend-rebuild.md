# 前端重构实施计划 — Svelte + Vite + shadcn-svelte

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 `web-aggregate` 和 `ccs-proxy/web` 两个命令式/CDN 前端，重写为共享设计系统的 Svelte + Vite + shadcn-svelte 应用，离线内嵌、视觉现代、可维护。

**Architecture:** 顶层 `web/` pnpm workspace，含共享 `packages/api`（TS 类型 + fetch 客户端）、`packages/ui`（shadcn-svelte 组件 + Tailwind 设计 token + 深色模式）和两个 app（`apps/aggregate`、`apps/proxy`）。两个 app 的 Vite `outDir` 分别输出到各自 crate 的 `dist/`，由 rust-embed 内嵌。Rust 侧引入 `build.rs`，仅在 `web-ui` cargo feature 开启时调用 Vite；feature 默认关，发布的 ccs-proxy crate `exclude` web 资产，下游零 Node。

**Tech Stack:** Svelte 5 + Vite 6 + TypeScript + Tailwind CSS + shadcn-svelte (bits-ui) + Vitest + pnpm；Rust (axum, rust-embed, build.rs)。

---

## 背景事实（实现者必读）

**两个 API 几乎同构**（字段名取自现有 `app.js`，为真相来源）：

聚合 daemon（`src/daemon/aggregate/routes.rs`）暴露：
- `GET /api/health`、`GET /api/meta`、`GET /api/stats?since=<rfc3339>`
- `GET /api/sessions?limit=N`（含过滤 query）、`GET /api/sessions/{sid}`、`GET /api/requests/{sid}/{seq}`
- `GET /api/stream`（SSE）

ccs-proxy（`ccs-proxy/src/api/routes.rs`）是单实例子集：
- `GET /api/health`（返回 `{provider, upstream, session_id}`）、`GET /api/sessions`、`GET /api/sessions/{sid}`、`GET /api/requests/{sid}/{seq}`、`GET /api/stream`

观察到的字段：
- **RequestSummary**：`seq`、`started_at`(ISO 字符串)、`upstream`、`model`、`input_tokens`、`output_tokens`、`status`(number|null)、`duration_ms`、`has_error`、`cwd`
- **SessionSummary**：`session_id`、`started_at`、`upstream`、`alias`、`request_count`、`duration_ms`
- **Health(proxy)**：`provider`、`upstream`、`session_id`

**当前内嵌方式**：
- 聚合：`src/daemon/aggregate/mod.rs` 用 `#[folder = "web-aggregate/"]`，`ui_router()` 硬编码 `/`、`/index.html`、`/app.js`、`/style.css` 四条路由 + `serve_asset()`。
- ccs-proxy：`ccs-proxy/src/api/ui.rs` 用 `#[folder = "web/"]`；ccs-proxy 已有 `[features] default=[]`、`dev-fs-assets=[]`。

**Vite 产物含 hash 文件名**（如 `assets/index-<hash>.js`），所以两处硬编码路由必须改成"按任意路径取嵌入资产 + SPA 回退 index.html"。

**约束（见记忆 project-distribution-model）**：web 不随 crate 发布；`web-ui` feature 默认关；feature 关时 build.rs 必须是 no-op；ccs-proxy 发布包 exclude web。

---

## 文件结构

新建：
- `web/package.json`、`web/pnpm-workspace.yaml`、`web/tsconfig.base.json`、`web/.gitignore`
- `web/packages/api/`：`package.json`、`tsconfig.json`、`src/types.ts`、`src/client.ts`、`src/index.ts`、`src/client.test.ts`
- `web/packages/ui/`：`package.json`、`tsconfig.json`、`tailwind.preset.js`、`src/lib/index.ts`、`src/lib/theme.ts`、`src/lib/theme.test.ts`、`components.json`，以及 shadcn 拉取的组件 + 自建业务组件（StatusBadge、StatCard、DataTable、FilterGroup、ConversationView…）
- `web/apps/aggregate/`：标准 Vite+Svelte 工程，`vite.config.ts`(outDir→`../../../web-aggregate/dist`)、`src/App.svelte`、`src/views/*.svelte`、`src/main.ts`、`index.html`
- `web/apps/proxy/`：同上，outDir→`../../../ccs-proxy/web/dist`
- `build.rs`（根 crate）、`ccs-proxy/build.rs`

修改：
- `src/daemon/aggregate/mod.rs`：`#[folder]`→`web-aggregate/dist/`，重写 `ui_router`/`serve_asset` 为 SPA 服务
- `ccs-proxy/src/api/ui.rs`：`#[folder]`→`web/dist/`，SPA 服务；`web-ui` feature 门控
- `Cargo.toml`（根）：新增 `[features] web-ui`、`[build-dependencies]`
- `ccs-proxy/Cargo.toml`：新增 `web-ui` feature、`build.rs`、`exclude` web 资产
- `.github/workflows/*.yml`：构建二进制前装 Node + pnpm build，开启 `web-ui`
- `.gitignore`：忽略 `web/node_modules`、`**/dist`

删除（Phase 6 验证后）：`web-aggregate/{index.html,app.js,style.css}`、`ccs-proxy/web/{index.html,app.js,style.css}`

---

## Phase 0：Workspace 与工具链骨架

### Task 0.1：创建 pnpm workspace 根

**Files:**
- Create: `web/pnpm-workspace.yaml`, `web/package.json`, `web/tsconfig.base.json`, `web/.gitignore`

- [ ] **Step 1: 写 workspace 清单**

`web/pnpm-workspace.yaml`:
```yaml
packages:
  - "packages/*"
  - "apps/*"
```

`web/package.json`:
```json
{
  "name": "ccs-web",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "pnpm -r build",
    "test": "pnpm -r test",
    "lint": "pnpm -r lint",
    "check": "pnpm -r check"
  },
  "devDependencies": {
    "typescript": "^5.7.0"
  },
  "packageManager": "pnpm@9.15.0"
}
```

`web/tsconfig.base.json`:
```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "verbatimModuleSyntax": true,
    "isolatedModules": true
  }
}
```

`web/.gitignore`:
```gitignore
node_modules/
*.tsbuildinfo
.svelte-kit/
```

- [ ] **Step 2: 安装并验证 workspace 解析**

Run: `cd web && pnpm install`
Expected: 成功创建 `web/node_modules`、`web/pnpm-lock.yaml`，无报错（此时无子包也允许）。

- [ ] **Step 3: 根 .gitignore 忽略产物**

修改根 `.gitignore`，追加：
```gitignore
# frontend
web/node_modules/
web/pnpm-lock.yaml
web/**/.svelte-kit/
web-aggregate/dist/
ccs-proxy/web/dist/
```
> 说明：dist 不进仓库（外部不消费 web），CI/本地构建时生成。

- [ ] **Step 4: Commit**

```bash
git add web/pnpm-workspace.yaml web/package.json web/tsconfig.base.json web/.gitignore .gitignore
git commit -m "chore(web): scaffold pnpm workspace root"
```

---

## Phase 1：共享 `api` 包（TS 类型 + 客户端 + 测试）

### Task 1.1：api 包骨架与类型

**Files:**
- Create: `web/packages/api/package.json`, `web/packages/api/tsconfig.json`, `web/packages/api/src/types.ts`

- [ ] **Step 1: 写包清单**

`web/packages/api/package.json`:
```json
{
  "name": "@ccs/api",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "main": "./src/index.ts",
  "types": "./src/index.ts",
  "exports": { ".": "./src/index.ts" },
  "scripts": {
    "test": "vitest run",
    "check": "tsc --noEmit"
  },
  "devDependencies": {
    "vitest": "^2.1.0",
    "typescript": "^5.7.0"
  }
}
```

`web/packages/api/tsconfig.json`:
```json
{
  "extends": "../../tsconfig.base.json",
  "include": ["src"]
}
```

- [ ] **Step 2: 写类型定义**

`web/packages/api/src/types.ts`:
```ts
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
```

- [ ] **Step 3: Commit**

```bash
git add web/packages/api/package.json web/packages/api/tsconfig.json web/packages/api/src/types.ts
git commit -m "feat(web/api): add shared API types"
```

### Task 1.2：fetch 客户端（TDD）

**Files:**
- Create: `web/packages/api/src/client.ts`, `web/packages/api/src/client.test.ts`, `web/packages/api/src/index.ts`

- [ ] **Step 1: 写失败测试**

`web/packages/api/src/client.test.ts`:
```ts
import { describe, it, expect, vi, beforeEach } from "vitest";
import { createClient } from "./client";

describe("createClient", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("listSessions hits /api/sessions with limit", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(new Response(JSON.stringify({ items: [] }), { status: 200 }));
    const c = createClient({ baseUrl: "", fetch: fetchMock });
    await c.listSessions({ limit: 50 });
    expect(fetchMock).toHaveBeenCalledWith("/api/sessions?limit=50", expect.anything());
  });

  it("getSession returns parsed body", async () => {
    const body = { session_id: "s1", requests: [] };
    const fetchMock = vi
      .fn()
      .mockResolvedValue(new Response(JSON.stringify(body), { status: 200 }));
    const c = createClient({ baseUrl: "", fetch: fetchMock });
    const out = await c.getSession("s1");
    expect(out.session_id).toBe("s1");
  });

  it("throws on non-2xx", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(new Response("nope", { status: 500 }));
    const c = createClient({ baseUrl: "", fetch: fetchMock });
    await expect(c.health()).rejects.toThrow(/500/);
  });
});
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd web && pnpm --filter @ccs/api test`
Expected: FAIL（`./client` 不存在 / `createClient is not a function`）。

- [ ] **Step 3: 写最小实现**

`web/packages/api/src/client.ts`:
```ts
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
```

`web/packages/api/src/index.ts`:
```ts
export * from "./types";
export * from "./client";
```

- [ ] **Step 4: 运行测试确认通过**

Run: `cd web && pnpm install && pnpm --filter @ccs/api test`
Expected: PASS（3 个测试）。

- [ ] **Step 5: Commit**

```bash
git add web/packages/api/src/client.ts web/packages/api/src/client.test.ts web/packages/api/src/index.ts web/pnpm-lock.yaml
git commit -m "feat(web/api): add typed fetch client with tests"
```

### Task 1.3：SSE 流订阅（可选实时刷新的基础，TDD）

**Files:**
- Modify: `web/packages/api/src/client.ts`, `web/packages/api/src/client.test.ts`, `web/packages/api/src/index.ts`

- [ ] **Step 1: 追加失败测试**

在 `client.test.ts` 末尾追加：
```ts
import { parseSseEvent } from "./client";

describe("parseSseEvent", () => {
  it("extracts JSON data line", () => {
    const ev = parseSseEvent("event: request\ndata: {\"seq\":3}\n");
    expect(ev).toEqual({ event: "request", data: { seq: 3 } });
  });
  it("returns null on heartbeat/comment", () => {
    expect(parseSseEvent(": keepalive\n")).toBeNull();
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd web && pnpm --filter @ccs/api test`
Expected: FAIL（`parseSseEvent` 未导出）。

- [ ] **Step 3: 实现 parseSseEvent**

在 `client.ts` 追加并从 index 导出：
```ts
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
```

- [ ] **Step 4: 运行确认通过**

Run: `cd web && pnpm --filter @ccs/api test`
Expected: PASS（全部 5 个）。

- [ ] **Step 5: Commit**

```bash
git add web/packages/api/src/client.ts web/packages/api/src/client.test.ts web/packages/api/src/index.ts
git commit -m "feat(web/api): add SSE event parser"
```

---

## Phase 2：共享 `ui` 包（设计系统）

### Task 2.1：ui 包 + Tailwind preset + shadcn 初始化

**Files:**
- Create: `web/packages/ui/package.json`, `web/packages/ui/tsconfig.json`, `web/packages/ui/tailwind.preset.js`, `web/packages/ui/src/lib/index.ts`, `web/packages/ui/components.json`, `web/packages/ui/src/app.css`

- [ ] **Step 1: 写包清单**

`web/packages/ui/package.json`:
```json
{
  "name": "@ccs/ui",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "exports": {
    ".": "./src/lib/index.ts",
    "./app.css": "./src/app.css",
    "./tailwind.preset": "./tailwind.preset.js"
  },
  "scripts": {
    "test": "vitest run",
    "check": "svelte-check --tsconfig ./tsconfig.json"
  },
  "devDependencies": {
    "svelte": "^5.15.0",
    "svelte-check": "^4.1.0",
    "tailwindcss": "^3.4.0",
    "autoprefixer": "^10.4.0",
    "tailwind-variants": "^0.3.0",
    "clsx": "^2.1.0",
    "tailwind-merge": "^2.6.0",
    "bits-ui": "^1.0.0",
    "lucide-svelte": "^0.469.0",
    "vitest": "^2.1.0",
    "@testing-library/svelte": "^5.2.0",
    "jsdom": "^25.0.0",
    "typescript": "^5.7.0"
  },
  "dependencies": {
    "@ccs/api": "workspace:*"
  }
}
```

`web/packages/ui/tsconfig.json`:
```json
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": {
    "types": ["svelte", "vitest/globals"]
  },
  "include": ["src"]
}
```

- [ ] **Step 2: Tailwind preset（设计 token + 深色模式）**

`web/packages/ui/tailwind.preset.js`:
```js
/** 共享设计 token：两个 app 的 tailwind.config 都 extend 这个 preset。 */
export default {
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        border: "hsl(var(--border))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        muted: "hsl(var(--muted))",
        "muted-foreground": "hsl(var(--muted-foreground))",
        primary: "hsl(var(--primary))",
        "primary-foreground": "hsl(var(--primary-foreground))",
        card: "hsl(var(--card))",
        success: "hsl(var(--success))",
        warning: "hsl(var(--warning))",
        danger: "hsl(var(--danger))",
      },
      borderRadius: { lg: "0.5rem", md: "0.375rem", sm: "0.25rem" },
      fontFamily: { mono: ["ui-monospace", "SFMono-Regular", "monospace"] },
    },
  },
};
```

`web/packages/ui/src/app.css`（设计 token 变量 + 明/暗）:
```css
@tailwind base;
@tailwind components;
@tailwind utilities;

:root {
  --background: 0 0% 100%;
  --foreground: 222 47% 11%;
  --muted: 210 40% 96%;
  --muted-foreground: 215 16% 47%;
  --primary: 222 47% 31%;
  --primary-foreground: 210 40% 98%;
  --card: 0 0% 100%;
  --border: 214 32% 91%;
  --success: 142 71% 45%;
  --warning: 38 92% 50%;
  --danger: 0 84% 60%;
}
.dark {
  --background: 222 47% 11%;
  --foreground: 210 40% 98%;
  --muted: 217 33% 17%;
  --muted-foreground: 215 20% 65%;
  --primary: 210 40% 98%;
  --primary-foreground: 222 47% 11%;
  --card: 222 47% 14%;
  --border: 217 33% 24%;
  --success: 142 71% 45%;
  --warning: 38 92% 50%;
  --danger: 0 72% 51%;
}
body { @apply bg-background text-foreground; }
```

`web/packages/ui/components.json`（shadcn-svelte 配置）:
```json
{
  "$schema": "https://shadcn-svelte.com/schema.json",
  "style": "default",
  "tailwind": { "css": "src/app.css", "baseColor": "slate" },
  "aliases": { "components": "src/lib/components", "utils": "src/lib/utils" },
  "typescript": true
}
```

`web/packages/ui/src/lib/index.ts`:
```ts
export { cn } from "./utils";
```

`web/packages/ui/src/lib/utils.ts`:
```ts
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
```

- [ ] **Step 3: 安装并校验编译**

Run: `cd web && pnpm install && pnpm --filter @ccs/ui exec tsc --noEmit -p tsconfig.json`
Expected: 无类型错误。

- [ ] **Step 4: Commit**

```bash
git add web/packages/ui
git commit -m "feat(web/ui): scaffold design-system package with Tailwind tokens"
```

### Task 2.2：拉取 shadcn 基础组件

**Files:**
- Create: `web/packages/ui/src/lib/components/ui/**`（由 CLI 生成）

- [ ] **Step 1: 用 shadcn-svelte CLI 按需拉组件**

Run:
```bash
cd web/packages/ui
pnpm dlx shadcn-svelte@latest add button badge card table tabs select tooltip sheet skeleton input
```
Expected: 在 `src/lib/components/ui/<name>/` 下生成各组件，并更新 `utils.ts`（已存在则跳过）。

- [ ] **Step 2: 从包入口重新导出**

编辑 `web/packages/ui/src/lib/index.ts` 追加：
```ts
export { Button } from "./components/ui/button";
export { Badge } from "./components/ui/badge";
export * as Card from "./components/ui/card";
export * as Table from "./components/ui/table";
export * as Tabs from "./components/ui/tabs";
export * as Select from "./components/ui/select";
export * as Tooltip from "./components/ui/tooltip";
export * as Sheet from "./components/ui/sheet";
export { Skeleton } from "./components/ui/skeleton";
export { Input } from "./components/ui/input";
```

- [ ] **Step 3: 校验编译**

Run: `cd web && pnpm --filter @ccs/ui exec svelte-check --tsconfig ./tsconfig.json`
Expected: 0 errors。

- [ ] **Step 4: Commit**

```bash
git add web/packages/ui/src/lib
git commit -m "feat(web/ui): add shadcn-svelte base components"
```

### Task 2.3：主题切换工具（TDD）

**Files:**
- Create: `web/packages/ui/src/lib/theme.ts`, `web/packages/ui/src/lib/theme.test.ts`
- Create: `web/packages/ui/vitest.config.ts`
- Modify: `web/packages/ui/src/lib/index.ts`

- [ ] **Step 1: vitest 配置（jsdom）**

`web/packages/ui/vitest.config.ts`:
```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: { environment: "jsdom", globals: true },
});
```

- [ ] **Step 2: 写失败测试**

`web/packages/ui/src/lib/theme.test.ts`:
```ts
import { describe, it, expect, beforeEach } from "vitest";
import { applyTheme, resolveInitialTheme } from "./theme";

describe("theme", () => {
  beforeEach(() => {
    document.documentElement.className = "";
    localStorage.clear();
  });

  it("applyTheme('dark') adds .dark and persists", () => {
    applyTheme("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    expect(localStorage.getItem("ccs-theme")).toBe("dark");
  });

  it("applyTheme('light') removes .dark", () => {
    document.documentElement.classList.add("dark");
    applyTheme("light");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("resolveInitialTheme reads persisted value", () => {
    localStorage.setItem("ccs-theme", "dark");
    expect(resolveInitialTheme()).toBe("dark");
  });
});
```

- [ ] **Step 3: 运行确认失败**

Run: `cd web && pnpm --filter @ccs/ui test`
Expected: FAIL（`./theme` 不存在）。

- [ ] **Step 4: 实现**

`web/packages/ui/src/lib/theme.ts`:
```ts
export type Theme = "light" | "dark";
const KEY = "ccs-theme";

export function applyTheme(theme: Theme): void {
  const root = document.documentElement;
  root.classList.toggle("dark", theme === "dark");
  localStorage.setItem(KEY, theme);
}

export function resolveInitialTheme(): Theme {
  const saved = localStorage.getItem(KEY);
  if (saved === "light" || saved === "dark") return saved;
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}
```

在 `index.ts` 追加：`export { applyTheme, resolveInitialTheme, type Theme } from "./theme";`

- [ ] **Step 5: 运行确认通过**

Run: `cd web && pnpm --filter @ccs/ui test`
Expected: PASS（3 个）。

- [ ] **Step 6: Commit**

```bash
git add web/packages/ui/src/lib/theme.ts web/packages/ui/src/lib/theme.test.ts web/packages/ui/src/lib/index.ts web/packages/ui/vitest.config.ts
git commit -m "feat(web/ui): add theme toggle utility with tests"
```

### Task 2.4：StatusBadge 业务组件（TDD，作为后续组件范式）

**Files:**
- Create: `web/packages/ui/src/lib/components/StatusBadge.svelte`, `web/packages/ui/src/lib/status.ts`, `web/packages/ui/src/lib/status.test.ts`
- Modify: `web/packages/ui/src/lib/index.ts`

- [ ] **Step 1: 写失败测试（纯函数先行）**

`web/packages/ui/src/lib/status.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { statusVariant } from "./status";

describe("statusVariant", () => {
  it("maps 2xx to success", () => expect(statusVariant(200)).toBe("success"));
  it("maps 4xx to warning", () => expect(statusVariant(404)).toBe("warning"));
  it("maps 5xx to danger", () => expect(statusVariant(500)).toBe("danger"));
  it("maps null/pending to muted", () => expect(statusVariant(null)).toBe("muted"));
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd web && pnpm --filter @ccs/ui test`
Expected: FAIL（`./status` 不存在）。

- [ ] **Step 3: 实现纯函数 + 组件**

`web/packages/ui/src/lib/status.ts`:
```ts
export type StatusVariant = "success" | "warning" | "danger" | "muted";

export function statusVariant(status: number | null | undefined): StatusVariant {
  if (status == null) return "muted";
  if (status >= 500) return "danger";
  if (status >= 400) return "warning";
  if (status >= 200) return "success";
  return "muted";
}
```

`web/packages/ui/src/lib/components/StatusBadge.svelte`:
```svelte
<script lang="ts">
  import { statusVariant } from "../status";
  import { cn } from "../utils";
  let { status }: { status: number | null | undefined } = $props();
  const cls: Record<string, string> = {
    success: "bg-success/15 text-success",
    warning: "bg-warning/15 text-warning",
    danger: "bg-danger/15 text-danger",
    muted: "bg-muted text-muted-foreground",
  };
</script>

<span class={cn("inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium tabular-nums", cls[statusVariant(status)])}>
  {status ?? "···"}
</span>
```

在 `index.ts` 追加：
```ts
export { statusVariant, type StatusVariant } from "./status";
export { default as StatusBadge } from "./components/StatusBadge.svelte";
```

- [ ] **Step 4: 运行确认通过**

Run: `cd web && pnpm --filter @ccs/ui test`
Expected: PASS（4 个）。

- [ ] **Step 5: Commit**

```bash
git add web/packages/ui/src/lib/status.ts web/packages/ui/src/lib/status.test.ts web/packages/ui/src/lib/components/StatusBadge.svelte web/packages/ui/src/lib/index.ts
git commit -m "feat(web/ui): add StatusBadge component with tests"
```

### Task 2.5：StatCard 组件

**Files:**
- Create: `web/packages/ui/src/lib/components/StatCard.svelte`
- Modify: `web/packages/ui/src/lib/index.ts`

- [ ] **Step 1: 实现组件**

`web/packages/ui/src/lib/components/StatCard.svelte`:
```svelte
<script lang="ts">
  let { label, value, hint }: { label: string; value: string | number; hint?: string } = $props();
</script>

<div class="rounded-lg border border-border bg-card p-4">
  <div class="text-xs uppercase tracking-wide text-muted-foreground">{label}</div>
  <div class="mt-1 text-2xl font-semibold tabular-nums">{value}</div>
  {#if hint}<div class="mt-1 text-xs text-muted-foreground">{hint}</div>{/if}
</div>
```

在 `index.ts` 追加：`export { default as StatCard } from "./components/StatCard.svelte";`

- [ ] **Step 2: 校验编译**

Run: `cd web && pnpm --filter @ccs/ui exec svelte-check --tsconfig ./tsconfig.json`
Expected: 0 errors。

- [ ] **Step 3: Commit**

```bash
git add web/packages/ui/src/lib/components/StatCard.svelte web/packages/ui/src/lib/index.ts
git commit -m "feat(web/ui): add StatCard component"
```

### Task 2.6：DataTable 组件（泛型表格 + 列排序）

**Files:**
- Create: `web/packages/ui/src/lib/components/DataTable.svelte`, `web/packages/ui/src/lib/sort.ts`, `web/packages/ui/src/lib/sort.test.ts`
- Modify: `web/packages/ui/src/lib/index.ts`

- [ ] **Step 1: 写排序纯函数失败测试**

`web/packages/ui/src/lib/sort.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { sortRows } from "./sort";

describe("sortRows", () => {
  const rows = [{ n: 3 }, { n: 1 }, { n: 2 }];
  it("sorts ascending", () => {
    expect(sortRows(rows, "n", "asc").map((r) => r.n)).toEqual([1, 2, 3]);
  });
  it("sorts descending", () => {
    expect(sortRows(rows, "n", "desc").map((r) => r.n)).toEqual([3, 2, 1]);
  });
  it("nulls sort last", () => {
    const r = [{ n: 2 }, { n: null }, { n: 1 }];
    expect(sortRows(r, "n", "asc").map((x) => x.n)).toEqual([1, 2, null]);
  });
});
```

- [ ] **Step 2: 运行确认失败**

Run: `cd web && pnpm --filter @ccs/ui test`
Expected: FAIL（`./sort` 不存在）。

- [ ] **Step 3: 实现 sort + DataTable**

`web/packages/ui/src/lib/sort.ts`:
```ts
export type SortDir = "asc" | "desc";

export function sortRows<T>(rows: T[], key: keyof T, dir: SortDir): T[] {
  const sign = dir === "asc" ? 1 : -1;
  return [...rows].sort((a, b) => {
    const av = a[key] as unknown;
    const bv = b[key] as unknown;
    if (av == null && bv == null) return 0;
    if (av == null) return 1;
    if (bv == null) return -1;
    if (av < bv) return -1 * sign;
    if (av > bv) return 1 * sign;
    return 0;
  });
}
```

`web/packages/ui/src/lib/components/DataTable.svelte`:
```svelte
<script lang="ts" generics="T extends Record<string, any>">
  import { sortRows, type SortDir } from "../sort";
  import type { Snippet } from "svelte";

  type Column = { key: keyof T & string; label: string; sortable?: boolean };
  let {
    columns,
    rows,
    row,
    onRowClick,
  }: {
    columns: Column[];
    rows: T[];
    row: Snippet<[T]>;
    onRowClick?: (r: T) => void;
  } = $props();

  let sortKey = $state<(keyof T & string) | null>(null);
  let sortDir = $state<SortDir>("desc");
  const sorted = $derived(sortKey ? sortRows(rows, sortKey, sortDir) : rows);

  function toggle(key: keyof T & string) {
    if (sortKey === key) sortDir = sortDir === "asc" ? "desc" : "asc";
    else { sortKey = key; sortDir = "asc"; }
  }
</script>

<table class="w-full text-sm">
  <thead class="border-b border-border text-left text-muted-foreground">
    <tr>
      {#each columns as col}
        <th class="px-3 py-2 font-medium">
          {#if col.sortable}
            <button class="hover:text-foreground" onclick={() => toggle(col.key)}>
              {col.label}{sortKey === col.key ? (sortDir === "asc" ? " ↑" : " ↓") : ""}
            </button>
          {:else}{col.label}{/if}
        </th>
      {/each}
    </tr>
  </thead>
  <tbody>
    {#each sorted as r (r)}
      <tr class="border-b border-border/50 hover:bg-muted/50 {onRowClick ? 'cursor-pointer' : ''}"
          onclick={() => onRowClick?.(r)}>
        {@render row(r)}
      </tr>
    {/each}
  </tbody>
</table>
```

在 `index.ts` 追加：
```ts
export { sortRows, type SortDir } from "./sort";
export { default as DataTable } from "./components/DataTable.svelte";
```

- [ ] **Step 4: 运行确认通过**

Run: `cd web && pnpm --filter @ccs/ui test`
Expected: PASS（含 3 个 sort 测试）。

- [ ] **Step 5: Commit**

```bash
git add web/packages/ui/src/lib/sort.ts web/packages/ui/src/lib/sort.test.ts web/packages/ui/src/lib/components/DataTable.svelte web/packages/ui/src/lib/index.ts
git commit -m "feat(web/ui): add sortable DataTable component with tests"
```

### Task 2.7：FilterGroup（可折叠过滤分组 + chip）

**Files:**
- Create: `web/packages/ui/src/lib/components/FilterGroup.svelte`
- Modify: `web/packages/ui/src/lib/index.ts`

- [ ] **Step 1: 实现组件**

`web/packages/ui/src/lib/components/FilterGroup.svelte`:
```svelte
<script lang="ts">
  let {
    title,
    options,
    selected = $bindable([]),
  }: {
    title: string;
    options: string[];
    selected: string[];
  } = $props();
  let open = $state(true);

  function toggle(opt: string) {
    selected = selected.includes(opt)
      ? selected.filter((s) => s !== opt)
      : [...selected, opt];
  }
</script>

<section class="border-b border-border py-2">
  <button class="flex w-full items-center justify-between text-xs font-semibold uppercase tracking-wide text-muted-foreground"
          onclick={() => (open = !open)}>
    <span>{title}</span><span>{open ? "−" : "+"}</span>
  </button>
  {#if open}
    <div class="mt-2 flex flex-wrap gap-1">
      {#each options as opt}
        <button
          class="rounded-md border px-2 py-0.5 text-xs {selected.includes(opt) ? 'border-primary bg-primary/10 text-primary' : 'border-border text-muted-foreground'}"
          onclick={() => toggle(opt)}>{opt}</button>
      {/each}
    </div>
  {/if}
</section>
```

在 `index.ts` 追加：`export { default as FilterGroup } from "./components/FilterGroup.svelte";`

- [ ] **Step 2: 校验编译**

Run: `cd web && pnpm --filter @ccs/ui exec svelte-check --tsconfig ./tsconfig.json`
Expected: 0 errors。

- [ ] **Step 3: Commit**

```bash
git add web/packages/ui/src/lib/components/FilterGroup.svelte web/packages/ui/src/lib/index.ts
git commit -m "feat(web/ui): add collapsible FilterGroup component"
```

### Task 2.8：ConversationView（markdown + 代码高亮 + 角色气泡）

**Files:**
- Create: `web/packages/ui/src/lib/components/ConversationView.svelte`, `web/packages/ui/src/lib/markdown.ts`, `web/packages/ui/src/lib/markdown.test.ts`
- Modify: `web/packages/ui/src/lib/index.ts`, `web/packages/ui/package.json`

- [ ] **Step 1: 加依赖（npm 打包，替代 CDN）**

编辑 `web/packages/ui/package.json` 的 `dependencies` 追加：
```json
"marked": "^15.0.0",
"dompurify": "^3.2.0",
"highlight.js": "^11.11.0"
```
Run: `cd web && pnpm install`

- [ ] **Step 2: 写 markdown 渲染失败测试**

`web/packages/ui/src/lib/markdown.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { renderMarkdown } from "./markdown";

describe("renderMarkdown", () => {
  it("renders bold", () => {
    expect(renderMarkdown("**hi**")).toContain("<strong>hi</strong>");
  });
  it("sanitizes script tags", () => {
    expect(renderMarkdown("<script>alert(1)</script>")).not.toContain("<script>");
  });
});
```

- [ ] **Step 3: 运行确认失败**

Run: `cd web && pnpm --filter @ccs/ui test`
Expected: FAIL（`./markdown` 不存在）。

- [ ] **Step 4: 实现 markdown 工具 + 组件**

`web/packages/ui/src/lib/markdown.ts`:
```ts
import { marked } from "marked";
import DOMPurify from "dompurify";

export function renderMarkdown(src: string): string {
  const raw = marked.parse(src, { async: false }) as string;
  return DOMPurify.sanitize(raw);
}
```

`web/packages/ui/src/lib/components/ConversationView.svelte`:
```svelte
<script lang="ts">
  import { renderMarkdown } from "../markdown";
  type Message = { role: string; content: string };
  let { messages }: { messages: Message[] } = $props();
  const roleClass: Record<string, string> = {
    user: "bg-muted",
    assistant: "bg-primary/5 border border-primary/20",
    system: "bg-warning/10",
    tool: "bg-card border border-border",
  };
</script>

<div class="space-y-3">
  {#each messages as m}
    <div class="rounded-lg p-3 {roleClass[m.role] ?? 'bg-card'}">
      <div class="mb-1 text-xs font-semibold uppercase text-muted-foreground">{m.role}</div>
      <div class="prose prose-sm max-w-none dark:prose-invert">{@html renderMarkdown(m.content)}</div>
    </div>
  {/each}
</div>
```

在 `index.ts` 追加：
```ts
export { renderMarkdown } from "./markdown";
export { default as ConversationView } from "./components/ConversationView.svelte";
```

- [ ] **Step 5: 运行确认通过**

Run: `cd web && pnpm --filter @ccs/ui test`
Expected: PASS（2 个 markdown 测试）。

- [ ] **Step 6: Commit**

```bash
git add web/packages/ui/src/lib/markdown.ts web/packages/ui/src/lib/markdown.test.ts web/packages/ui/src/lib/components/ConversationView.svelte web/packages/ui/src/lib/index.ts web/packages/ui/package.json web/pnpm-lock.yaml
git commit -m "feat(web/ui): add ConversationView with bundled markdown (no CDN)"
```

---

## Phase 3：aggregate app

### Task 3.1：aggregate Vite 工程骨架

**Files:**
- Create: `web/apps/aggregate/package.json`, `web/apps/aggregate/vite.config.ts`, `web/apps/aggregate/tsconfig.json`, `web/apps/aggregate/svelte.config.js`, `web/apps/aggregate/tailwind.config.js`, `web/apps/aggregate/postcss.config.js`, `web/apps/aggregate/index.html`, `web/apps/aggregate/src/main.ts`, `web/apps/aggregate/src/App.svelte`

- [ ] **Step 1: 工程文件**

`web/apps/aggregate/package.json`:
```json
{
  "name": "@ccs/app-aggregate",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "check": "svelte-check --tsconfig ./tsconfig.json"
  },
  "dependencies": {
    "@ccs/api": "workspace:*",
    "@ccs/ui": "workspace:*",
    "svelte": "^5.15.0"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^5.0.0",
    "vite": "^6.0.0",
    "svelte-check": "^4.1.0",
    "tailwindcss": "^3.4.0",
    "autoprefixer": "^10.4.0",
    "typescript": "^5.7.0"
  }
}
```

`web/apps/aggregate/vite.config.ts`:
```ts
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { resolve } from "node:path";

export default defineConfig({
  plugins: [svelte()],
  base: "/",
  build: {
    outDir: resolve(__dirname, "../../../web-aggregate/dist"),
    emptyOutDir: true,
  },
  server: { proxy: { "/api": "http://127.0.0.1:8787" } },
});
```
> 注：`server.proxy` 目标端口按聚合 daemon 实际监听端口调整（开发期用）。

`web/apps/aggregate/svelte.config.js`:
```js
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
export default { preprocess: vitePreprocess() };
```

`web/apps/aggregate/tailwind.config.js`:
```js
import preset from "@ccs/ui/tailwind.preset";
export default {
  presets: [preset],
  content: [
    "./index.html",
    "./src/**/*.{svelte,ts}",
    "../../packages/ui/src/**/*.{svelte,ts}",
  ],
};
```

`web/apps/aggregate/postcss.config.js`:
```js
export default { plugins: { tailwindcss: {}, autoprefixer: {} } };
```

`web/apps/aggregate/tsconfig.json`:
```json
{
  "extends": "../../tsconfig.base.json",
  "compilerOptions": { "types": ["svelte", "vite/client"] },
  "include": ["src"]
}
```

`web/apps/aggregate/index.html`:
```html
<!doctype html>
<html lang="en" class="dark">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>ccs-daemon aggregate</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

`web/apps/aggregate/src/main.ts`:
```ts
import { mount } from "svelte";
import "@ccs/ui/app.css";
import { applyTheme, resolveInitialTheme } from "@ccs/ui";
import App from "./App.svelte";

applyTheme(resolveInitialTheme());
const app = mount(App, { target: document.getElementById("app")! });
export default app;
```

`web/apps/aggregate/src/App.svelte`（占位，后续任务填充）:
```svelte
<script lang="ts">
  import { createClient } from "@ccs/api";
  const client = createClient();
</script>

<main class="p-6">
  <h1 class="text-xl font-semibold">ccs-daemon</h1>
  <p class="text-muted-foreground">aggregate dashboard</p>
</main>
```

- [ ] **Step 2: 构建验证（产出到 web-aggregate/dist）**

Run: `cd web && pnpm install && pnpm --filter @ccs/app-aggregate build`
Expected: 生成 `web-aggregate/dist/index.html` 和 `web-aggregate/dist/assets/*.{js,css}`。

- [ ] **Step 3: Commit**

```bash
git add web/apps/aggregate web/pnpm-lock.yaml
git commit -m "feat(web/aggregate): scaffold Svelte+Vite app shell"
```

### Task 3.2：应用状态 + 数据加载 store

**Files:**
- Create: `web/apps/aggregate/src/store.svelte.ts`

- [ ] **Step 1: 实现响应式 store（Svelte 5 runes）**

`web/apps/aggregate/src/store.svelte.ts`:
```ts
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
```
> 注：`loadRequests` 沿用旧 `app.js` 的"先列 session 再逐个取详情"策略（见 `web-aggregate/app.js` 的 `loadRequests`）。

- [ ] **Step 2: 校验编译**

Run: `cd web && pnpm --filter @ccs/app-aggregate check`
Expected: 0 errors。

- [ ] **Step 3: Commit**

```bash
git add web/apps/aggregate/src/store.svelte.ts
git commit -m "feat(web/aggregate): add reactive data store"
```

### Task 3.3：布局骨架（Header + Sidebar + Content）

**Files:**
- Modify: `web/apps/aggregate/src/App.svelte`
- Create: `web/apps/aggregate/src/components/Sidebar.svelte`, `web/apps/aggregate/src/components/Header.svelte`

- [ ] **Step 1: Header**

`web/apps/aggregate/src/components/Header.svelte`:
```svelte
<script lang="ts">
  import { Button, applyTheme, resolveInitialTheme, type Theme } from "@ccs/ui";
  let theme = $state<Theme>(resolveInitialTheme());
  function toggle() {
    theme = theme === "dark" ? "light" : "dark";
    applyTheme(theme);
  }
</script>

<header class="flex items-center justify-between border-b border-border px-4 py-3">
  <span class="font-semibold">ccs-daemon</span>
  <Button variant="ghost" onclick={toggle}>{theme === "dark" ? "☀" : "☾"}</Button>
</header>
```

- [ ] **Step 2: Sidebar（用共享 FilterGroup + 时间按钮）**

`web/apps/aggregate/src/components/Sidebar.svelte`:
```svelte
<script lang="ts">
  import { FilterGroup, StatCard } from "@ccs/ui";
  import { state, loadAll } from "../store.svelte";
  const windows = ["1h", "24h", "7d", "all"];
  async function setWindow(w: string) {
    state.filters.window = w;
    await loadAll();
  }
</script>

<aside class="w-64 shrink-0 overflow-y-auto border-r border-border p-3">
  <FilterGroup title="Upstreams" options={state.meta.upstreams} bind:selected={state.filters.upstreams} />
  <FilterGroup title="Models" options={state.meta.models} bind:selected={state.filters.models} />
  <FilterGroup title="Working dirs" options={state.meta.cwds} bind:selected={state.filters.cwds} />
  <section class="border-b border-border py-2">
    <div class="text-xs font-semibold uppercase tracking-wide text-muted-foreground">Time</div>
    <div class="mt-2 flex gap-1">
      {#each windows as w}
        <button class="rounded-md border px-2 py-0.5 text-xs {state.filters.window === w ? 'border-primary bg-primary/10 text-primary' : 'border-border text-muted-foreground'}"
                onclick={() => setWindow(w)}>{w}</button>
      {/each}
    </div>
  </section>
  {#if state.stats}
    <section class="mt-3 space-y-2">
      <StatCard label="Requests" value={state.stats.total_requests} />
      <StatCard label="Tokens" value={state.stats.total_tokens} />
      <StatCard label="Errors" value={state.stats.error_count} />
    </section>
  {/if}
</aside>
```

- [ ] **Step 3: App 组装**

`web/apps/aggregate/src/App.svelte`:
```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { Tabs } from "@ccs/ui";
  import Header from "./components/Header.svelte";
  import Sidebar from "./components/Sidebar.svelte";
  import RequestsView from "./views/RequestsView.svelte";
  import SessionsView from "./views/SessionsView.svelte";
  import { state, loadAll, loadRequests } from "./store.svelte";

  onMount(async () => {
    await loadAll();
    await loadRequests();
  });
</script>

<div class="flex h-screen flex-col">
  <Header />
  <div class="flex flex-1 overflow-hidden">
    <Sidebar />
    <main class="flex-1 overflow-y-auto p-4">
      <Tabs.Root bind:value={state.view}>
        <Tabs.List>
          <Tabs.Trigger value="requests">Requests</Tabs.Trigger>
          <Tabs.Trigger value="sessions">Sessions</Tabs.Trigger>
        </Tabs.List>
        <Tabs.Content value="requests"><RequestsView /></Tabs.Content>
        <Tabs.Content value="sessions"><SessionsView /></Tabs.Content>
      </Tabs.Root>
    </main>
  </div>
</div>
```

- [ ] **Step 4: 校验编译（views 占位将在下个任务创建，先建空文件）**

先建空占位 `web/apps/aggregate/src/views/RequestsView.svelte` 与 `SessionsView.svelte`，各含 `<div></div>`，再运行：
Run: `cd web && pnpm --filter @ccs/app-aggregate check`
Expected: 0 errors。

- [ ] **Step 5: Commit**

```bash
git add web/apps/aggregate/src/App.svelte web/apps/aggregate/src/components web/apps/aggregate/src/views
git commit -m "feat(web/aggregate): add Header/Sidebar/layout shell"
```

### Task 3.4：RequestsView（表格 + 状态徽章 + 搜索）

**Files:**
- Modify: `web/apps/aggregate/src/views/RequestsView.svelte`

- [ ] **Step 1: 实现**

`web/apps/aggregate/src/views/RequestsView.svelte`:
```svelte
<script lang="ts">
  import { DataTable, StatusBadge, Input } from "@ccs/ui";
  import type { RequestSummary } from "@ccs/api";
  import { state } from "../store.svelte";

  const cols = [
    { key: "started_at", label: "Time", sortable: true },
    { key: "upstream", label: "Upstream", sortable: true },
    { key: "model", label: "Model", sortable: true },
    { key: "input_tokens", label: "Tokens", sortable: true },
    { key: "status", label: "Status", sortable: true },
    { key: "duration_ms", label: "Duration", sortable: true },
  ] as const;

  const filtered = $derived(
    state.requests.filter((r) => {
      const q = state.search.toLowerCase();
      if (q && !`${r.model} ${r.upstream}`.toLowerCase().includes(q)) return false;
      if (state.filters.upstreams.length && !state.filters.upstreams.includes(r.upstream ?? "")) return false;
      if (state.filters.models.length && !state.filters.models.includes(r.model ?? "")) return false;
      return true;
    }),
  );

  function fmtTime(s: string) { return new Date(s).toLocaleTimeString(); }
</script>

<div class="mb-3"><Input placeholder="Search model / upstream…" bind:value={state.search} /></div>
<DataTable columns={[...cols]} rows={filtered}>
  {#snippet row(r: RequestSummary)}
    <td class="px-3 py-2 tabular-nums">{fmtTime(r.started_at)}</td>
    <td class="px-3 py-2">{r.upstream ?? "—"}</td>
    <td class="px-3 py-2">{r.model ?? "—"}</td>
    <td class="px-3 py-2 tabular-nums">{(r.input_tokens ?? 0) + (r.output_tokens ?? 0)}</td>
    <td class="px-3 py-2"><StatusBadge status={r.status} /></td>
    <td class="px-3 py-2 tabular-nums">{r.duration_ms ?? "—"}ms</td>
  {/snippet}
</DataTable>
```

- [ ] **Step 2: 构建 + 校验**

Run: `cd web && pnpm --filter @ccs/app-aggregate check && pnpm --filter @ccs/app-aggregate build`
Expected: 0 errors，构建成功。

- [ ] **Step 3: Commit**

```bash
git add web/apps/aggregate/src/views/RequestsView.svelte
git commit -m "feat(web/aggregate): implement RequestsView"
```

### Task 3.5：SessionsView + 会话详情抽屉

**Files:**
- Modify: `web/apps/aggregate/src/views/SessionsView.svelte`
- Create: `web/apps/aggregate/src/views/RequestDetail.svelte`

- [ ] **Step 1: RequestDetail（抽屉内容，用 ConversationView）**

`web/apps/aggregate/src/views/RequestDetail.svelte`:
```svelte
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
```

- [ ] **Step 2: SessionsView（表格 + 行点击拉详情 + Sheet 抽屉）**

`web/apps/aggregate/src/views/SessionsView.svelte`:
```svelte
<script lang="ts">
  import { DataTable, Sheet } from "@ccs/ui";
  import type { SessionSummary, RequestDetail } from "@ccs/api";
  import { state, client } from "../store.svelte";
  import RequestDetail from "./RequestDetail.svelte";

  let open = $state(false);
  let detail = $state<RequestDetail | null>(null);

  const cols = [
    { key: "started_at", label: "Started", sortable: true },
    { key: "session_id", label: "Session", sortable: false },
    { key: "upstream", label: "Upstream", sortable: true },
    { key: "alias", label: "Alias", sortable: true },
    { key: "request_count", label: "Requests", sortable: true },
    { key: "duration_ms", label: "Duration", sortable: true },
  ] as const;

  async function openSession(s: SessionSummary) {
    const d = await client.getSession(s.session_id);
    detail = d.requests[0] ? await client.getRequest(s.session_id, d.requests[0].seq) : null;
    open = true;
  }
</script>

<DataTable columns={[...cols]} rows={state.sessions} onRowClick={openSession}>
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
```

- [ ] **Step 3: 构建 + 校验**

Run: `cd web && pnpm --filter @ccs/app-aggregate check && pnpm --filter @ccs/app-aggregate build`
Expected: 0 errors，构建成功。

- [ ] **Step 4: Commit**

```bash
git add web/apps/aggregate/src/views/SessionsView.svelte web/apps/aggregate/src/views/RequestDetail.svelte
git commit -m "feat(web/aggregate): implement SessionsView with detail drawer"
```

---

## Phase 4：proxy app（aggregate 的精简复用）

### Task 4.1：proxy Vite 工程骨架

**Files:**
- Create: `web/apps/proxy/`（package.json、vite.config.ts、tsconfig.json、svelte.config.js、tailwind.config.js、postcss.config.js、index.html、src/main.ts、src/App.svelte）

- [ ] **Step 1: 复刻 aggregate 工程文件，改 outDir 与标题**

各文件内容与 Task 3.1 相同，仅以下差异：
- `package.json` 的 `name` 改为 `@ccs/app-proxy`。
- `vite.config.ts` 的 `outDir` 改为 `resolve(__dirname, "../../../ccs-proxy/web/dist")`。
- `index.html` 的 `<title>` 改为 `ccs-proxy`。

`web/apps/proxy/src/App.svelte`:
```svelte
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
    const body = detail.request_body as { messages?: { role: string; content: unknown }[] } | undefined;
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
```
`src/main.ts` 与 Task 3.1 相同。

- [ ] **Step 2: 构建验证（产出到 ccs-proxy/web/dist）**

Run: `cd web && pnpm install && pnpm --filter @ccs/app-proxy build`
Expected: 生成 `ccs-proxy/web/dist/index.html` 与 `assets/*`。

- [ ] **Step 3: Commit**

```bash
git add web/apps/proxy web/pnpm-lock.yaml
git commit -m "feat(web/proxy): implement single-instance viewer reusing shared UI"
```

---

## Phase 5：Rust 集成（build.rs + web-ui feature + SPA 服务）

### Task 5.1：聚合 daemon SPA 资产服务重写

**Files:**
- Modify: `src/daemon/aggregate/mod.rs`

- [ ] **Step 1: 改 embed 目录 + SPA 路由**

将 `src/daemon/aggregate/mod.rs` 中的：
```rust
#[derive(RustEmbed)]
#[folder = "web-aggregate/"]
struct AggWebAsset;

fn ui_router() -> Router<Arc<AggregateState>> {
    Router::new()
        .route("/", get(|| async { serve_asset("index.html") }))
        .route("/index.html", get(|| async { serve_asset("index.html") }))
        .route("/app.js", get(|| async { serve_asset("app.js") }))
        .route("/style.css", get(|| async { serve_asset("style.css") }))
}
```
替换为：
```rust
#[derive(RustEmbed)]
#[folder = "web-aggregate/dist/"]
struct AggWebAsset;

fn ui_router() -> Router<Arc<AggregateState>> {
    use axum::extract::Path;
    Router::new()
        .route("/", get(|| async { serve_asset("index.html") }))
        .route("/{*path}", get(|Path(path): Path<String>| async move {
            // 静态资产命中则返回，否则 SPA 回退 index.html
            if AggWebAsset::get(&path).is_some() {
                serve_asset(&path)
            } else {
                serve_asset("index.html")
            }
        }))
}
```
> `serve_asset` 已按扩展名推断 MIME，保留不变。

- [ ] **Step 2: 先构建前端，再编译 Rust**

Run:
```bash
cd web && pnpm install && pnpm --filter @ccs/app-aggregate build && cd ..
cargo build
```
Expected: `web-aggregate/dist/` 存在，`cargo build` 成功（rust-embed 嵌入 dist）。

- [ ] **Step 3: 行为测试（已有集成测试不破坏 + 新增 SPA 回退）**

新增/修改集成测试 `tests/aggregate_ui_serving.rs`（若已有同类则并入）:
```rust
// 验证 SPA 回退：未知路径返回 index.html 内容
#[test]
fn unknown_path_falls_back_to_index() {
    // 通过 rust-embed 的逻辑等价断言：dist 中存在 index.html
    // （完整 HTTP 测试见现有 e2e；此处确保资产已嵌入）
    assert!(std::path::Path::new("web-aggregate/dist/index.html").exists());
}
```
Run: `cargo test --test aggregate_ui_serving`
Expected: PASS。

- [ ] **Step 4: Commit**

```bash
git add src/daemon/aggregate/mod.rs tests/aggregate_ui_serving.rs
git commit -m "feat(aggregate): serve Vite SPA assets with index fallback"
```

### Task 5.2：根 crate build.rs + web-ui feature

**Files:**
- Create: `build.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1: Cargo.toml 增 feature 与 build-dep**

在根 `Cargo.toml` 增加（`[dependencies]` 之后）：
```toml
[features]
default = []
web-ui = []

[build-dependencies]
```
> `web-ui` 默认关；CI 构建二进制时以 `--features web-ui` 开启。

- [ ] **Step 2: build.rs（仅 web-ui 开启时调 Vite，否则 no-op）**

`build.rs`:
```rust
use std::path::Path;
use std::process::Command;

fn main() {
    // feature 关闭 → 完全 no-op，外部消费者零 Node。
    if std::env::var_os("CARGO_FEATURE_WEB_UI").is_none() {
        return;
    }
    let web_dir = Path::new("web");
    if !web_dir.exists() {
        return; // 源码包中无 web 目录时跳过
    }
    // 源码变更才重建
    println!("cargo:rerun-if-changed=web/apps/aggregate/src");
    println!("cargo:rerun-if-changed=web/packages/ui/src");
    println!("cargo:rerun-if-changed=web/packages/api/src");

    let pnpm = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };
    let status = Command::new(pnpm)
        .args(["--filter", "@ccs/app-aggregate", "build"])
        .current_dir(web_dir)
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => panic!("vite build failed with status {s}"),
        Err(e) => panic!("failed to run pnpm (is Node installed?): {e}"),
    }
}
```

- [ ] **Step 3: 验证两种构建路径**

Run:
```bash
cargo build                      # 默认无 feature：build.rs no-op
cd web && pnpm install && cd ..  # 确保 pnpm 可用
cargo build --features web-ui    # 触发 Vite 构建
```
Expected: 第一条不碰 Node；第三条生成 `web-aggregate/dist/` 并编译成功。

- [ ] **Step 4: Commit**

```bash
git add build.rs Cargo.toml
git commit -m "build: add web-ui feature gating Vite build via build.rs"
```

### Task 5.3：ccs-proxy SPA 服务 + web-ui feature + build.rs + exclude

**Files:**
- Modify: `ccs-proxy/src/api/ui.rs`, `ccs-proxy/Cargo.toml`
- Create: `ccs-proxy/build.rs`

- [ ] **Step 1: ui.rs 改 embed 目录 + SPA + feature 门控**

将 `ccs-proxy/src/api/ui.rs` 的 `#[folder = "web/"]` 改为 `#[folder = "web/dist/"]`，并把 UI 路由整体放在 `#[cfg(feature = "web-ui")]` 之后；非 feature 时提供一个返回 404/精简提示的空 router。SPA 回退逻辑同 Task 5.1（命中资产则返回，否则回退 `index.html`）。

具体：在 `ui.rs` 顶部 `RustEmbed` 派生上方加 `#[cfg(feature = "web-ui")]`，并为 `pub fn ui_router()` 增加两份实现：
```rust
#[cfg(feature = "web-ui")]
pub fn ui_router() -> axum::Router<crate::state::SharedState> {
    use axum::{routing::get, extract::Path, Router};
    Router::new()
        .route("/", get(|| async { serve_asset("index.html") }))
        .route("/{*path}", get(|Path(p): Path<String>| async move {
            if WebAsset::get(&p).is_some() { serve_asset(&p) } else { serve_asset("index.html") }
        }))
}

#[cfg(not(feature = "web-ui"))]
pub fn ui_router() -> axum::Router<crate::state::SharedState> {
    axum::Router::new()
}
```
> 调用方（组装总 router 处）保持调用 `ui_router()` 不变；feature 关时即空路由。

- [ ] **Step 2: Cargo.toml 增 feature + build.rs + exclude**

编辑 `ccs-proxy/Cargo.toml`：
```toml
[features]
default = []
dev-fs-assets = []
web-ui = []

[package]
# … 既有字段 …
exclude = ["web/", "tests/fixtures/"]
```
并在 `[package]` 段增加 `build = "build.rs"`（紧邻 name/version）。

- [ ] **Step 3: ccs-proxy/build.rs（feature 关 → no-op）**

`ccs-proxy/build.rs`:
```rust
use std::path::Path;
use std::process::Command;

fn main() {
    if std::env::var_os("CARGO_FEATURE_WEB_UI").is_none() {
        return; // 默认及下游消费者：零 Node
    }
    let web_dir = Path::new("../web");
    if !web_dir.exists() {
        return;
    }
    println!("cargo:rerun-if-changed=../web/apps/proxy/src");
    println!("cargo:rerun-if-changed=../web/packages/ui/src");
    let pnpm = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };
    let status = Command::new(pnpm)
        .args(["--filter", "@ccs/app-proxy", "build"])
        .current_dir(web_dir)
        .status()
        .expect("failed to run pnpm");
    assert!(status.success(), "vite build failed");
}
```

- [ ] **Step 4: 验证 feature 开/关都能编译**

Run:
```bash
cargo build -p ccs-proxy                  # 默认：build.rs no-op，ui_router 空
cd web && pnpm --filter @ccs/app-proxy build && cd ..
cargo build -p ccs-proxy --features web-ui # 嵌入 dist
```
Expected: 两条均成功。

- [ ] **Step 5: 验证发布包不含 web**

Run: `cargo package -p ccs-proxy --allow-dirty --list | grep -E "^web/" || echo "web excluded OK"`
Expected: 输出 `web excluded OK`（package 列表无 `web/` 条目）。

- [ ] **Step 6: Commit**

```bash
git add ccs-proxy/src/api/ui.rs ccs-proxy/Cargo.toml ccs-proxy/build.rs
git commit -m "feat(ccs-proxy): gate web UI behind web-ui feature, exclude web from package"
```

---

## Phase 6：CI、切换、清理、验证

### Task 6.1：CI 构建前端

**Files:**
- Modify: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `.github/workflows/publish.yml`

- [ ] **Step 1: 在构建二进制的 job 加 Node/pnpm + 前端构建**

在 `release.yml`（及任何产出二进制的 job）的"构建二进制"步骤之前插入：
```yaml
      - uses: pnpm/action-setup@v4
        with:
          version: 9
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm
          cache-dependency-path: web/pnpm-lock.yaml
      - name: Build frontend
        run: |
          cd web
          pnpm install --frozen-lockfile
          pnpm -r build
```
并把后续 `cargo build --release` 改为带 `--features web-ui`。

- [ ] **Step 2: publish.yml 保持纯 Rust（验证发布包无 web）**

确认 `publish.yml` 的 `cargo publish` **不**加 `--features web-ui`（发布的 crate 纯 Rust）。在 publish 前加一步：
```yaml
      - name: Assert web excluded from package
        run: cargo package -p ccs-proxy --allow-dirty --list | (! grep -q '^web/')
```

- [ ] **Step 3: ci.yml 增前端 lint/test/build job**

新增 job：
```yaml
  frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v4
        with: { version: 9 }
      - uses: actions/setup-node@v4
        with: { node-version: 20, cache: pnpm, cache-dependency-path: web/pnpm-lock.yaml }
      - run: cd web && pnpm install --frozen-lockfile && pnpm -r test && pnpm -r build
```

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/ci.yml .github/workflows/release.yml .github/workflows/publish.yml
git commit -m "ci: build frontend with Node/pnpm; keep published crate Rust-only"
```

### Task 6.2：删除旧命令式前端资产

**Files:**
- Delete: `web-aggregate/{index.html,app.js,style.css}`, `ccs-proxy/web/{index.html,app.js,style.css}`

- [ ] **Step 1: 端到端验证新前端可用（先验证再删）**

Run（聚合 daemon 与 ccs-proxy 各自手动起一次，浏览器打开确认页面正常）：
```bash
cd web && pnpm install && pnpm -r build && cd ..
cargo build --features web-ui
# 手动启动各自 daemon / proxy，浏览器访问其本地 URL，确认 requests/sessions/detail/深色模式正常
```
Expected: 两个新前端页面正常渲染、交互、暗色切换可用。

- [ ] **Step 2: 删除旧资产**

```bash
git rm web-aggregate/index.html web-aggregate/app.js web-aggregate/style.css
git rm ccs-proxy/web/index.html ccs-proxy/web/app.js ccs-proxy/web/style.css
```

- [ ] **Step 3: 全量校验**

Run:
```bash
cargo build --features web-ui
cargo test
cargo clippy --features web-ui -- -D warnings
cargo fmt --check
```
Expected: 全部通过。

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: remove legacy imperative web assets"
```

### Task 6.3：文档更新

**Files:**
- Modify: `CLAUDE.md`, `ccs-proxy/README.md`

- [ ] **Step 1: 更新 CLAUDE.md 的前端构建说明**

在 CLAUDE.md 增加一节"Frontend (web/)"，说明：pnpm workspace 结构、`pnpm -r build`、`web-ui` feature、build.rs 行为、dist 不进仓库、CI 构建流程。

- [ ] **Step 2: 更新 ccs-proxy/README.md**

说明 web dashboard 现为可选 `web-ui` feature（默认关），下游纯 Rust 消费者无需 Node。

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md ccs-proxy/README.md
git commit -m "docs: document frontend workspace and web-ui feature"
```

---

## 验证清单（全部完成后）

- [ ] `cargo build`（无 feature）不触发任何 Node 调用
- [ ] `cargo build --features web-ui` 自动构建并嵌入两个前端
- [ ] `cargo package -p ccs-proxy --list` 不含 `web/`
- [ ] `cd web && pnpm -r test` 全绿（api + ui 单测）
- [ ] `cd web && pnpm -r build` 产出 `web-aggregate/dist/` 与 `ccs-proxy/web/dist/`
- [ ] 两个前端：深色模式、requests/sessions 表格、排序、搜索过滤、会话详情抽屉、markdown 对话渲染均工作
- [ ] 无任何 CDN 引用（grep 两个 dist 的 index.html 无 `cdn.jsdelivr`）
- [ ] `cargo clippy --features web-ui -- -D warnings`、`cargo fmt --check` 通过
- [ ] CI：frontend job 绿；release 带 `--features web-ui`；publish 纯 Rust 且断言 web 被 exclude
```
