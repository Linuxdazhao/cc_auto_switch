# 聚合 Dashboard 请求详情:结构化展示设计

- 日期:2026-05-29
- 范围:`web-aggregate/` 前端(daemon 聚合 dashboard 的请求详情面板)
- 目标:把详情面板里的「JSON 全量 dump」换成对话式结构化展示(LLM prompt / 工具调用 / 响应),并提供 **Structured ⇄ Raw** 切换。

## 背景与现状

点击请求表格的某一行会调用 `/api/requests/{sid}/{seq}`,返回一条序列化的
`CaptureRecord`(`ccs-proxy/src/capture/mod.rs:9`):

- `request: RequestPart` = `{ method, path, headers, body }`,其中 `body` 是标准
  **Anthropic Messages API 请求体**:`model` / `system` / `messages[]` / `tools[]` / 采样参数。
- `response: Option<ResponsePart>` = `{ status, headers, body_reassembled, raw_sse_text, raw_sse_frames_count }`,
  其中 `body_reassembled` 是 **Messages API 响应体**:`content[]`(text / tool_use / thinking 块)/ `stop_reason` / `usage`。
- 顶层还有 `seq` / `session_id` / `request_id` / `started_at` / `ended_at` / `duration_ms` / `ttft_ms` / `usage` / `model` / `error` / `partial`。

当前 `web-aggregate/app.js` 的 `renderDetail()`(约 175 行)把 `request` / `response` / `usage` / `error`
直接 `JSON.stringify(..., null, 2)` 塞进 `<pre>`。`index.html` 顶部已有 Overview / Request / Response
三个 tab,但点击只切 `.active` class,**没有任何渲染逻辑**(app.js:273–278 的处理器是死的)。

## 关键决策(已与用户对齐)

| 维度 | 决定 |
|------|------|
| 布局 | 接上现有三个 tab(Overview / Request / Response)+ 面板头部一个全局 Structured⇄Raw 切换钮,作用于当前 tab |
| 渲染库 | `marked`(md→HTML)、`DOMPurify`(净化)、`highlight.js` + github 浅色主题(代码/JSON 高亮),全部 **CDN 引入** |
| 框架 | **Vue 3**(CDN global 完整构建,含运行时编译器),**只挂在详情面板子树**;表格 / SSE 流 / 侧边过滤保持 vanilla 不动 |
| 引入方式 | CDN `<script>`/`<link>` 标签,版本锁定;无打包器、无构建步骤 |

> 选 Vue 而非 React 的原因:无构建环境下 Vue 3 的 global 构建 `<script>` 一引即用、可写字符串模板;
> React 不带构建要么浏览器内 Babel 转译 JSX(拖慢加载)、要么满屏 `createElement`。

## 架构

### vanilla ↔ Vue 桥接

引入一个 Vue 响应式 store 作为唯一桥梁,vanilla 代码只写这个 store,不直接调用组件方法:

```js
const detailStore = Vue.reactive({
  visible: false,
  target: null,        // { sid, seq }
  loading: false,
  error: null,
  record: null,        // 已加载的 CaptureRecord
  viewMode: 'structured', // 'structured' | 'raw'
  activeTab: 'overview',  // 'overview' | 'request' | 'response'
});
```

- vanilla 的 `selectRow(sid, seq)` 改为:`detailStore.target = { sid, seq }; detailStore.visible = true;`
- Vue 根组件 `watch` `target`,负责 `fetch('/api/requests/{sid}/{seq}')`、设置 `loading/error/record`。
  所有详情相关请求与渲染逻辑都收敛进 Vue。
- 切 tab / 切模式只改 `detailStore`,**不重新请求接口**(record 已缓存)。
- 关闭面板:Vue 渲染 close 钮,点击置 `detailStore.visible = false`。

### Vue 组件树(挂载点 `#detail-panel`)

- `DetailPanel`(根):读 `detailStore`;渲染面板头(标题 + `ToggleButton` + close)、tab 条、当前 tab 内容;`watch(target)` 触发 fetch。
- `OverviewTab`:结构=元信息表(session / seq / model / 时间 / TTFT / usage / error);raw=整条 record 的 `JsonBlock`。
- `RequestTab`:结构=`SystemSection` + `ToolsSection` + `MessageThread`;raw=`request.body` 的 `JsonBlock`。
- `ResponseTab`:结构=assistant `content[]` 块 + stop_reason + usage;raw=`response.body_reassembled` 的 `JsonBlock`(无则回退 `raw_sse_text` 文本)。
- `MessageThread` → 多个 `MessageItem`(按 role 渲染标签 + 内容)。
- `ContentBlock`:按 `block.type` 分发,递归渲染:
  - `text` → `Markdown`
  - `tool_use` → 折叠卡片:工具名 + `JsonBlock(input)`
  - `tool_result` → 折叠卡片:对应 `tool_use_id` + content(字符串走 `Markdown`,数组递归 `ContentBlock`)
  - `thinking` → 弱化/斜体样式块,内文走 `Markdown`
  - `image` → 文本占位 `[image: <media_type>]`(不加载远程 URL,避免 SSRF/追踪;v1 不渲染实图)
  - 未知类型 → `JsonBlock(block)` 兜底
- `SystemSection` / `ToolsSection`:可折叠;system 为字符串或文本块数组;tools 列出 name + description + `JsonBlock(input_schema)`(折叠)。
- `Collapsible`:复用折叠容器(基于 `<details>` 或按钮 + v-show)。
- `Markdown`(prop `text`):`computed` 里 `DOMPurify.sanitize(marked.parse(text || ''))` → `v-html`;通过 `v-highlight` 指令在 mount/update 后对 `pre code` 跑 `hljs.highlightElement`。
- `JsonBlock`(prop `value`):`JSON.stringify(value, null, 2)` 经**文本插值**(非 v-html)填入 `<pre><code class="language-json">`,再 `v-highlight`。
- `ToggleButton`:绑定 `detailStore.viewMode` 两态切换。

## 数据流

1. 点表格行 → vanilla `selectRow` 写 `detailStore.target` + `visible=true`。
2. Vue 根组件 `watch(target)` → fetch 详情 → 写 `record`。
3. `DetailPanel` 据 `activeTab` + `viewMode` 渲染对应 tab。
4. 切 tab / 切模式仅改 store,组件响应式重渲染,无网络请求。

## 优雅降级

- **形状不匹配**:request 结构化前判 `record.request?.body?.messages` 是否为数组;response 判 `body_reassembled?.content` 是否为数组。不是(如 Codex/OpenAI 形状、空体)→ 自动回退 `JsonBlock`。
- **库加载失败(离线/内网)**:用户已接受 CDN 的联网前提。但 app.js 初始化时若检测到 `window.Vue` 未定义,则**不挂 Vue**,详情面板回退到当前的原始 `<pre>` JSON 渲染(保留旧 `renderDetail` 作为 fallback 路径),保证 dashboard 仍可用。

## 安全(XSS)

- 一切**模型产出的文本**渲染前必过 `DOMPurify.sanitize`,只在 `Markdown` 组件里 `v-html`。
- JSON、工具名、tool_use_id、stop_reason 等一律走 Vue **文本插值**(`{{ }}` / textContent),不用 v-html。
- `image` 块仅显示文本占位,不注入任意 `src`。

## CDN 资源(`index.html` `<head>`,版本锁定)

- Vue 3 global 完整构建(含编译器,非 runtime-only):`vue.global.prod.js`
- `marked`(全局 `marked`)
- `DOMPurify`(全局 `DOMPurify`)
- `highlight.js` JS + github 浅色主题 CSS(全局 `hljs`)

> 具体版本号与 jsdelivr 路径在实现时逐一访问确认可解析后再锁定;CDN 脚本不加 `defer`,
> 保证在 body 末尾的 `app.js` 执行前已就绪。

## 改动文件

- `web-aggregate/index.html`:`<head>` 加 4 组 CDN 标签;`#detail-panel` 内部结构改由 Vue 接管(模板写在 app.js 字符串模板里;HTML 仅保留挂载容器)。
- `web-aggregate/app.js`:`selectRow` 改为写 store;新增 Vue app(store + 组件 + 指令);删除/降级旧 `renderDetail` 与死 tab 处理器。
- `web-aggregate/style.css`:消息块 / role 标签 / 工具卡片 / 折叠 / 切换钮 / markdown 正文(code、pre、列表)样式;highlight 主题由 CDN link 引入。
- 服务端无改动:`rust_embed` 仍只嵌入这三个文件(CDN 而非 vendor,不新增静态资源,无需新路由)。

## 测试策略

前端是无打包器的 vanilla/CDN-Vue,仓库目前**没有 JS 测试框架**,引入测试 runner 超出本次范围。
因此采用**手动验证**:

1. `cargo build` 通过(确认 rust_embed 仍正常嵌入改动后的三文件)。
2. 启动 daemon,打开 dashboard,产生若干真实请求。
3. 点击请求,逐项验证:
   - 纯文本用户轮 / assistant 文本轮(markdown 正确渲染,代码块高亮)
   - `tool_use` 轮(工具名 + input JSON 折叠卡片)
   - `tool_result` 轮(对应工具 + 内容)
   - `thinking` 块样式
   - Response tab 的 assistant 内容 + stop_reason + usage
   - 非 Anthropic 形状 / 空 response → 回退 JSON
   - Structured ⇄ Raw 切换在三个 tab 上均生效
   - 关闭/重开、切换不同请求行 record 正确刷新
4. 断网模拟(或 DevTools 阻断 CDN)→ 详情面板回退到原始 `<pre>` JSON,dashboard 不崩。

## 明确不做(YAGNI)

- 不 vendor 库进仓库(用户选 CDN)。
- 不重写表格 / 侧边过滤 / SSE 流(保持 vanilla)。
- 不渲染远程图片实图。
- 不为前端引入 JS 测试框架。
- 不支持 Codex/OpenAI 形状的结构化渲染(走 JSON 回退);后续如需可单独立项。
