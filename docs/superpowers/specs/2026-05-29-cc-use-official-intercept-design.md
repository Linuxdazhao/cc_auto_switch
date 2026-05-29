# `cc use official` 也走 daemon proxy 拦截

**Status**: Approved (2026-05-29)
**Author**: brainstorming session with @Linuxdazhao

## Problem

`cc use official`（和交互式 UI 里选 `official`）当前**完全绕过** daemon proxy。
代码在 `src/cli/main.rs:858-875` 和 `src/interactive/interactive.rs:786-797 /
869-884 / 908-924` 的分支里短路：判断 alias 为 `cc` 或 `official` →
`settings.remove_anthropic_env()` → 用 `EnvironmentConfig::empty()` 启动
Claude，**在调到 `crate::daemon::try_resolve_proxy(...)` 之前就 return**。

后果：用户切到 official 后，所有跟官方 Anthropic 的对话都直连，cc-switch 的抓取/
观测能力丢失。这与其他 alias 的"daemon 在跑就自动走 proxy"行为不一致。

## Goal

切 official 时，如果 daemon 在跑，流量经 daemon proxy 转发到
`https://api.anthropic.com` 并被 ccs-proxy 抓取；daemon 没起来时降级为直连 +
蓝色提示，不阻断。

## Non-Goals

| 不做 | 理由 |
| --- | --- |
| `--no-intercept` flag / 全局 opt-out 开关 | YAGNI；用户要跳过可以不启 daemon |
| 可配置的 official upstream URL | 硬编码 `https://api.anthropic.com`，避免 daemon vs CLI 字符串不一致导致 `find_proxy` literal-match 失败 |
| 自动拉起 daemon | 跟现有非 official alias 行为对齐：daemon 不在就警告，不隐式启动 |
| 防御"用户配 alias URL = `https://api.anthropic.com`"的冲突 | 用户不会这么做；万一发生 dedupe 自然兜底，不是设计目标 |
| 修改 ccs-proxy 内部 | proxy 现有 auth 透传 + 抓取已经满足需求 |

## Current Architecture Findings

这些事实是设计的前提，来自代码调研：

1. **Proxy 是 per-upstream，不是 per-alias**。`dedupe_upstreams`
   (`src/daemon/lifecycle.rs:44-57`) 收集所有 config 的 unique URL，为每个 URL
   spawn 一个 ccs-proxy 实例。
2. **`find_proxy` 是字面字符串匹配**，无 normalization
   (`src/daemon/state.rs:73-77`)。upstream URL 字符串必须和 daemon 注册时一致。
3. **URL 替换发生在客户端**。`try_resolve_proxy` 命中时，CLI 把
   `config.url` 覆写成 proxy URL (`src/cli/main.rs:888`,
   `src/interactive/interactive.rs:935`)，作为 `ANTHROPIC_BASE_URL` env var 传给
   Claude CLI (`src/config/config.rs:36`)。
4. **Token 注入在客户端，不在 proxy**。proxy 的 `send_upstream`
   (`ccs-proxy/src/proxy/forward.rs:164-201`) 复制所有 inbound header（除 hop-by-hop
   + content-length）verbatim 转发，从不注入或替换 Authorization。
5. **OAuth 路径完全安全**。official 模式用 `EnvironmentConfig::empty()` 启动 Claude
   CLI，Claude CLI 自己从 `~/.claude/.credentials.json` 读 OAuth 凭证并发
   Authorization header。proxy 的 `redact_headers`
   (`ccs-proxy/src/capture/redact.rs:28`) 只作用于存档的 `req_headers_map` 副本
   (`forward.rs:273`)，不影响在线 bytes。
6. **抓取层 alias-agnostic**。`CaptureRecord` 和 `SessionMeta` 只携带
   `session_id` / `provider` / `upstream`，没有 alias 字段。alias 是
   `AliasMap::from_storage` (`src/daemon/aggregate/state.rs:11-37`) 从 upstream
   反查出来的；聚合层（`event_merger`、`routes::list_sessions/health`）通过
   `aliases_for(upstream)` 附加 alias。

## Design

### 改动定位

6 个文件，约 80 行：

| # | 文件 | 位置 | 改动 |
| --- | --- | --- | --- |
| 1 | `src/daemon/mod.rs` | 顶部 | 新增 `pub const OFFICIAL_UPSTREAM: &str = "https://api.anthropic.com";` |
| 2 | `src/daemon/mod.rs` | 与 `try_resolve_proxy` 同 module | 新 helper `resolve_official_proxy()`（签名见下节），4 个调用点共享 |
| 3 | `src/daemon/lifecycle.rs` | `dedupe_upstreams` (L44-57) | upstream 集合**无条件** push `OFFICIAL_UPSTREAM`；HashSet/BTreeSet 自然处理重复 |
| 4 | `src/config/config.rs` | `EnvironmentConfig` | 加链式方法 `pub fn with_base_url(mut self, url: impl Into<String>) -> Self` |
| 5 | `src/cli/main.rs` | `Commands::Use` (L858-875) | 删 short-circuit body，改成调 helper |
| 6 | `src/interactive/interactive.rs` | 3 个 official 启动点 (L786-797, L869-884, L908-924) | 同 #5 调 helper |
| 7 | `src/daemon/aggregate/state.rs` | `AliasMap::from_storage` (L11-37) | 在最终 map 里给 `OFFICIAL_UPSTREAM` 插入 `vec!["official".to_string()]`（即使没有用户 alias 也建条目） |

### Helper 签名

```rust
// in src/daemon/mod.rs
pub enum OfficialResolution {
    Proxied { proxy_url: String },
    Direct,
}

pub fn resolve_official_proxy() -> OfficialResolution {
    match try_resolve_proxy(OFFICIAL_UPSTREAM) {
        ProxyResolution::Proxied { proxy_url } => OfficialResolution::Proxied { proxy_url },
        ProxyResolution::Direct => OfficialResolution::Direct,
    }
}
```

调用点共享逻辑（伪代码）：

```rust
let env = match daemon::resolve_official_proxy() {
    OfficialResolution::Proxied { proxy_url } => {
        eprintln!("{}", format!(
            "Routing official traffic through cc daemon proxy at {proxy_url}"
        ).blue());
        EnvironmentConfig::empty()
            .with_alias("official")
            .with_base_url(proxy_url)
    }
    OfficialResolution::Direct => {
        eprintln!("{}", format!(
            "cc daemon is not running — official traffic will NOT be captured."
        ).blue());
        eprintln!("{}", "  Run `cc-switch daemon start` and re-run to enable capture.".blue());
        EnvironmentConfig::empty().with_alias("official")
    }
};
launch_claude_with_env(env, ...)?;
```

### Data Flow

**daemon 启动**：

```
daemon start
  └─→ dedupe_upstreams(configs) ∪ { OFFICIAL_UPSTREAM }
        └─→ 对每个 unique upstream spawn 一个 ccs-proxy on port P_i
              └─→ DaemonState 记录 { URL_i → port P_i }
```

`https://api.anthropic.com` 永远在 state 里有一个 port。

**`cc use official` 运行时**：

```
cc use official
  └─→ daemon::resolve_official_proxy()
        ├─ Proxied { proxy_url }  →  env = empty + alias=official + BASE_URL=proxy_url
        │                             (无 ANTHROPIC_AUTH_TOKEN → Claude CLI 走 OAuth)
        └─ Direct                  →  蓝色提示；env = empty + alias=official（现状）
  └─→ launch_claude_with_env(env)
```

**请求飞行（命中 Proxied 时）**：

```
claude CLI                                      ccs-proxy                       Anthropic
  ─── POST http://127.0.0.1:P/v1/messages ────→
       Authorization: Bearer <OAuth>
                                                ─── POST https://api.anthropic.com/v1/messages ───→
                                                     Authorization: Bearer <OAuth>  (verbatim)
                                                ←── response stream ─────────────────
  ←── response stream ─────────────────
       (proxy 同时 capture session 写 store)
```

OAuth header **完全不被改动**：redact 只作用于存档副本。

**Web 视图归属**：

```
ccs-proxy 写 session_meta: { upstream: "https://api.anthropic.com", ... }
  ↓
daemon /api/sessions 聚合
  ↓
AliasMap::aliases_for("https://api.anthropic.com") → ["official"]
  ↓
web 客户端把这条 session 放进 "official" tab
```

### 错误处理

| 场景 | 行为 |
| --- | --- |
| daemon 没起来 | 蓝色提示 "daemon is not running — official traffic will NOT be captured"，继续直连，不阻断 |
| daemon 在跑但 official proxy spawn 失败 | 现有 daemon spawn 失败兜底路径生效（`print_version_mismatch_warning` 等），不新增分支 |
| Claude OAuth 凭证缺失/失效 | 不变 — 不是 cc-switch 责任，Claude CLI 自报错 |
| proxy 转发到 anthropic 失败（网络 / 5xx） | ccs-proxy 现有 502 / connection error 路径生效 |

### 颜色

统一规范：**daemon / proxy 相关的所有用户提示一律蓝色**（不区分 info / warn），
official 和非 official 都用蓝色。用 `colored` crate（已是依赖）。

需要上色的现有 + 新增点：

- `src/cli/main.rs:860` `"Using official Claude configuration"` → 蓝色
- `src/cli/main.rs:892-898` `"cc daemon is not running — traffic for '...' will NOT be captured."` → 蓝色
- 新增 `"Routing official traffic through cc daemon proxy at ..."` → 蓝色
- 新增 official 路径的 `"cc daemon is not running — official traffic will NOT be captured."` → 蓝色
- 交互式 UI 里 official 切换相关的等价提示 → 蓝色

### 测试

**单元测试**（添加 / 扩展）：

| 文件 | 测试 |
| --- | --- |
| `tests/main_tests.rs` | `cc use official` 路径不再无条件 short-circuit；mock `try_resolve_proxy` 返回 Proxied 时 env 包含 `ANTHROPIC_BASE_URL` 而非 token；返回 Direct 时 env 为 empty |
| `tests/integration_tests.rs` | `EnvironmentConfig::empty().with_alias("official").with_base_url(...)` 的 env tuples 正确（含 BASE_URL，无 token） |
| `src/daemon/aggregate/state.rs` `#[cfg(test)]` | `AliasMap::from_storage` 无论 ConfigStorage 是否为空，`aliases_for(OFFICIAL_UPSTREAM)` 都返回 `["official"]` |
| `src/daemon/lifecycle.rs` `#[cfg(test)]` | `dedupe_upstreams(&[])` 结果仍包含 `OFFICIAL_UPSTREAM` |

**集成 / 手工验证**（PR description 记录）：

1. `cc-switch daemon start` → 用 `lsof -i` 或 daemon status 确认有 proxy 监听
   `OFFICIAL_UPSTREAM` 对应端口
2. `cc use official` → 看到蓝色 "Routing official traffic through cc daemon proxy at ..."
3. Claude 里发一条消息 → 打开 web 视图 → 看到新 session 归属在 `official` tab 下，
   model 字段已记录
4. 停 daemon → `cc use official` → 看到蓝色 "daemon is not running … will NOT be captured"，
   Claude 正常启动并能通话（OAuth 直连）

## Rationale

**为什么选 A（daemon 启动时无条件注入 official upstream）而非 D（按需复用）**：
用户明确表达 official 拦截应该是 first-class 行为，不依赖用户是否凑巧配了一个
URL 等于 `https://api.anthropic.com` 的 alias（按 user statement 这种情况不会发生）。
A 让"daemon 在跑 → official 就被拦截"成为强保证，D 把可观测性变成偶然属性。
A 的代价仅多一个 ccs-proxy 进程（~几 MB RSS + 一个 epoll fd），值得。

**为什么选 A 而非 B（把 official 实现为伪 alias）**：B 要修改 ConfigStorage 语义、
保护伪 alias 不被持久化和 remove、改交互式 UI 的索引/排序（official 现在是固定第一项），
改动面显著大于 A，没带来对应收益。

**为什么选 A 而非 C（lazy spawn）**：C 为了省一个 proxy 进程引入运行时 IPC 操作和
daemon 可变状态，spawn 失败的错误路径要单独设计，复杂度换那点资源不划算。

**为什么不加 opt-out**：cc-switch 的定位就是拦截工具；用户不想被拦截可以选择不启 daemon。
将来真有需求再加。

**为什么硬编码 URL**：`find_proxy` 是 literal match，daemon 注册的 upstream 字符串和
CLI 查询的字符串必须一字不差。硬编码消除 source-of-truth 漂移风险。

## Out of Scope (Recap)

- opt-out flag / 全局开关
- 可配置的 official URL
- 自动拉起 daemon
- 用户冲突 alias 防御
- ccs-proxy 内部改动
