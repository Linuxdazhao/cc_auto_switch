# 前端重构设计 — Svelte + Vite + shadcn-svelte

日期：2026-05-29
状态：已批准设计，待评审 spec

## 1. 背景与目标

项目当前有两个 web 前端，都是原生 HTML/CSS/JS、命令式 DOM 操作，且依赖 CDN：

- **`web-aggregate/`**（~1900 行）：聚合 dashboard，主程序 `cc-switch` 通过 rust-embed 内嵌，展示跨 upstream 的 requests / sessions / stats / 详情。是**超集**。
- **`ccs-proxy/web/`**（~174 行）：单实例会话查看器，`ccs-proxy` 内嵌。是 aggregate 的**子集**。

两者加载 Vue 3 / marked / dompurify / highlight.js 但全走 CDN，且并未真正使用框架的响应式，重复且难维护。

**核心目标（四项全选）**：
1. 视觉美化 — 现代、专业的观感，深色模式。
2. 技术栈现代化 — 从命令式 DOM + CDN 转向真正的组件化框架 + 构建打包。
3. 离线/内嵌可靠 — 去掉所有 CDN，资源本地打包内嵌。
4. 功能体验 — 交互、实时刷新、搜索过滤、响应式布局。

## 2. 关键约束：分发模型

（详见记忆 `project-distribution-model`）

- **主程序 `cc-switch`**：不走 `cargo install`，用户通过 brew / 预编译二进制安装；二进制在 **CI（有 Node）** 中构建 → 终端用户机器永不运行 build.rs。
- **`ccs-proxy` crate**：发布到 crates.io，被用户的**其他项目**当 Rust 库依赖（用其 proxy/capture 的 Rust 功能，不用 web UI）。
- **web 前端不随 crate 发布**：发布的 `ccs-proxy` 把 web 资产 `exclude` 出包，web UI 置于 `web-ui` cargo feature 之后、**默认关**。下游消费者拿到纯 Rust、永不需要 Node。

**推论**：因为外部消费者永远不接收也不构建 web 资产，"build.rs 要求 Node 会搞挂 crates.io 消费者"的担忧不成立。Node 只在用户自己的 CI/开发环境出现。

## 3. 技术栈

| 维度 | 选型 |
|---|---|
| 框架 | **Svelte + Vite**（运行时最小、产物最轻，适合内嵌） |
| 组件库 | **shadcn-svelte**（复制式组件，按需 CLI 拉取，代码归己；底层 Tailwind + bits-ui） |
| 样式 | Tailwind CSS（随 shadcn 引入）+ 设计 token + 深色模式 |
| 富文本 | `marked` + `highlight.js` + `dompurify` 改为 npm 依赖，Vite 打包（**去 CDN**） |
| 测试 | Vitest（组件 + api 客户端） |
| 包管理 | pnpm workspace |

## 4. 前端工程结构

顶层 pnpm workspace `web/`：

```
web/
├── packages/
│   ├── ui/          # 设计系统：shadcn-svelte 组件 + Tailwind preset/tokens
│   │                #   + 深色模式 + 共享业务组件
│   │                #   (DataTable, StatCard, FilterPanel, ConversationView…)
│   └── api/         # 共享 TS 类型 + fetch 客户端
│                    #   (/api/health|meta|stats|sessions|requests|stream)
├── apps/
│   ├── aggregate/   # 聚合 dashboard → Vite outDir = web-aggregate/dist/
│   └── proxy/       # 单实例查看器 → Vite outDir = ccs-proxy/web/dist/
├── package.json
└── pnpm-workspace.yaml
```

- 共享 `ui` 包承载 shadcn 组件 + 设计 token + 深色模式，两个 app 复用 → 视觉语言统一。
- `proxy` app 复用 `ui` 的 `ConversationView`/`DataTable` 等组件，是 `aggregate` 的精简版（无跨 upstream 聚合）。
- rust-embed 的 `#[folder]` 改指向各自的 `dist/`（`web-aggregate/dist/`、`ccs-proxy/web/dist/`）。

## 5. 构建衔接（cargo ↔ Vite）

- 引入 `build.rs`，**仅当 `web-ui` cargo feature 启用时**调用 `pnpm`/`vite build`，产物输出到对应 embed 目录。
- `web-ui` feature **默认关**（发布的 ccs-proxy 纯 Rust，零 Node）；你的二进制构建（主程序、`ccs-proxy` 独立 bin）以 `web-ui` 开启。
- feature 关闭时 build.rs 为 no-op → 下游 `cargo build` 不碰 Node。
- `ccs-proxy` 的 `Cargo.toml` 用 `exclude` 把 `web/`、`dist/` 排除出发布包。
- `dist/` **不强制**进仓库（外部不消费 web），仅作本地开发可选缓存。

## 6. 视觉 / UX 升级范围

**视觉层**
- 设计 token：统一配色、间距、圆角、字号阶梯；深色模式（跟随系统 + 手动切换）。
- 用 shadcn 的 `Table`/`Card`/`Badge`/`Button`/`Tabs`/`Select`/`Tooltip` 替换手写 DOM。
- 状态语义色 `Badge`（2xx 绿 / 4xx 黄 / 5xx 红）；等宽数字对齐 token 列；骨架屏 loading；空状态提示。

**信息架构 / 布局**
- 侧栏过滤区可折叠分组（Upstreams / Models / Working dirs / Time）；过滤态 chip 化、一键清除。
- requests / sessions 用 `Tabs`；session 详情用面包屑 + 抽屉(`Sheet`)展示请求详情，弃用 `.hidden` 切 div。
- 响应式：窄屏侧栏收成抽屉。

**功能体验**
- 客户端搜索/过滤即时生效；表格列排序；分页改"加载更多"或虚拟滚动。
- `ConversationView`：markdown + 代码高亮 + 角色气泡，工具调用可折叠。
- **可选**：接 `/api/stream`（SSE）做实时自动刷新 — 标为可选，避免范围膨胀。

## 7. 迁移策略：两个 app 一起重写

1. 搭 `web/` workspace + `ui`/`api` 包骨架；引入 build.rs + `web-ui` feature（旧产物先照常工作）。
2. 用共享组件**同时重写** `aggregate`（超集）与 `proxy`（子集）。
3. 两个 app 一起切换 embed 到新 `dist/`；旧的命令式资产保留至新版验证通过。
4. 验证通过后删除旧 `web-aggregate/*.js`、`ccs-proxy/web/*.js` 等命令式资产。

**测试**
- 前端：Vitest 覆盖组件与 `api` 客户端。
- Rust：build.rs 行为 + `web-ui` feature 开关（开/关都能编译，关时 no-op）。
- 保留现有 CI 关卡（fmt/clippy/test/audit），新增前端 lint + build 一步。

**回滚**
- 切换前保留旧资产；回退只需把 rust-embed `#[folder]` 指回旧目录。

## 8. 范围之外（YAGNI）

- 不引入后端 API 改动（除非 SSE 实时刷新被采纳，且那是可选项）。
- 不做账号/权限/多租户等 dashboard 之外的功能。
- 不在本轮做无关重构。
