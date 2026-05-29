# `cc use official` 走 daemon proxy 拦截 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 `cc use official` 和交互式 UI 选 official 时，流量经 daemon proxy 转发到 `https://api.anthropic.com` 被抓取，OAuth 凭证端到端透传；daemon 没起来时降级直连并蓝色提示。

**Architecture:** daemon 启动时无条件为 `OFFICIAL_UPSTREAM = "https://api.anthropic.com"` spawn 一个 ccs-proxy；4 个 official 启动点都改成调用共享 helper `build_official_env()`（位于 `src/daemon/mod.rs`），helper 内部查 daemon 状态、决定是否设 `ANTHROPIC_BASE_URL` 到 proxy port、打印蓝色提示。`AliasMap` 注入 `OFFICIAL_UPSTREAM → ["official"]` 使 web 视图归属正确。

**Tech Stack:** Rust 2024 edition / clap / anyhow / colored / tokio / 现有 daemon + ccs-proxy

**Spec:** [`docs/superpowers/specs/2026-05-29-cc-use-official-intercept-design.md`](../specs/2026-05-29-cc-use-official-intercept-design.md)

---

## Task 1: `EnvironmentConfig::with_base_url` 链式方法

**Files:**
- Modify: `src/config/config.rs` (在 `with_alias` 之后加方法)
- Test: `src/config/config.rs` 同文件内 `#[cfg(test)]` 模块（如不存在则新建），或 `tests/integration_tests.rs`

- [ ] **Step 1: 检查 config.rs 是否已有 `#[cfg(test)] mod tests`**

Run:
```bash
grep -n '#\[cfg(test)\]' src/config/config.rs
```
Expected: 可能没有 — 如果没有，下面 Step 2 在文件末尾新增；如果有，把 test fn 加进去。

- [ ] **Step 2: 写 failing test**

如果文件末尾**没有** test 模块，在 `src/config/config.rs` 文件末尾追加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_base_url_sets_anthropic_base_url() {
        let env = EnvironmentConfig::empty()
            .with_alias("official")
            .with_base_url("http://127.0.0.1:9876");
        assert_eq!(
            env.env_vars.get("ANTHROPIC_BASE_URL").map(String::as_str),
            Some("http://127.0.0.1:9876"),
        );
        assert_eq!(
            env.env_vars.get("CC_SWITCH_CURRENT_ALIAS").map(String::as_str),
            Some("official"),
        );
        assert!(env.env_vars.get("ANTHROPIC_AUTH_TOKEN").is_none(),
            "with_base_url must NOT set a token (OAuth must flow through unchanged)");
    }
}
```

如果已存在 test 模块，把 `with_base_url_sets_anthropic_base_url` 函数加到模块里。

- [ ] **Step 3: 运行确认 test 失败（编译失败 = method not found）**

Run:
```bash
cargo test --lib with_base_url_sets_anthropic_base_url
```
Expected: 编译错误，提示 `no method named with_base_url found for struct EnvironmentConfig`

- [ ] **Step 4: 实现 `with_base_url`**

在 `src/config/config.rs` 的 `with_alias` 方法（约 L148-152）之后追加：

```rust
    /// Set the `ANTHROPIC_BASE_URL` env var (used to point Claude CLI at a
    /// proxy without injecting a token — OAuth Authorization headers flow
    /// through verbatim).
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.env_vars
            .insert("ANTHROPIC_BASE_URL".to_string(), url.into());
        self
    }
```

- [ ] **Step 5: 跑 test 确认通过**

Run:
```bash
cargo test --lib with_base_url_sets_anthropic_base_url
```
Expected: `test ... ok`

- [ ] **Step 6: Commit**

```bash
git add src/config/config.rs
git commit -m "feat(config): add EnvironmentConfig::with_base_url chainable method"
```

(prek hook 会跑全套 fmt/clippy/test/audit/doc/release build — 全过才会成 commit)

---

## Task 2: 加常量 `OFFICIAL_UPSTREAM` + 共享 helper `build_official_env()`

**Files:**
- Modify: `src/daemon/mod.rs` (在 `try_resolve_proxy` 之后追加)

Helper 本身没有可独立测试的纯逻辑（涉及全局文件系统 + println 副作用），把覆盖留给 Task 5 之后的手动验证 + 下游集成。常量是单纯 `pub const`，无测试需求。

- [ ] **Step 1: 加常量 + helper**

在 `src/daemon/mod.rs` 文件**最末尾**追加：

```rust
/// Official Anthropic upstream URL. The daemon spawns one ccs-proxy for this
/// URL at startup so `cc use official` traffic can be captured.
///
/// MUST stay byte-identical to Claude CLI's default `ANTHROPIC_BASE_URL`, since
/// `find_proxy` does literal string match.
pub const OFFICIAL_UPSTREAM: &str = "https://api.anthropic.com";

/// Build the `EnvironmentConfig` for the "official" launch path, printing
/// user-facing status in blue. Returns an env with `CC_SWITCH_CURRENT_ALIAS`
/// set to `"official"`, plus `ANTHROPIC_BASE_URL` pointing at the daemon proxy
/// if it's running. Never sets `ANTHROPIC_AUTH_TOKEN` — Claude CLI's OAuth
/// credentials must flow through unchanged.
pub fn build_official_env() -> crate::config::config::EnvironmentConfig {
    use crate::config::config::EnvironmentConfig;
    use colored::Colorize;
    let env = EnvironmentConfig::empty().with_alias("official");
    match try_resolve_proxy(OFFICIAL_UPSTREAM) {
        ProxyResolution::Proxied { proxy_url } => {
            eprintln!(
                "{}",
                format!(
                    "\u{2139} Routing official traffic through cc daemon proxy at {proxy_url}"
                )
                .blue()
            );
            env.with_base_url(proxy_url)
        }
        ProxyResolution::Direct => {
            eprintln!(
                "{}",
                "\u{2139} cc daemon is not running — official traffic will NOT be captured."
                    .blue()
            );
            eprintln!(
                "{}",
                "  Run `cc-switch daemon start` and re-run to enable capture.".blue()
            );
            env
        }
    }
}
```

- [ ] **Step 2: 跑 cargo check 确认编译通过**

Run:
```bash
cargo check
```
Expected: clean exit, no warnings about unused (helper will be called in Task 5).

如果 clippy 之后报 `dead_code` 警告，先 attribute `#[allow(dead_code)]` 上去，Task 5 用上后再移除。但 lib 公开 `pub` 函数通常不会被 dead_code 警告。

- [ ] **Step 3: Commit**

```bash
git add src/daemon/mod.rs
git commit -m "feat(daemon): add OFFICIAL_UPSTREAM const + build_official_env helper"
```

---

## Task 3: `dedupe_upstreams` 始终包含 `OFFICIAL_UPSTREAM`

**Files:**
- Modify: `src/daemon/lifecycle.rs:44-57` (函数体改动 + 新增 `#[cfg(test)]` 模块)

- [ ] **Step 1: 写 failing test**

在 `src/daemon/lifecycle.rs` 文件末尾追加：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::{ConfigStorage, Configuration};
    use std::collections::BTreeMap;

    fn empty_storage() -> ConfigStorage {
        ConfigStorage {
            configurations: BTreeMap::new(),
            ..Default::default()
        }
    }

    fn storage_with_url(url: &str) -> ConfigStorage {
        let mut configurations = BTreeMap::new();
        configurations.insert(
            "my".to_string(),
            Configuration {
                alias_name: "my".to_string(),
                token: "sk-test".to_string(),
                url: url.to_string(),
                ..Default::default()
            },
        );
        ConfigStorage {
            configurations,
            ..Default::default()
        }
    }

    #[test]
    fn dedupe_upstreams_always_includes_official() {
        let result = dedupe_upstreams(&empty_storage());
        assert!(
            result.contains(&("claude".to_string(), crate::daemon::OFFICIAL_UPSTREAM.to_string())),
            "OFFICIAL_UPSTREAM must always be in dedupe_upstreams output, got {result:?}",
        );
    }

    #[test]
    fn dedupe_upstreams_dedupes_when_user_has_official_url() {
        // Belt-and-suspenders: user shouldn't normally do this, but if they
        // configure an alias with the official URL, we must not spawn two
        // proxies for the same URL.
        let result = dedupe_upstreams(&storage_with_url(crate::daemon::OFFICIAL_UPSTREAM));
        let count = result
            .iter()
            .filter(|(_, url)| url == crate::daemon::OFFICIAL_UPSTREAM)
            .count();
        assert_eq!(count, 1, "OFFICIAL_UPSTREAM must appear exactly once, got {result:?}");
    }
}
```

注意：`ConfigStorage` 和 `Configuration` 都必须支持 `..Default::default()`。如果当前不实现 `Default`，先 grep 确认：

```bash
grep -n 'derive(.*Default' src/config/types.rs
```

如果它们没有 Default derive，可改用 `Configuration::new(...)` 等现有构造，或在 `storage_with_url` 里逐字段填 `None`。如果存在 `make_storage` 这种 helper 已在 `state.rs` 测试里用过，借鉴它。

- [ ] **Step 2: 运行确认 test 失败**

Run:
```bash
cargo test --lib dedupe_upstreams
```
Expected: `dedupe_upstreams_always_includes_official` FAIL（结果不含 OFFICIAL_UPSTREAM），`dedupe_upstreams_dedupes_when_user_has_official_url` 可能 PASS（dedupe 还正常）。

- [ ] **Step 3: 实现 — 修改 dedupe_upstreams**

把 `src/daemon/lifecycle.rs:44-57` 的函数体改为：

```rust
fn dedupe_upstreams(storage: &ConfigStorage) -> Vec<Upstream> {
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for config in storage.configurations.values() {
        if config.url.is_empty() {
            continue;
        }
        let key = ("claude".to_string(), config.url.clone());
        if seen.insert(key.clone()) {
            result.push(key);
        }
    }
    // Always include the official Anthropic upstream so `cc use official`
    // routes through the daemon. Dedup naturally handles the (rare) case
    // where a user-defined alias points at the same URL.
    let official = (
        "claude".to_string(),
        crate::daemon::OFFICIAL_UPSTREAM.to_string(),
    );
    if seen.insert(official.clone()) {
        result.push(official);
    }
    result
}
```

- [ ] **Step 4: 跑 test 确认两个都通过**

Run:
```bash
cargo test --lib dedupe_upstreams
```
Expected: 两个 test 都 PASS。

- [ ] **Step 5: Commit**

```bash
git add src/daemon/lifecycle.rs
git commit -m "feat(daemon): always include OFFICIAL_UPSTREAM in dedupe_upstreams"
```

---

## Task 4: `AliasMap::from_storage` 注入 `official` 标签

**Files:**
- Modify: `src/daemon/aggregate/state.rs:11-37` (函数体改动)
- Modify: `src/daemon/aggregate/state.rs:46+` 已有的 `#[cfg(test)] mod tests`（追加 test）

- [ ] **Step 1: 写 failing test**

在 `src/daemon/aggregate/state.rs` 的现有 test 模块里追加（紧跟在 `make_storage` 等已有 helper 之后）：

```rust
    #[test]
    fn alias_map_always_attributes_official_upstream() {
        let map = AliasMap::from_storage(&make_storage(&[]));
        assert_eq!(
            map.aliases_for(crate::daemon::OFFICIAL_UPSTREAM),
            vec!["official".to_string()],
            "OFFICIAL_UPSTREAM must always map to [official] even with empty storage",
        );
    }

    #[test]
    fn alias_map_keeps_user_alias_when_url_overlaps_official() {
        // User shouldn't normally configure an alias with the official URL,
        // but if they do, both the user alias and "official" should appear.
        let map = AliasMap::from_storage(&make_storage(&[(
            "myofficial",
            crate::daemon::OFFICIAL_UPSTREAM,
        )]));
        let aliases = map.aliases_for(crate::daemon::OFFICIAL_UPSTREAM);
        assert!(aliases.contains(&"myofficial".to_string()), "got {aliases:?}");
        assert!(aliases.contains(&"official".to_string()), "got {aliases:?}");
    }
```

注意：检查 `make_storage` 现有签名（state.rs L52+）— 它返回 `ConfigStorage` 接受 `&[(&str, &str)]`。如果签名不一样，调整测试以使用现有 helper。

- [ ] **Step 2: 运行确认 test 失败**

Run:
```bash
cargo test --lib alias_map_always_attributes_official_upstream alias_map_keeps_user_alias_when_url_overlaps_official
```
Expected: 两个 test 都 FAIL — `aliases_for(OFFICIAL_UPSTREAM)` 返回空 vec。

- [ ] **Step 3: 实现**

把 `src/daemon/aggregate/state.rs:16-26` 的 `from_storage` 改为：

```rust
    pub fn from_storage(storage: &ConfigStorage) -> Self {
        let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for config in storage.configurations.values() {
            if !config.url.is_empty() {
                map.entry(config.url.clone())
                    .or_default()
                    .push(config.alias_name.clone());
            }
        }
        // Always attribute the official Anthropic upstream to the "official"
        // alias so the web view groups daemon-proxied official traffic
        // correctly. Appended (not replaced) so an overlapping user alias
        // still appears alongside.
        map.entry(crate::daemon::OFFICIAL_UPSTREAM.to_string())
            .or_default()
            .push("official".to_string());
        Self { map }
    }
```

- [ ] **Step 4: 跑 test 确认通过**

Run:
```bash
cargo test --lib alias_map
```
Expected: 两个新 test + 已有 alias_map 相关 test 全 PASS。

- [ ] **Step 5: Commit**

```bash
git add src/daemon/aggregate/state.rs
git commit -m "feat(daemon): inject 'official' alias for OFFICIAL_UPSTREAM in AliasMap"
```

---

## Task 5: CLI `Commands::Use` — 调用 helper，official 路径走 daemon proxy

**Files:**
- Modify: `src/cli/main.rs:858-875`

无独立单元测试（涉及 launch_claude_with_env 启动子进程）。Task 8 做端到端手动验证。

- [ ] **Step 1: 把 short-circuit body 改成调用 helper**

把 `src/cli/main.rs:858-875` 这个块：

```rust
                // Handle special reset aliases
                if alias_name == "cc" || alias_name == "official" {
                    println!("Using official Claude configuration");

                    let mut settings = ClaudeSettings::load(
                        storage.get_claude_settings_dir().map(|s| s.as_str()),
                    )?;
                    settings.remove_anthropic_env();
                    settings.save(storage.get_claude_settings_dir().map(|s| s.as_str()))?;

                    launch_claude_with_env(
                        EnvironmentConfig::empty().with_alias("official"),
                        None,
                        None,
                        r#continue,
                    )?;
                    return Ok(());
                }
```

改成：

```rust
                // Handle special reset aliases (route through daemon proxy if running)
                if alias_name == "cc" || alias_name == "official" {
                    use colored::Colorize;
                    println!("{}", "Using official Claude configuration".blue());

                    let mut settings = ClaudeSettings::load(
                        storage.get_claude_settings_dir().map(|s| s.as_str()),
                    )?;
                    settings.remove_anthropic_env();
                    settings.save(storage.get_claude_settings_dir().map(|s| s.as_str()))?;

                    crate::daemon::print_version_mismatch_warning();
                    let env = crate::daemon::build_official_env();

                    launch_claude_with_env(env, None, None, r#continue)?;
                    return Ok(());
                }
```

变更要点：
- `println!("Using official Claude configuration")` 加蓝色（`use colored::Colorize` + `.blue()`）
- 删除直接构造 `EnvironmentConfig::empty().with_alias("official")` 的内联代码
- 加 `print_version_mismatch_warning()`（和非 official 路径对齐）
- 用 `daemon::build_official_env()` 拿 env（helper 内部处理 daemon 查询 + 蓝色提示）

- [ ] **Step 2: 跑现有单元测试和 fmt 确认不破坏**

Run:
```bash
cargo fmt --all -- --check && cargo clippy -- -D warnings && cargo test --lib
```
Expected: 全过。

- [ ] **Step 3: Commit**

```bash
git add src/cli/main.rs
git commit -m "feat(cli): route 'cc use official' through daemon proxy"
```

---

## Task 6: Interactive UI — 3 个 official 启动点调用 helper

**Files:**
- Modify: `src/interactive/interactive.rs` 三处：约 L786-797 / L869-884 / L908-924（具体行号以当前文件为准，找特征 `EnvironmentConfig::empty().with_alias("official")`）

- [ ] **Step 1: 找到所有 3 处**

Run:
```bash
grep -n 'with_alias("official")' src/interactive/interactive.rs
```
Expected: 3 个匹配，对应 spec 里的三处启动点。

- [ ] **Step 2: 三处统一改造**

对**每一处**做相同改造：把
```rust
EnvironmentConfig::empty().with_alias("official")
```
改为
```rust
crate::daemon::build_official_env()
```

注意：
- 如果某处把 EnvironmentConfig 赋给变量后才传给 launch — 也按 `let env = crate::daemon::build_official_env();` 替换
- 检查每处之前/之后有没有"Using official Claude configuration"等提示，如果有需要保留就加 `.blue()`，重复就删掉（helper 已经打了 routing 提示）
- 不要在改造完之前删旧的 `EnvironmentConfig` 引用 — 让编译器告诉你哪里漏改

- [ ] **Step 3: 编译 + clippy + fmt**

Run:
```bash
cargo fmt --all && cargo clippy -- -D warnings && cargo build
```
Expected: clean。

- [ ] **Step 4: 跑测试**

Run:
```bash
cargo test
```
Expected: 全过。

- [ ] **Step 5: Commit**

```bash
git add src/interactive/interactive.rs
git commit -m "feat(interactive): route 'official' selection through daemon proxy"
```

---

## Task 7: 给现有非 official daemon 警告也上蓝色

**Files:**
- Modify: `src/cli/main.rs:892-898`

为了和 official 路径的颜色规范一致（"daemon/proxy 相关提示统一蓝色"）。

- [ ] **Step 1: 包蓝色**

把 `src/cli/main.rs:892-898`：

```rust
                            eprintln!(
                                "\u{2139} cc daemon is not running — traffic for '{}' will NOT be captured.",
                                alias_name
                            );
                            eprintln!(
                                "  Run `cc-switch daemon start` and re-run to enable capture."
                            );
```

改为：

```rust
                            use colored::Colorize;
                            eprintln!(
                                "{}",
                                format!(
                                    "\u{2139} cc daemon is not running — traffic for '{alias_name}' will NOT be captured."
                                )
                                .blue()
                            );
                            eprintln!(
                                "{}",
                                "  Run `cc-switch daemon start` and re-run to enable capture.".blue()
                            );
```

(如果文件顶部已 `use colored::Colorize;`，则不重复 use)

- [ ] **Step 2: 测 + 提交**

Run:
```bash
cargo fmt --all && cargo clippy -- -D warnings && cargo test
```

```bash
git add src/cli/main.rs
git commit -m "chore(cli): paint non-official daemon-down warning blue for consistency"
```

---

## Task 8: 端到端手工验证（无代码改动）

这一步**必须通过**才能宣告功能完成。把所有结果记入 PR description。

- [ ] **Step 1: 确认 daemon 健康，official upstream 已注册**

```bash
cargo build --release
./target/release/cc-switch daemon stop 2>/dev/null
./target/release/cc-switch daemon start
./target/release/cc-switch daemon status
```
Expected: status 输出里能看到 `https://api.anthropic.com` 对应的一个 proxy 端口（具体输出格式以 status 命令现有格式为准；如不直观，看 `~/.cc-switch/daemon-state.json` 里 `proxies` 数组是否包含 `upstream: "https://api.anthropic.com"` 的条目）。

辅助验证端口确实在监听：
```bash
# 提取上面 status 里 official 的端口（假设是 P），然后：
lsof -nP -iTCP:P -sTCP:LISTEN
```
Expected: 看到一个进程在监听。

- [ ] **Step 2: 切到 official，看蓝色路由提示**

```bash
./target/release/cc-switch use official
```
Expected: 终端看到两条蓝色文字：
```
ℹ Using official Claude configuration   (blue)
ℹ Routing official traffic through cc daemon proxy at http://127.0.0.1:<P>   (blue)
```
紧接着 Claude CLI 启动（保留你 OAuth 登录状态）。

- [ ] **Step 3: 在 Claude 里发一条消息，确认 OAuth 工作**

在打开的 claude session 里输入任意 prompt，比如 `hello`。
Expected: 正常收到回复 — 证明 OAuth header 经 proxy 转发到 anthropic 没被破坏。

- [ ] **Step 4: 打开 web 视图，确认归属 official**

```bash
# 退出当前 claude session（Ctrl+D），然后查看 aggregate web 端口：
./target/release/cc-switch daemon status   # 看 aggregate 端口
# 浏览器打开 http://127.0.0.1:<aggregate_port>/
```
Expected: 新 session 出现在 alias=`official` 的分组下，model 字段被记录（如 `claude-sonnet-4-6` 或 `claude-opus-4-7`）。

- [ ] **Step 5: 停 daemon，看蓝色 fallback 提示**

```bash
./target/release/cc-switch daemon stop
./target/release/cc-switch use official
```
Expected: 看到两条蓝色文字：
```
ℹ cc daemon is not running — official traffic will NOT be captured.   (blue)
  Run `cc-switch daemon start` and re-run to enable capture.            (blue)
```
然后 Claude CLI 正常启动 — 证明 daemon 不在也不阻断，能 OAuth 直连。

- [ ] **Step 6: 跑 alias 路径，验证 Task 7 的颜色对齐**

(daemon 仍停止)
```bash
./target/release/cc-switch use <某个普通 alias>
```
Expected: 看到蓝色 "cc daemon is not running — traffic for '...' will NOT be captured."（之前是纯文本）。

- [ ] **Step 7: 把所有截图/输出记入 PR description**

PR description 模板：

```markdown
## Manual verification

- [x] daemon status 列出 https://api.anthropic.com proxy on port P
- [x] cc use official → blue routing message, Claude OAuth round-trip works
- [x] web view groups session under "official" tab with model recorded
- [x] daemon stopped → blue fallback message, Claude direct OAuth works
- [x] non-official alias daemon-down warning also blue (color consistency)
```

---

## Final: 推送 + 监控 CI

- [ ] **Step 1: 看本地全部 commit**

```bash
git log --oneline main..HEAD
```
Expected: 看到 ~7 个新 commit（Task 1-7 各一个，Task 8 无 commit）。

- [ ] **Step 2: push**

```bash
git push origin main
```

- [ ] **Step 3: 监控 CI**

```bash
gh run watch --repo Linuxdazhao/cc_auto_switch
```
Expected: ci.yml 全绿。

- [ ] **Step 4: （可选）发版本**

如果改动够大值得发新版：参考 `CLAUDE.md` 的 release workflow，跑 `./scripts/release.sh`。本次是新功能，按 semver 应该是 minor bump（如 0.1.39 → 0.2.0）；如果 maintainer 偏好把"新功能"也算 patch（项目还在 0.x），就 patch bump。

---

## Plan Self-Review

Spec 覆盖检查：
- ✅ "Daemon 启动时无条件注入 OFFICIAL_UPSTREAM" → Task 3
- ✅ "4 处 official short-circuit 改走 try_resolve_proxy" → Task 5 + Task 6
- ✅ "ANTHROPIC_BASE_URL 设但 token 不设" → Task 1 (with_base_url) + Task 2 (helper)
- ✅ "AliasMap 注入 official" → Task 4
- ✅ "build_official_env 共享 helper" → Task 2
- ✅ "Colors 统一蓝色" → Task 5 (Using official + routing/fallback) + Task 7 (现有非 official 警告)
- ✅ "OFFICIAL_UPSTREAM 常量统一 source of truth" → Task 2
- ✅ "with_base_url 链式方法" → Task 1
- ✅ "保留 settings.remove_anthropic_env()" → Task 5 (代码保留了)
- ✅ "保留 print_version_mismatch_warning" → Task 5 (新加，和非 official 对齐)
- ✅ 错误处理（daemon 停 → 蓝色降级；OAuth 缺失 → 不变；proxy spawn 失败 → 现有兜底）→ Task 8 Step 5 验证；其余分支无新增代码

Spec 未覆盖：无。

类型一致性：
- `EnvironmentConfig::with_base_url(impl Into<String>) -> Self` — Task 1 定义，Task 2 helper 调用 `env.with_base_url(proxy_url)` 其中 `proxy_url: String` ✅
- `OFFICIAL_UPSTREAM: &str` — Task 2 定义为 `&'static str`，Task 3 用 `.to_string()`，Task 4 用 `.to_string()`，类型一致 ✅
- `build_official_env() -> EnvironmentConfig` — Task 2 定义，Task 5/6 调用 ✅
- `ProxyResolution::{Proxied, Direct}` — 现有 enum，Task 2 helper 内部 match ✅

Placeholder 扫描：所有 "TBD"/"TODO"/"similar to" 都没用到，每一步都有 exact code 或 exact command。Task 6 Step 2 有一处"对每一处做相同改造"，给了完整 before/after 模式，可视为通用的，不算 placeholder。Task 3 Step 1 有 fallback ("如果它们没有 Default derive...") — 这是 defensive instruction，因为我没读 types.rs 全部细节，给执行者一个明确的备选指令而不是 TBD。
