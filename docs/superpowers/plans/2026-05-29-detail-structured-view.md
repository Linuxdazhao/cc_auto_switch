# Aggregate Dashboard — Structured Request Detail View Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the daemon aggregate dashboard's raw-JSON request detail panel with a conversation-style structured view (system / tools / message thread / tool calls / thinking / response content), with a per-tab **Structured ⇄ Raw** toggle and graceful fallback when third-party libraries fail to load.

**Architecture:** Mount a Vue 3 island only on `#detail-panel`. The rest of the dashboard (header, sidebar, request table, SSE stream) stays vanilla. A `Vue.reactive` `detailStore` is the only bridge: vanilla `selectRow` writes the store; Vue reads it and owns all fetch + render logic. Libraries (Vue, marked, DOMPurify, highlight.js) are CDN-loaded. If `window.Vue` is absent at boot, fall back to the existing `<pre>`-only renderer so the dashboard still works.

**Tech Stack:** Vue 3 (global prod build, with compiler), marked, DOMPurify, highlight.js + github theme — all via jsdelivr CDN. No bundler, no new build step. Rust side (`src/daemon/aggregate/mod.rs:101-103` `rust_embed`) is untouched: only the existing three files (`index.html`, `app.js`, `style.css`) under `web-aggregate/` change.

**Source spec:** `docs/superpowers/specs/2026-05-29-detail-structured-view-design.md`

---

## File Structure

All changes live in three files under `web-aggregate/` (embedded into the daemon binary at `cargo build` time via `rust_embed` declared at `src/daemon/aggregate/mod.rs:101-103`):

- `web-aggregate/index.html` — add 4 CDN `<link>`/`<script>` tags to `<head>`; replace `#detail-panel` inner markup with an empty Vue mount container.
- `web-aggregate/app.js` — add Vue feature detection; add `detailStore`; add Vue app with all components and the `v-highlight` directive (all templates inline as JS strings); rewrite `selectRow` to write the store; keep old `renderDetail` as the no-Vue fallback path; remove the dead tab click handler.
- `web-aggregate/style.css` — append styles for: panel header, mode toggle, message bubbles, role badges, tool / thinking / collapsible blocks, markdown body (code, pre, lists, blockquotes).

**No new files**, **no new Rust routes**, **no new embedded assets** — the existing router (`src/daemon/aggregate/mod.rs:105-111`) only serves the three names above, and we are not adding any.

### Why one big `app.js`

The aggregate UI is intentionally bundler-free. Splitting components into separate `.js` files would require either ES module imports (forcing the Rust router to serve new paths) or extra `<script>` tags (forcing additions to `index.html` and the router). Keeping everything in `app.js` as inline string templates is the documented design choice in the spec, and it preserves the "three-file embed" simplicity. The file does grow; structure it with clearly labeled sections (`// ===== detailStore =====`, `// ===== components =====`, etc.).

### Why the legacy `renderDetail` survives

Spec § "优雅降级" requires that the dashboard remain usable when CDN scripts fail to load (offline / corporate network blocking jsdelivr). The simplest fallback is to keep the existing `renderDetail` function and call it from `selectRow` whenever `window.Vue` is undefined at boot.

---

## CDN Versions

These are the concrete versions to pin. If a URL fails to resolve at implementation time (e.g., jsdelivr cache miss, library has yanked the version), use the latest matching major from <https://www.jsdelivr.com> and update the plan.

- Vue **3.5.13** global production (with template compiler): `https://cdn.jsdelivr.net/npm/vue@3.5.13/dist/vue.global.prod.js`
- marked **15.0.7**: `https://cdn.jsdelivr.net/npm/marked@15.0.7/marked.min.js`
- DOMPurify **3.2.4**: `https://cdn.jsdelivr.net/npm/dompurify@3.2.4/dist/purify.min.js`
- highlight.js **11.11.1** (cdn-assets bundle, includes common languages):
  - JS: `https://cdn.jsdelivr.net/npm/@highlightjs/cdn-assets@11.11.1/highlight.min.js`
  - CSS (github theme): `https://cdn.jsdelivr.net/npm/@highlightjs/cdn-assets@11.11.1/styles/github.min.css`

Tags must be **synchronous** (no `defer`, no `async`) so `window.Vue`, `window.marked`, `window.DOMPurify`, `window.hljs` are all defined before the bottom-of-body `app.js` runs.

---

## Verification Reality Check

There is no JS test framework in this repo (spec § "测试策略"). Every task ends with **manual browser verification** plus `cargo build` to confirm `rust_embed` still picks up the changed files. The implementer cannot claim a task complete without running the daemon, opening the dashboard, and clicking through.

To launch the daemon and find the dashboard URL during verification:

```bash
cargo build
# Start the daemon. The exact subcommand depends on local setup; check:
cargo run -- --help
# Look for the daemon / aggregate dashboard subcommand and the URL it prints
# on startup (the daemon emits the bound aggregate URL — see
# src/daemon/aggregate/mod.rs:46-47 which binds 127.0.0.1:<port>).
# Open that URL in a browser, generate a few real Claude requests through a
# proxied alias, then click a row in the table.
```

If the implementer cannot get traffic flowing through a real proxy in their environment, they should say so and stop — do **not** claim "structured view works" based on an empty table.

---

## Tasks

### Task 1: Load CDN libraries and add the Vue mount point

**Files:**
- Modify: `web-aggregate/index.html` (`<head>` and `#detail-panel` body)

- [ ] **Step 1: Add CDN tags to `<head>` and convert `#detail-panel` into a Vue mount container**

Replace the entire file with:

```html
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>ccs-daemon aggregate</title>
<link rel="stylesheet" href="/style.css">
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/@highlightjs/cdn-assets@11.11.1/styles/github.min.css">
<script src="https://cdn.jsdelivr.net/npm/vue@3.5.13/dist/vue.global.prod.js"></script>
<script src="https://cdn.jsdelivr.net/npm/marked@15.0.7/marked.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/dompurify@3.2.4/dist/purify.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/@highlightjs/cdn-assets@11.11.1/highlight.min.js"></script>
</head>
<body>
<header id="header">
  <span class="brand">ccs-daemon</span>
  <span id="meta"></span>
</header>
<main id="main">
  <aside id="sidebar">
    <section id="filter-section">
      <h3>Upstreams</h3>
      <div id="upstream-filters"></div>
      <h3>Time</h3>
      <div id="time-filters">
        <button class="time-btn active" data-window="1h">1h</button>
        <button class="time-btn" data-window="24h">24h</button>
        <button class="time-btn" data-window="7d">7d</button>
        <button class="time-btn" data-window="all">all</button>
      </div>
    </section>
    <section id="stats-section">
      <h3>Stats</h3>
      <div id="stats-cards"></div>
    </section>
  </aside>
  <section id="content">
    <table id="request-table">
      <thead>
        <tr>
          <th>Time</th>
          <th>Upstream</th>
          <th>Model</th>
          <th>Tokens</th>
          <th>Status</th>
          <th>Duration</th>
        </tr>
      </thead>
      <tbody id="request-list"></tbody>
    </table>
    <div id="pagination"></div>
    <div id="detail-panel" class="hidden">
      <div id="detail-mount"></div>
    </div>
  </section>
</main>
<script src="/app.js"></script>
</body>
</html>
```

Key changes vs. the current file:
- Four new lines in `<head>` for highlight.js CSS + Vue + marked + DOMPurify + highlight.js.
- `#detail-panel` now contains only `<div id="detail-mount"></div>` (close button, tab bar, and content area are owned by Vue from Task 4 onward; the legacy fallback in `app.js` writes into `#detail-mount` directly via `innerHTML`).
- The outer `#detail-panel` div, its `hidden` class, and its inline `id="close-detail"` button are gone — visibility and the close button are now Vue-driven (`detailStore.visible` + `v-if`). The CSS rule `#detail-panel.hidden { display: none }` remains, but it's now only the *initial* state; once Vue mounts it manages visibility via `v-if`.

- [ ] **Step 2: Rebuild to pick up the embedded HTML change**

Run: `cargo build`
Expected: build succeeds. (If you skip the rebuild, `rust_embed` will serve the stale baked-in copy.)

- [ ] **Step 3: Visual verification — dashboard still loads, CDN scripts resolve**

Launch the daemon, open the dashboard URL in a browser, open DevTools.
- Network tab: confirm all four CDN URLs return 200.
- Console tab: confirm no errors. `window.Vue`, `window.marked`, `window.DOMPurify`, `window.hljs` are all defined (type each in the console).
- Page: header + sidebar + request table still render. Clicking a row does nothing yet (expected — handler not rewritten until Task 3).

If any CDN URL 404s, swap to the latest version per the "CDN Versions" note above and re-verify before continuing.

- [ ] **Step 4: Commit**

```bash
git add web-aggregate/index.html
git commit -m "feat(daemon-ui): load Vue/marked/DOMPurify/highlight.js CDNs and add Vue mount point"
```

---

### Task 2: Append CSS for the structured detail view

**Files:**
- Modify: `web-aggregate/style.css` (append at end)

The new view introduces: a panel header with a Structured/Raw toggle, role-tagged message bubbles, tool-use / tool-result / thinking cards, collapsible containers (system / tools / individual cards), and markdown body styling (code, pre, lists, blockquotes). highlight.js theme is already pulled in by `<link>` from Task 1.

- [ ] **Step 1: Append the following block to `web-aggregate/style.css`**

```css
/* ===== Detail panel — structured view ===== */

#detail-panel .panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 8px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border);
}

#detail-panel .panel-header h3 {
  font-size: 13px;
  font-weight: bold;
  margin: 0;
}

#detail-panel .panel-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

#detail-panel .mode-toggle {
  font-family: var(--font);
  font-size: 11px;
  padding: 3px 8px;
  border: 1px solid var(--border);
  border-radius: 3px;
  background: #fff;
  cursor: pointer;
}

#detail-panel .mode-toggle.raw {
  background: var(--fg);
  color: #fff;
  border-color: var(--fg);
}

#detail-panel .panel-close {
  font-size: 20px;
  line-height: 1;
  border: none;
  background: none;
  cursor: pointer;
  color: var(--muted);
  padding: 0 4px;
}

#detail-panel .status-line {
  padding: 12px;
  color: var(--muted);
  font-size: 12px;
}

#detail-panel .status-line.error {
  color: var(--error);
}

/* Overview metadata table */
#detail-panel dl.meta {
  display: grid;
  grid-template-columns: 100px 1fr;
  gap: 4px 8px;
  font-size: 12px;
  margin: 0 0 12px;
}

#detail-panel dl.meta dt {
  font-weight: bold;
  color: var(--muted);
}

#detail-panel dl.meta dd {
  margin: 0;
  word-break: break-all;
}

/* Section headers (Request tab: System / Tools / Messages) */
#detail-panel .section {
  margin: 12px 0;
}

#detail-panel .section > summary,
#detail-panel .section-title {
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--muted);
  font-weight: bold;
  cursor: pointer;
  padding: 4px 0;
  list-style: none;
}

#detail-panel .section > summary::-webkit-details-marker { display: none; }

#detail-panel .section > summary::before {
  content: '▸ ';
  display: inline-block;
  width: 12px;
  transition: transform 0.1s;
}

#detail-panel .section[open] > summary::before {
  transform: rotate(90deg);
}

/* Message thread */
#detail-panel .message {
  margin: 8px 0;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: #fff;
}

#detail-panel .message.role-user { border-left: 3px solid var(--accent); }
#detail-panel .message.role-assistant { border-left: 3px solid var(--success); }
#detail-panel .message.role-system { border-left: 3px solid var(--muted); }

#detail-panel .role-badge {
  display: inline-block;
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding: 2px 6px;
  border-radius: 3px;
  margin-bottom: 6px;
  color: #fff;
  background: var(--muted);
}

#detail-panel .role-badge.role-user { background: var(--accent); }
#detail-panel .role-badge.role-assistant { background: var(--success); }
#detail-panel .role-badge.role-system { background: var(--muted); }

/* Content blocks inside a message */
#detail-panel .block {
  margin: 6px 0;
}

#detail-panel .block.tool-use,
#detail-panel .block.tool-result {
  border: 1px solid var(--border);
  border-radius: 4px;
  background: var(--sidebar-bg);
  margin: 6px 0;
}

#detail-panel .block.tool-use > summary,
#detail-panel .block.tool-result > summary {
  padding: 6px 8px;
  cursor: pointer;
  font-size: 11px;
  font-weight: bold;
  list-style: none;
}

#detail-panel .block.tool-use > summary::-webkit-details-marker,
#detail-panel .block.tool-result > summary::-webkit-details-marker { display: none; }

#detail-panel .block.tool-use > summary::before,
#detail-panel .block.tool-result > summary::before {
  content: '▸ ';
  display: inline-block;
  width: 12px;
  transition: transform 0.1s;
}

#detail-panel .block.tool-use[open] > summary::before,
#detail-panel .block.tool-result[open] > summary::before {
  transform: rotate(90deg);
}

#detail-panel .block.tool-use .tool-name,
#detail-panel .block.tool-result .tool-ref {
  font-family: var(--font);
  color: var(--accent);
}

#detail-panel .block.tool-use .body,
#detail-panel .block.tool-result .body {
  padding: 0 8px 8px;
}

#detail-panel .block.thinking {
  border-left: 2px solid var(--muted);
  padding: 4px 10px;
  margin: 6px 0;
  font-style: italic;
  color: var(--muted);
  background: rgba(0, 0, 0, 0.02);
}

#detail-panel .block.image-placeholder {
  font-size: 11px;
  color: var(--muted);
  font-style: italic;
  padding: 4px 0;
}

/* Markdown body — applies inside .markdown */
#detail-panel .markdown {
  font-size: 12px;
  line-height: 1.5;
}

#detail-panel .markdown p { margin: 4px 0; }
#detail-panel .markdown ul, #detail-panel .markdown ol { margin: 4px 0 4px 20px; }
#detail-panel .markdown li { margin: 2px 0; }
#detail-panel .markdown blockquote {
  border-left: 3px solid var(--border);
  padding-left: 8px;
  margin: 4px 0;
  color: var(--muted);
}
#detail-panel .markdown code {
  background: var(--sidebar-bg);
  padding: 1px 4px;
  border-radius: 2px;
  font-size: 11px;
}
#detail-panel .markdown pre {
  background: var(--sidebar-bg);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 8px;
  overflow: auto;
  margin: 6px 0;
}
#detail-panel .markdown pre code {
  background: none;
  padding: 0;
  font-size: 11px;
  white-space: pre;
}
#detail-panel .markdown a {
  color: var(--accent);
  text-decoration: underline;
}
#detail-panel .markdown table {
  border-collapse: collapse;
  margin: 6px 0;
}
#detail-panel .markdown th, #detail-panel .markdown td {
  border: 1px solid var(--border);
  padding: 3px 6px;
}

/* JSON block (used in raw mode and inside tool cards) */
#detail-panel .json-block {
  background: var(--sidebar-bg);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 8px;
  overflow: auto;
  max-height: 500px;
  font-size: 11px;
  white-space: pre;
  margin: 4px 0;
}

/* Tool descriptions list (Tools section) */
#detail-panel .tool-entry {
  padding: 6px 0;
  border-bottom: 1px dashed var(--border);
}
#detail-panel .tool-entry:last-child { border-bottom: none; }
#detail-panel .tool-entry .tool-desc {
  color: var(--muted);
  margin: 2px 0 4px;
}

/* Tab bar — keep existing .tab styling but ensure it lives inside Vue panel */
#detail-panel .detail-tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 12px;
}
```

- [ ] **Step 2: Rebuild**

Run: `cargo build`
Expected: build succeeds.

- [ ] **Step 3: Visual verification**

Reload the dashboard. The page should still render with no visual regressions in header / sidebar / table (the new rules are all scoped under `#detail-panel`, so nothing outside is affected). The detail panel itself is still inert at this point — only the styles are loaded.

- [ ] **Step 4: Commit**

```bash
git add web-aggregate/style.css
git commit -m "feat(daemon-ui): add CSS for structured detail panel (messages, tool cards, markdown)"
```

---

### Task 3: Replace `selectRow`, add Vue feature detection, set up `detailStore`, mount a stub Vue app

**Files:**
- Modify: `web-aggregate/app.js` (replace `selectRow`, delete dead tab handler & close handler, append new Vue scaffolding)

The goal of this task is the smallest possible Vue wiring: the store, root component, mount call, plus the no-Vue fallback path. The actual structured rendering arrives in Tasks 4–10.

- [ ] **Step 1: Replace `selectRow` (currently lines 161-173) with a store-writing variant, and delete the dead tab/close handlers (currently lines 273-283)**

In `web-aggregate/app.js`, locate the existing `selectRow` function and replace it with:

```javascript
async function selectRow(sid, seq) {
  // Vue path: write the store; Vue components handle fetch + render.
  if (window.__detailStore) {
    window.__detailStore.target = { sid, seq };
    window.__detailStore.visible = true;
    return;
  }
  // Fallback (no Vue): use the legacy renderer below.
  legacySelectRow(sid, seq);
}

async function legacySelectRow(sid, seq) {
  const panel = document.getElementById('detail-panel');
  const mount = document.getElementById('detail-mount');
  panel.classList.remove('hidden');
  mount.innerHTML = '<button class="panel-close" onclick="document.getElementById(\'detail-panel\').classList.add(\'hidden\')">&times;</button><div class="status-line">Loading...</div>';
  try {
    const resp = await fetch(`/api/requests/${sid}/${seq}`);
    const data = await resp.json();
    legacyRenderDetail(data);
  } catch (e) {
    mount.innerHTML = '<button class="panel-close" onclick="document.getElementById(\'detail-panel\').classList.add(\'hidden\')">&times;</button><div class="status-line error">Failed to load request detail.</div>';
  }
}

function legacyRenderDetail(data) {
  const mount = document.getElementById('detail-mount');
  mount.innerHTML = `
    <button class="panel-close" onclick="document.getElementById('detail-panel').classList.add('hidden')">&times;</button>
    <dl>
      <dt>Session</dt><dd>${data.session_id}</dd>
      <dt>Seq</dt><dd>${data.seq}</dd>
      <dt>Model</dt><dd>${data.model || '—'}</dd>
      <dt>Started</dt><dd>${data.started_at}</dd>
      <dt>Duration</dt><dd>${data.duration_ms ? data.duration_ms + 'ms' : '—'}</dd>
      <dt>TTFT</dt><dd>${data.ttft_ms ? data.ttft_ms + 'ms' : '—'}</dd>
      <dt>Request ID</dt><dd>${data.request_id || '—'}</dd>
    </dl>
    ${data.usage ? `<h4 style="margin-top:12px">Usage</h4><pre>${JSON.stringify(data.usage, null, 2)}</pre>` : ''}
    <h4 style="margin-top:12px">Request</h4>
    <pre>${JSON.stringify(data.request, null, 2)}</pre>
    ${data.response ? `<h4 style="margin-top:12px">Response</h4><pre>${JSON.stringify(data.response, null, 2)}</pre>` : ''}
    ${data.error ? `<h4 style="margin-top:12px">Error</h4><pre>${JSON.stringify(data.error, null, 2)}</pre>` : ''}
  `;
}
```

Then delete the old `renderDetail` function entirely (currently lines 175-193 — its logic now lives in `legacyRenderDetail`).

Then delete this dead block (currently lines 273-278):

```javascript
// Detail panel tabs
document.querySelectorAll('#detail-tabs .tab').forEach(tab => {
  tab.addEventListener('click', () => {
    document.querySelector('#detail-tabs .tab.active')?.classList.remove('active');
    tab.classList.add('active');
  });
});
```

And delete the old close-button handler (currently lines 280-283):

```javascript
// Close detail
document.getElementById('close-detail')?.addEventListener('click', () => {
  document.getElementById('detail-panel').classList.add('hidden');
});
```

Both are no-ops now that those DOM nodes are gone (from Task 1's HTML rewrite).

- [ ] **Step 2: Append the Vue bootstrap block at the end of `app.js` (after `init();`)**

```javascript
// ===== Vue detail panel =====
// If Vue failed to load (offline / CDN blocked), this whole block is skipped
// and selectRow() falls through to legacySelectRow().
(function mountDetailPanel() {
  if (!window.Vue) {
    console.warn('[ccs-daemon] Vue not loaded; detail panel falls back to JSON-only renderer.');
    return;
  }
  const { createApp, reactive, computed, watch, ref, h } = window.Vue;

  const detailStore = reactive({
    visible: false,
    target: null,        // { sid, seq } or null
    loading: false,
    error: null,         // string or null
    record: null,        // CaptureRecord or null
    viewMode: 'structured', // 'structured' | 'raw'
    activeTab: 'overview',  // 'overview' | 'request' | 'response'
  });
  window.__detailStore = detailStore;

  // Root component — full template appears in Task 4.
  const DetailPanel = {
    setup() {
      return { store: detailStore };
    },
    template: `
      <div v-if="store.visible" class="detail-root">
        <div class="status-line">Vue mounted (stub) — sid={{ store.target?.sid }} seq={{ store.target?.seq }}</div>
        <button class="panel-close" @click="store.visible = false">&times;</button>
      </div>
    `,
  };

  createApp(DetailPanel).mount('#detail-mount');

  // Toggle the legacy .hidden class on the outer panel based on store.visible
  // (the outer #detail-panel still owns positioning/overflow CSS).
  watch(() => detailStore.visible, (v) => {
    const panel = document.getElementById('detail-panel');
    if (!panel) return;
    if (v) panel.classList.remove('hidden');
    else panel.classList.add('hidden');
  });
})();
```

- [ ] **Step 3: Rebuild**

Run: `cargo build`
Expected: build succeeds.

- [ ] **Step 4: Visual verification — Vue path**

Reload the dashboard.
- Console: should print no errors. Optionally type `window.__detailStore` — it should be a reactive proxy object.
- Click any request row. The panel should slide in and show: `"Vue mounted (stub) — sid=<some-id> seq=<n>"` plus a working `×` close button.
- Clicking `×` closes the panel; clicking another row reopens it with new sid/seq.

- [ ] **Step 5: Visual verification — fallback path**

In DevTools Network tab, block `cdn.jsdelivr.net` (or any of the four CDN URLs). Hard-reload.
- Console: warning `[ccs-daemon] Vue not loaded; detail panel falls back to JSON-only renderer.`
- Click a row. Legacy JSON `<pre>` view appears with a working `×` close button. This proves the fallback path still works.

Unblock the CDN before continuing.

- [ ] **Step 6: Commit**

```bash
git add web-aggregate/app.js
git commit -m "feat(daemon-ui): wire Vue island for detail panel with no-Vue fallback"
```

---

### Task 4: Build the real `DetailPanel` root — fetch, loading/error states, tab bar, mode toggle

**Files:**
- Modify: `web-aggregate/app.js` (replace the stub `DetailPanel` from Task 3)

- [ ] **Step 1: Replace the stub `DetailPanel` object with the full root component**

Inside the `mountDetailPanel` IIFE, replace the `const DetailPanel = { ... }` definition with:

```javascript
  const DetailPanel = {
    setup() {
      // Fetch whenever target changes.
      watch(() => detailStore.target, async (target) => {
        if (!target) return;
        detailStore.loading = true;
        detailStore.error = null;
        detailStore.record = null;
        try {
          const resp = await fetch(`/api/requests/${target.sid}/${target.seq}`);
          if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
          detailStore.record = await resp.json();
        } catch (e) {
          detailStore.error = String(e.message || e);
        } finally {
          detailStore.loading = false;
        }
      });

      return { store: detailStore };
    },
    template: `
      <div v-if="store.visible" class="detail-root">
        <div class="panel-header">
          <h3>Request {{ store.target?.seq }}</h3>
          <div class="panel-actions">
            <button
              class="mode-toggle"
              :class="{ raw: store.viewMode === 'raw' }"
              @click="store.viewMode = store.viewMode === 'structured' ? 'raw' : 'structured'"
            >{{ store.viewMode === 'structured' ? 'Structured' : 'Raw' }}</button>
            <button class="panel-close" @click="store.visible = false">&times;</button>
          </div>
        </div>
        <div class="detail-tabs">
          <button
            v-for="t in ['overview', 'request', 'response']"
            :key="t"
            class="tab"
            :class="{ active: store.activeTab === t }"
            @click="store.activeTab = t"
          >{{ t[0].toUpperCase() + t.slice(1) }}</button>
        </div>
        <div v-if="store.loading" class="status-line">Loading…</div>
        <div v-else-if="store.error" class="status-line error">Failed to load: {{ store.error }}</div>
        <div v-else-if="store.record">
          <div v-if="store.activeTab === 'overview'" class="status-line">[overview placeholder — Task 6]</div>
          <div v-else-if="store.activeTab === 'request'" class="status-line">[request placeholder — Task 9]</div>
          <div v-else-if="store.activeTab === 'response'" class="status-line">[response placeholder — Task 10]</div>
        </div>
      </div>
    `,
  };
```

- [ ] **Step 2: Rebuild**

Run: `cargo build`
Expected: build succeeds.

- [ ] **Step 3: Visual verification**

Reload, click a row.
- Header shows `Request <seq>`, a `Structured` mode toggle, and `×` close.
- Tab bar shows Overview / Request / Response with Overview active.
- Briefly see `Loading…`, then the placeholder for the active tab.
- Clicking another tab switches the placeholder without re-fetching (watch the Network tab: only one `/api/requests/...` call per row click).
- Clicking the mode toggle flips the label between `Structured` and `Raw` and applies the `.raw` class (button background turns dark).
- Clicking another row updates the header and re-fetches.

- [ ] **Step 4: Commit**

```bash
git add web-aggregate/app.js
git commit -m "feat(daemon-ui): DetailPanel root with fetch/loading/error/tabs/mode toggle"
```

---

### Task 5: Add the `Markdown` component, `JsonBlock` component, and `v-highlight` directive

**Files:**
- Modify: `web-aggregate/app.js` (add inside the `mountDetailPanel` IIFE, before `const DetailPanel`)

These three primitives are used by every later task. Markdown sanitizes once via DOMPurify before `v-html`; `JsonBlock` always uses text interpolation (never `v-html`); `v-highlight` runs `hljs.highlightElement` on every `pre code` inside the bound element.

- [ ] **Step 1: Inside `mountDetailPanel`, before `const DetailPanel`, add the primitives**

```javascript
  // v-highlight: applies hljs to every <pre><code> inside the bound element.
  // Safe to call repeatedly (hljs marks elements with data-highlighted).
  const highlightDirective = {
    mounted(el) { highlightAll(el); },
    updated(el) { highlightAll(el); },
  };
  function highlightAll(el) {
    if (!window.hljs) return;
    el.querySelectorAll('pre code').forEach((node) => {
      // hljs ≥10 sets data-highlighted="yes" after highlighting; skip re-runs.
      if (node.dataset.highlighted === 'yes') return;
      try { window.hljs.highlightElement(node); } catch (_) { /* ignore */ }
    });
  }

  const Markdown = {
    props: { text: { type: String, default: '' } },
    directives: { highlight: highlightDirective },
    computed: {
      html() {
        if (!window.marked || !window.DOMPurify) {
          // Libraries failed: fall back to plain text rendering.
          const div = document.createElement('div');
          div.textContent = this.text || '';
          return div.innerHTML;
        }
        const raw = window.marked.parse(this.text || '');
        return window.DOMPurify.sanitize(raw);
      },
    },
    template: `<div class="markdown" v-highlight v-html="html"></div>`,
  };

  const JsonBlock = {
    props: { value: { required: true } },
    directives: { highlight: highlightDirective },
    computed: {
      text() {
        try { return JSON.stringify(this.value, null, 2); }
        catch (_) { return String(this.value); }
      },
    },
    // Text interpolation only — never v-html for JSON content.
    template: `<pre class="json-block" v-highlight><code class="language-json">{{ text }}</code></pre>`,
  };
```

- [ ] **Step 2: Register the primitives on the Vue app and add them as global components**

Replace the `createApp(DetailPanel).mount('#detail-mount');` line with:

```javascript
  const app = createApp(DetailPanel);
  app.component('Markdown', Markdown);
  app.component('JsonBlock', JsonBlock);
  app.directive('highlight', highlightDirective);
  app.mount('#detail-mount');
```

- [ ] **Step 3: Temporary smoke test — wire JsonBlock into the placeholder area**

In the `DetailPanel` template from Task 4, replace this line (inside the overview branch):

```html
<div v-if="store.activeTab === 'overview'" class="status-line">[overview placeholder — Task 6]</div>
```

with:

```html
<div v-if="store.activeTab === 'overview'">
  <Markdown text="## Smoke test\n\nThis is **bold**, *italic*, and `inline code`.\n\n```js\nfunction hi() { return 42; }\n```" />
  <JsonBlock :value="store.record" />
</div>
```

(This is throw-away wiring; Task 6 replaces it.)

- [ ] **Step 4: Rebuild and visually verify**

Run: `cargo build`
Expected: build succeeds.

Reload, click a row, stay on Overview.
- The markdown sample renders with bold, italic, inline code, and a syntax-highlighted JS code block.
- Below it, the full record renders as pretty-printed JSON with highlight.js coloring (keys vs strings vs numbers).
- DevTools console: no errors, no `[Vue warn]` about template compilation.

- [ ] **Step 5: Revert the smoke-test wiring back to the placeholder**

Re-replace the overview branch with:

```html
<div v-if="store.activeTab === 'overview'" class="status-line">[overview placeholder — Task 6]</div>
```

- [ ] **Step 6: Commit**

```bash
git add web-aggregate/app.js
git commit -m "feat(daemon-ui): add Markdown + JsonBlock components and v-highlight directive"
```

---

### Task 6: Implement `OverviewTab`

**Files:**
- Modify: `web-aggregate/app.js` (add component inside `mountDetailPanel`, register, swap into `DetailPanel`)

- [ ] **Step 1: Add `OverviewTab` inside `mountDetailPanel` (alongside `Markdown` / `JsonBlock`)**

```javascript
  const OverviewTab = {
    props: { record: { required: true } },
    setup(props) {
      const fmtMs = (n) => (n == null ? '—' : `${n}ms`);
      const fmtTokens = (u) => {
        if (!u) return '—';
        const parts = [];
        if (u.input_tokens != null) parts.push(`in=${u.input_tokens}`);
        if (u.output_tokens != null) parts.push(`out=${u.output_tokens}`);
        if (u.cache_creation_input_tokens) parts.push(`cache_create=${u.cache_creation_input_tokens}`);
        if (u.cache_read_input_tokens) parts.push(`cache_read=${u.cache_read_input_tokens}`);
        return parts.join(' · ') || '—';
      };
      return { store: detailStore, fmtMs, fmtTokens };
    },
    template: `
      <div v-if="store.viewMode === 'structured'">
        <dl class="meta">
          <dt>Session</dt><dd>{{ record.session_id }}</dd>
          <dt>Seq</dt><dd>{{ record.seq }}</dd>
          <dt>Request ID</dt><dd>{{ record.request_id || '—' }}</dd>
          <dt>Model</dt><dd>{{ record.model || '—' }}</dd>
          <dt>Started</dt><dd>{{ record.started_at }}</dd>
          <dt>Ended</dt><dd>{{ record.ended_at || '—' }}</dd>
          <dt>Duration</dt><dd>{{ fmtMs(record.duration_ms) }}</dd>
          <dt>TTFT</dt><dd>{{ fmtMs(record.ttft_ms) }}</dd>
          <dt>Usage</dt><dd>{{ fmtTokens(record.usage) }}</dd>
          <dt v-if="record.partial">Partial</dt><dd v-if="record.partial">yes</dd>
        </dl>
        <div v-if="record.error">
          <div class="section-title">Error</div>
          <JsonBlock :value="record.error" />
        </div>
      </div>
      <JsonBlock v-else :value="record" />
    `,
  };
```

- [ ] **Step 2: Register `OverviewTab` on the app**

In the `app.component(...)` block, add:

```javascript
  app.component('OverviewTab', OverviewTab);
```

- [ ] **Step 3: Use `OverviewTab` in `DetailPanel`**

In `DetailPanel.template`, replace this line:

```html
<div v-if="store.activeTab === 'overview'" class="status-line">[overview placeholder — Task 6]</div>
```

with:

```html
<OverviewTab v-if="store.activeTab === 'overview'" :record="store.record" />
```

- [ ] **Step 4: Rebuild and verify**

Run: `cargo build`
Expected: build succeeds.

Reload, click a row.
- Overview tab shows a 2-column metadata grid: session, seq, request_id, model, started, ended, duration, TTFT, usage. Values are correct and missing fields show `—`.
- If the record has an error, an Error subsection with a JSON block appears.
- Click the `Structured`/`Raw` toggle: the Raw view shows the entire `CaptureRecord` as one syntax-highlighted JSON block.
- For a request with non-empty usage, the usage line reads e.g. `in=12 · out=34`.

- [ ] **Step 5: Commit**

```bash
git add web-aggregate/app.js
git commit -m "feat(daemon-ui): OverviewTab with metadata grid and raw JSON mode"
```

---

### Task 7: Implement `ContentBlock` (recursive) + tool / thinking / image subcomponents

**Files:**
- Modify: `web-aggregate/app.js` (add inside `mountDetailPanel`, register)

`ContentBlock` is the dispatcher for one element of an Anthropic Messages API `content[]` array. It dispatches by `block.type` and is recursive (a `tool_result` can contain a nested array of `ContentBlock`s).

- [ ] **Step 1: Add `ContentBlock` inside `mountDetailPanel`**

```javascript
  const ContentBlock = {
    name: 'ContentBlock',
    props: { block: { required: true } },
    setup() { return { store: detailStore }; },
    template: `
      <div class="block" :class="blockClass">
        <!-- text -->
        <Markdown v-if="block.type === 'text'" :text="block.text || ''" />

        <!-- tool_use: <details> collapsible -->
        <details v-else-if="block.type === 'tool_use'" class="block tool-use" open>
          <summary>
            <span class="tool-name">{{ block.name }}</span>
            <span v-if="block.id" style="color: var(--muted); font-weight: normal;"> · {{ block.id }}</span>
          </summary>
          <div class="body">
            <JsonBlock :value="block.input ?? {}" />
          </div>
        </details>

        <!-- tool_result: id + nested content -->
        <details v-else-if="block.type === 'tool_result'" class="block tool-result" open>
          <summary>
            tool_result
            <span v-if="block.tool_use_id" class="tool-ref"> · {{ block.tool_use_id }}</span>
            <span v-if="block.is_error" style="color: var(--error); font-weight: normal;"> · error</span>
          </summary>
          <div class="body">
            <template v-if="typeof block.content === 'string'">
              <Markdown :text="block.content" />
            </template>
            <template v-else-if="Array.isArray(block.content)">
              <ContentBlock v-for="(child, i) in block.content" :key="i" :block="child" />
            </template>
            <JsonBlock v-else :value="block.content ?? null" />
          </div>
        </details>

        <!-- thinking -->
        <div v-else-if="block.type === 'thinking'" class="block thinking">
          <Markdown :text="block.thinking || ''" />
        </div>

        <!-- image: text placeholder only (no SSRF, no remote loads) -->
        <div v-else-if="block.type === 'image'" class="block image-placeholder">
          [image: {{ block.source?.media_type || 'unknown' }}]
        </div>

        <!-- unknown type -->
        <JsonBlock v-else :value="block" />
      </div>
    `,
    computed: {
      blockClass() {
        return this.block && this.block.type ? `type-${this.block.type}` : 'type-unknown';
      },
    },
  };
```

- [ ] **Step 2: Register `ContentBlock` on the app**

In the `app.component(...)` block, add:

```javascript
  app.component('ContentBlock', ContentBlock);
```

- [ ] **Step 3: Rebuild and smoke-test**

Run: `cargo build`
Expected: build succeeds.

(No visible change yet — `ContentBlock` is consumed in Task 8 and Task 10. The build proves the template compiles and no Vue warnings appear in the console on next reload.)

- [ ] **Step 4: Commit**

```bash
git add web-aggregate/app.js
git commit -m "feat(daemon-ui): ContentBlock dispatcher (text/tool_use/tool_result/thinking/image)"
```

---

### Task 8: Implement `MessageItem`, `MessageThread`, `SystemSection`, `ToolsSection`

**Files:**
- Modify: `web-aggregate/app.js` (add inside `mountDetailPanel`, register)

- [ ] **Step 1: Add the four components inside `mountDetailPanel`**

```javascript
  const MessageItem = {
    props: { message: { required: true } },
    template: `
      <div class="message" :class="'role-' + (message.role || 'unknown')">
        <span class="role-badge" :class="'role-' + (message.role || 'unknown')">{{ message.role || 'unknown' }}</span>
        <template v-if="typeof message.content === 'string'">
          <Markdown :text="message.content" />
        </template>
        <template v-else-if="Array.isArray(message.content)">
          <ContentBlock v-for="(block, i) in message.content" :key="i" :block="block" />
        </template>
        <JsonBlock v-else :value="message.content ?? null" />
      </div>
    `,
  };

  const MessageThread = {
    props: { messages: { type: Array, required: true } },
    template: `
      <div class="message-thread">
        <MessageItem v-for="(m, i) in messages" :key="i" :message="m" />
      </div>
    `,
  };

  const SystemSection = {
    props: { system: { required: false, default: null } },
    computed: {
      // system can be a string OR an array of {type:'text', text:string} blocks
      // OR an array of arbitrary content blocks. Normalize to an array of blocks.
      blocks() {
        if (this.system == null) return [];
        if (typeof this.system === 'string') return [{ type: 'text', text: this.system }];
        if (Array.isArray(this.system)) return this.system;
        return [];
      },
    },
    template: `
      <details v-if="blocks.length" class="section" open>
        <summary>System</summary>
        <ContentBlock v-for="(b, i) in blocks" :key="i" :block="b" />
      </details>
    `,
  };

  const ToolsSection = {
    props: { tools: { type: Array, required: false, default: () => [] } },
    template: `
      <details v-if="tools && tools.length" class="section">
        <summary>Tools ({{ tools.length }})</summary>
        <div class="tool-entry" v-for="(tool, i) in tools" :key="i">
          <strong>{{ tool.name }}</strong>
          <div v-if="tool.description" class="tool-desc">{{ tool.description }}</div>
          <details v-if="tool.input_schema">
            <summary style="cursor: pointer; font-size: 11px; color: var(--muted);">input_schema</summary>
            <JsonBlock :value="tool.input_schema" />
          </details>
        </div>
      </details>
    `,
  };
```

- [ ] **Step 2: Register on the app**

In the `app.component(...)` block, add:

```javascript
  app.component('MessageItem', MessageItem);
  app.component('MessageThread', MessageThread);
  app.component('SystemSection', SystemSection);
  app.component('ToolsSection', ToolsSection);
```

- [ ] **Step 3: Rebuild**

Run: `cargo build`
Expected: build succeeds. (Still no visible change — consumed in Task 9.)

- [ ] **Step 4: Commit**

```bash
git add web-aggregate/app.js
git commit -m "feat(daemon-ui): MessageItem/MessageThread/SystemSection/ToolsSection components"
```

---

### Task 9: Implement `RequestTab` with Anthropic-shape detection and JSON fallback

**Files:**
- Modify: `web-aggregate/app.js` (add inside `mountDetailPanel`, register, swap into `DetailPanel`)

- [ ] **Step 1: Add `RequestTab` inside `mountDetailPanel`**

```javascript
  const RequestTab = {
    props: { record: { required: true } },
    setup() { return { store: detailStore }; },
    computed: {
      body() { return this.record?.request?.body ?? null; },
      // Anthropic Messages API shape detection: body.messages must be an array.
      isAnthropicShape() {
        return this.body && Array.isArray(this.body.messages);
      },
    },
    template: `
      <div v-if="store.viewMode === 'structured'">
        <template v-if="isAnthropicShape">
          <SystemSection :system="body.system" />
          <ToolsSection :tools="body.tools || []" />
          <div class="section-title">Messages ({{ body.messages.length }})</div>
          <MessageThread :messages="body.messages" />
        </template>
        <template v-else>
          <div class="status-line">Non-Anthropic shape — showing raw body.</div>
          <JsonBlock :value="body" />
        </template>
      </div>
      <JsonBlock v-else :value="body" />
    `,
  };
```

- [ ] **Step 2: Register and wire into `DetailPanel`**

In the `app.component(...)` block, add:

```javascript
  app.component('RequestTab', RequestTab);
```

In `DetailPanel.template`, replace this line:

```html
<div v-else-if="store.activeTab === 'request'" class="status-line">[request placeholder — Task 9]</div>
```

with:

```html
<RequestTab v-else-if="store.activeTab === 'request'" :record="store.record" />
```

- [ ] **Step 3: Rebuild and verify**

Run: `cargo build`
Expected: build succeeds.

Reload, click a row, switch to **Request** tab. For a typical Anthropic request:
- A collapsible **System** section appears at the top (open) if a system prompt is present; clicking the summary collapses/expands it.
- A collapsible **Tools (N)** section appears (collapsed by default) if the request includes tools; expanding shows each tool's name, description, and a nested collapsible `input_schema` JsonBlock.
- Below that, a **Messages (N)** header and the message thread: each message has a colored role badge (user=blue, assistant=green, system=grey), markdown rendered for text content, and tool_use / tool_result / thinking blocks rendered as cards per Task 7.
- Toggle to **Raw**: just the `request.body` JSON.
- Click on a non-Anthropic request (e.g. one captured from a Codex shape, or if you have an obviously different body): the structured view shows the "Non-Anthropic shape" notice + JSON block, and Raw is the same JSON block. If you don't have a non-Anthropic request in the table, you can temporarily test by typing in DevTools: `window.__detailStore.record.request.body = { foo: 'bar' }` — the view should immediately switch to the fallback (then restore by clicking the row again).

- [ ] **Step 4: Commit**

```bash
git add web-aggregate/app.js
git commit -m "feat(daemon-ui): RequestTab with structured Anthropic view and JSON fallback"
```

---

### Task 10: Implement `ResponseTab` with content-block view, stop_reason, usage, and SSE/JSON fallback

**Files:**
- Modify: `web-aggregate/app.js` (add inside `mountDetailPanel`, register, swap into `DetailPanel`)

- [ ] **Step 1: Add `ResponseTab` inside `mountDetailPanel`**

```javascript
  const ResponseTab = {
    props: { record: { required: true } },
    setup() { return { store: detailStore }; },
    computed: {
      response() { return this.record?.response ?? null; },
      reassembled() { return this.response?.body_reassembled ?? null; },
      content() {
        return Array.isArray(this.reassembled?.content) ? this.reassembled.content : null;
      },
      stopReason() { return this.reassembled?.stop_reason ?? null; },
      respUsage() { return this.reassembled?.usage ?? null; },
      rawSse() { return this.response?.raw_sse_text ?? null; },
    },
    template: `
      <div v-if="!response" class="status-line">No response captured.</div>
      <div v-else-if="store.viewMode === 'structured'">
        <template v-if="content">
          <div class="section-title">Content blocks ({{ content.length }})</div>
          <div class="message role-assistant">
            <span class="role-badge role-assistant">assistant</span>
            <ContentBlock v-for="(block, i) in content" :key="i" :block="block" />
          </div>
          <dl class="meta" style="margin-top: 12px;">
            <dt>Status</dt><dd>{{ response.status }}</dd>
            <dt>Stop reason</dt><dd>{{ stopReason || '—' }}</dd>
            <dt>SSE frames</dt><dd>{{ response.raw_sse_frames_count }}</dd>
          </dl>
          <div v-if="respUsage">
            <div class="section-title">Response usage</div>
            <JsonBlock :value="respUsage" />
          </div>
        </template>
        <template v-else-if="reassembled">
          <div class="status-line">Non-Anthropic response shape — showing raw body.</div>
          <JsonBlock :value="reassembled" />
        </template>
        <template v-else-if="rawSse">
          <div class="status-line">No reassembled body — showing raw SSE.</div>
          <pre class="json-block">{{ rawSse }}</pre>
        </template>
        <div v-else class="status-line">Empty response body.</div>
      </div>
      <div v-else>
        <JsonBlock v-if="reassembled" :value="reassembled" />
        <pre v-else-if="rawSse" class="json-block">{{ rawSse }}</pre>
        <div v-else class="status-line">Empty response body.</div>
      </div>
    `,
  };
```

- [ ] **Step 2: Register and wire into `DetailPanel`**

In the `app.component(...)` block, add:

```javascript
  app.component('ResponseTab', ResponseTab);
```

In `DetailPanel.template`, replace this line:

```html
<div v-else-if="store.activeTab === 'response'" class="status-line">[response placeholder — Task 10]</div>
```

with:

```html
<ResponseTab v-else-if="store.activeTab === 'response'" :record="store.record" />
```

- [ ] **Step 3: Rebuild and verify**

Run: `cargo build`
Expected: build succeeds.

Reload, click a completed request row, switch to **Response** tab.
- Structured: a "Content blocks (N)" header, then an assistant-colored message bubble containing each content block (text rendered as markdown, tool_use as a collapsible card, thinking as italic muted text). Below, a metadata grid with Status / Stop reason / SSE frames. If `usage` is present in the reassembled body, a separate Response-usage JSON block follows.
- Raw: the entire `body_reassembled` as a JSON block, falling back to the raw SSE text if no reassembled body.
- Click a row whose response is missing (still-running request, or an upstream error before any response bytes): shows "No response captured." or "Empty response body." depending on which field is missing.

- [ ] **Step 4: Commit**

```bash
git add web-aggregate/app.js
git commit -m "feat(daemon-ui): ResponseTab with content blocks, stop_reason, usage, and SSE fallback"
```

---

### Task 11: End-to-end manual verification pass + final commit

This task is the **explicit success criterion** for the feature. The previous tasks each verified one slice; this task replays spec § "测试策略" in full and is the only point at which the feature can be claimed done.

**Files:**
- No code changes (verification + cleanup commit only)

- [ ] **Step 1: Rebuild from a clean state**

```bash
cargo clean -p cc-switch
cargo build
```

Expected: build succeeds.

- [ ] **Step 2: Generate a varied request corpus**

Start the daemon, route enough real Claude traffic through it to cover all of these in the request table:
- A pure-text user turn (single string `content`).
- A user turn whose `content` is an array including a `text` block (e.g. an Anthropic SDK call with multi-modal-shaped input).
- An assistant turn that returns a `tool_use` block.
- A follow-up user turn containing a `tool_result` block referencing the prior `tool_use_id` — both string content and array content variants if you can produce both.
- An assistant turn containing a `thinking` block (use a model that emits it, e.g. extended-thinking variant of Claude).
- A request that errored upstream (kill the network mid-flight, or hit an invalid model name) to populate `record.error` and possibly leave `response` empty.
- An in-flight / partial request if the capture layer surfaces one — verify `partial: true` renders.

- [ ] **Step 3: For each row, click through and walk this checklist**

For each request, with **both** Structured and Raw mode toggled at least once on each tab:

- [ ] **Overview tab** — metadata grid populates correctly; missing values show `—`; usage line shows `in=N · out=N` with cache fields if present; error subsection appears only when `record.error` is set; partial row appears only when `record.partial`.
- [ ] **Request tab — Anthropic shape** — System (if present) renders as markdown (collapsible, open by default); Tools (if present) renders collapsed with name + description + collapsible `input_schema`; Messages thread renders one bubble per message with the right role badge color; markdown text renders with code-block highlighting; tool_use cards show tool name + id + JSON input; tool_result cards show id, error flag if `is_error`, and nested rendering of content (string → markdown, array → nested ContentBlocks).
- [ ] **Request tab — non-Anthropic / empty body** — fallback notice + JSON block appears, no Vue errors in console.
- [ ] **Response tab** — assistant-colored bubble lists content blocks; thinking blocks render italic/muted; stop_reason and SSE frame count appear; response usage renders when present; missing response shows "No response captured.".
- [ ] **Response tab — raw SSE fallback** — for a request where reassembly failed, the raw SSE text shows in a `<pre>` block.
- [ ] **Structured ⇄ Raw toggle** — switching applies to whichever tab is active and persists across tab changes (toggle is global per spec).
- [ ] **Tab switching** — switching tabs triggers **no** network requests (watch the Network tab). Only clicking a new row does.
- [ ] **Re-open / row change** — clicking another row replaces the content cleanly; clicking the same row twice still works.
- [ ] **Close** — `×` hides the panel; clicking another row re-opens it.

- [ ] **Step 4: CDN-blocked fallback test**

In DevTools Network tab, block `cdn.jsdelivr.net`. Hard-reload the dashboard.
- Console warning: `[ccs-daemon] Vue not loaded; detail panel falls back to JSON-only renderer.`
- Click a row — the legacy `<pre>` JSON view appears (same layout as before this feature), with a working `×` close button.
- No JS errors. Other dashboard features (header, sidebar filters, table, SSE live updates) still work.

Unblock and reload to confirm Vue path comes back.

- [ ] **Step 5: XSS sanity check**

Manually craft (via DevTools console) a fake record that exercises sanitization:

```javascript
window.__detailStore.record = {
  ...window.__detailStore.record,
  request: {
    ...window.__detailStore.record.request,
    body: {
      ...window.__detailStore.record.request.body,
      messages: [
        { role: 'user', content: 'Hello <script>alert(1)</script> <img src=x onerror="alert(2)">' },
        { role: 'assistant', content: '[link](javascript:alert(3))' },
      ],
    },
  },
};
```

Switch to the Request tab, structured mode. Expected: the `<script>` and `<img onerror>` are stripped by DOMPurify, the `javascript:` href is neutralized (link is either inert or removed). No alert popups. Reload to clear the injected state.

- [ ] **Step 6: Code-quality pass**

```bash
cargo fmt
cargo clippy --all-targets -- -W warnings
cargo test
```

Expected: all pass. None of these should produce new findings (the changes are all in static assets), but run them per the project's pre-commit conventions in CLAUDE.md before merging.

- [ ] **Step 7: Final commit (only if any cleanup edits were needed during verification)**

If verification surfaced fixes that need their own commit, commit them now with descriptive messages. Otherwise skip.

```bash
git status
# If clean, no commit needed — Task 10 was the last code commit.
# If dirty, commit per the issue, e.g.:
# git commit -m "fix(daemon-ui): <specific fix from verification>"
```

---

## Self-Review

Checked against the source spec § by §:

- **Layout decision (3 tabs + global Structured/Raw):** Task 4 builds the tab bar; the mode toggle lives in the panel header and is read by every tab component via `store.viewMode`. ✓
- **Render library choices (marked, DOMPurify, highlight.js):** Task 1 loads them via CDN; Task 5 wires them into `Markdown`, `JsonBlock`, and the `v-highlight` directive. ✓
- **Framework (Vue 3 global build mounted only on detail panel):** Task 1 loads `vue.global.prod.js`; Task 3 mounts on `#detail-mount`; the rest of the dashboard (header / sidebar / table / SSE stream) is untouched. ✓
- **CDN-only (no vendoring):** All four libraries are CDN `<script>`/`<link>`; no files added under `web-aggregate/`. ✓
- **`detailStore` reactive bridge:** Task 3 creates the store with exactly the shape specified (`visible / target / loading / error / record / viewMode / activeTab`). ✓
- **vanilla `selectRow` writes the store; Vue owns fetch:** Task 3 rewrites `selectRow`; Task 4 puts the fetch inside the root component's `watch(target)`. ✓
- **Tab/mode change does not re-fetch:** Task 4 splits fetch (watcher on `target`) from render (tabs and mode are local store writes). Verified in Task 4 Step 3. ✓
- **Component tree (DetailPanel / OverviewTab / RequestTab / ResponseTab / MessageThread / MessageItem / ContentBlock / SystemSection / ToolsSection / Markdown / JsonBlock):** Tasks 4, 5, 6, 7, 8, 9, 10 implement each. ✓ (`Collapsible` is realized via native `<details>` rather than a separate component — same outcome, less code.)
- **ContentBlock dispatch (text/tool_use/tool_result/thinking/image/unknown):** Task 7. Recursion through `tool_result.content` arrays. ✓
- **Image: text placeholder only, no remote loads:** Task 7 image branch. ✓
- **Markdown: sanitize then v-html; JsonBlock: text interpolation only:** Task 5. ✓
- **Shape detection / JSON fallback:** Task 9 (`isAnthropicShape`), Task 10 (content array / reassembled / rawSse fallback chain). ✓
- **Vue-missing fallback:** Task 3 Step 2 IIFE guards on `window.Vue`; `selectRow` checks for `window.__detailStore` and falls back to the legacy `<pre>` path; Task 3 Step 5 verifies. ✓
- **Server side untouched:** No Rust changes anywhere in the plan. ✓
- **XSS surface:** Task 5 (only `Markdown` uses `v-html`, and only after `DOMPurify`); Task 11 Step 5 explicitly verifies sanitization. ✓
- **Manual test plan from spec:** Task 11 Step 3 / 4 / 5 replay the spec checklist directly. ✓
- **YAGNI list (no vendoring, no JS test framework, no Codex shape, no image rendering):** None of these appear in the plan. ✓

Type/naming consistency: `record` (not `data` or `capture`) flows from `detailStore.record` through all tab components; `block` for ContentBlock prop; `record.request.body.messages` / `record.response.body_reassembled.content` paths are the same in Tasks 6, 9, 10. The store key names match Task 3's declaration in every later task. No drift found.

Placeholder scan: no "TODO", "TBD", "implement later", "similar to Task N", or vague "add validation". Every step that changes code shows the code; every step that runs a command shows the command and expected outcome.

Spec coverage: every section of the spec maps to a task or to an explicit "won't do" in the YAGNI list. No gaps.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-29-detail-structured-view.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
