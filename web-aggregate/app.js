'use strict';

const COLORS = ['#0066cc', '#e65100', '#2e7d32', '#6a1b9a', '#c62828', '#00838f'];
let upstreamColors = {};
let activeUpstreams = new Set();
let timeWindow = '1h';
let currentPage = 0;
const PAGE_SIZE = 50;
let allRequests = [];
let healthData = null;

async function init() {
  try {
    const resp = await fetch('/api/health');
    healthData = await resp.json();
    renderHeader();
    renderUpstreamFilters();
    await loadStats();
    await loadRequests();
    connectStream();
  } catch (e) {
    document.getElementById('meta').textContent = 'connection failed';
  }
}

function renderHeader() {
  const meta = document.getElementById('meta');
  const uptime = formatUptime(healthData.uptime_s);
  meta.textContent = `${healthData.proxy_count} proxies | ${healthData.total_requests} requests | uptime ${uptime}`;
}

function renderUpstreamFilters() {
  const container = document.getElementById('upstream-filters');
  container.innerHTML = '';
  (healthData.proxies || []).forEach((proxy, i) => {
    const color = COLORS[i % COLORS.length];
    upstreamColors[proxy.upstream] = color;
    activeUpstreams.add(proxy.upstream);

    const label = document.createElement('label');
    const cb = document.createElement('input');
    cb.type = 'checkbox';
    cb.checked = true;
    cb.addEventListener('change', () => {
      if (cb.checked) activeUpstreams.add(proxy.upstream);
      else activeUpstreams.delete(proxy.upstream);
      renderRequestTable();
    });

    const dot = document.createElement('span');
    dot.className = 'color-dot';
    dot.style.background = color;

    const aliases = proxy.aliases.length ? ` (${proxy.aliases.join(', ')})` : '';
    const shortUrl = proxy.upstream.replace(/^https?:\/\//, '').slice(0, 30);

    label.appendChild(cb);
    label.appendChild(dot);
    label.appendChild(document.createTextNode(` ${shortUrl}${aliases}`));
    container.appendChild(label);
  });
}

async function loadStats() {
  const since = timeWindowToISO(timeWindow);
  const url = since ? `/api/stats?since=${since}` : '/api/stats';
  try {
    const resp = await fetch(url);
    const data = await resp.json();
    renderStats(data);
  } catch (e) { /* ignore */ }
}

function renderStats(data) {
  const container = document.getElementById('stats-cards');
  const t = data.totals;
  container.innerHTML = `
    <div class="stat-card"><div class="label">Requests</div><div class="value">${t.total_requests}</div></div>
    <div class="stat-card"><div class="label">Input Tokens</div><div class="value">${formatNumber(t.total_input_tokens)}</div></div>
    <div class="stat-card"><div class="label">Output Tokens</div><div class="value">${formatNumber(t.total_output_tokens)}</div></div>
    <div class="stat-card"><div class="label">Avg Duration</div><div class="value">${t.avg_duration_ms}ms</div></div>
    <div class="stat-card"><div class="label">Errors</div><div class="value">${t.error_count}</div></div>
  `;
}

async function loadRequests() {
  try {
    const resp = await fetch('/api/sessions?limit=200');
    const sessions = await resp.json();
    allRequests = [];
    for (const session of sessions) {
      const rResp = await fetch(`/api/sessions/${session.session_id}`);
      const detail = await rResp.json();
      for (const req of (detail.requests || [])) {
        allRequests.push({
          ...req,
          upstream: session.upstream,
          aliases: session.aliases,
        });
      }
    }
    allRequests.sort((a, b) => new Date(b.started_at) - new Date(a.started_at));
    renderRequestTable();
  } catch (e) { /* ignore */ }
}

function renderRequestTable() {
  const tbody = document.getElementById('request-list');
  const since = timeWindowToDate(timeWindow);
  const filtered = allRequests.filter(r => {
    if (!activeUpstreams.has(r.upstream)) return false;
    if (since && new Date(r.started_at) < since) return false;
    return true;
  });

  const start = currentPage * PAGE_SIZE;
  const page = filtered.slice(start, start + PAGE_SIZE);

  tbody.innerHTML = '';
  for (const req of page) {
    const tr = document.createElement('tr');
    const color = upstreamColors[req.upstream] || '#999';
    tr.style.borderLeftColor = color;
    tr.className = req.has_error ? 'status-error' : 'status-ok';

    const tokens = (req.input_tokens || 0) + (req.output_tokens || 0);
    const status = req.status || '—';
    const duration = req.duration_ms ? `${req.duration_ms}ms` : '—';
    const time = new Date(req.started_at).toLocaleTimeString();
    const shortUpstream = req.upstream.replace(/^https?:\/\//, '').slice(0, 25);

    tr.innerHTML = `
      <td>${time}</td>
      <td>${shortUpstream}</td>
      <td>${req.model || '—'}</td>
      <td>${tokens || '—'}</td>
      <td>${status}</td>
      <td>${duration}</td>
    `;
    tr.addEventListener('click', () => selectRow(req.session_id, req.seq));
    tbody.appendChild(tr);
  }

  renderPagination(filtered.length);
}

function renderPagination(total) {
  const container = document.getElementById('pagination');
  const pages = Math.ceil(total / PAGE_SIZE);
  if (pages <= 1) { container.innerHTML = ''; return; }

  container.innerHTML = `
    <button ${currentPage === 0 ? 'disabled' : ''} id="prev-page">&laquo; Prev</button>
    <span>${currentPage + 1} / ${pages}</span>
    <button ${currentPage >= pages - 1 ? 'disabled' : ''} id="next-page">Next &raquo;</button>
  `;
  document.getElementById('prev-page')?.addEventListener('click', () => { currentPage--; renderRequestTable(); });
  document.getElementById('next-page')?.addEventListener('click', () => { currentPage++; renderRequestTable(); });
}

async function selectRow(sid, seq) {
  if (window.__detailStore) {
    window.__detailStore.target = { sid, seq };
    window.__detailStore.visible = true;
    return;
  }
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

function connectStream() {
  const evtSource = new EventSource('/api/stream');

  evtSource.addEventListener('request_started', (e) => {
    const data = JSON.parse(e.data);
    prependLiveRow(data, 'started');
  });

  evtSource.addEventListener('request_completed', (e) => {
    const data = JSON.parse(e.data);
    prependLiveRow(data, 'completed');
    refreshHeaderCount();
  });

  evtSource.onerror = () => {
    setTimeout(() => connectStream(), 3000);
    evtSource.close();
  };
}

function prependLiveRow(data, type) {
  const tbody = document.getElementById('request-list');
  if (!activeUpstreams.has(data.upstream)) return;

  const tr = document.createElement('tr');
  const color = upstreamColors[data.upstream] || '#999';
  tr.style.borderLeftColor = color;
  tr.className = 'new-row';

  if (type === 'completed') {
    tr.className += data.has_error ? ' status-error' : ' status-ok';
    const tokens = (data.usage?.input_tokens || 0) + (data.usage?.output_tokens || 0);
    tr.innerHTML = `
      <td>${new Date().toLocaleTimeString()}</td>
      <td>${data.upstream.replace(/^https?:\/\//, '').slice(0, 25)}</td>
      <td>—</td>
      <td>${tokens || '—'}</td>
      <td>${data.status}</td>
      <td>${data.duration_ms}ms</td>
    `;
  } else {
    tr.innerHTML = `
      <td>${new Date().toLocaleTimeString()}</td>
      <td>${data.upstream.replace(/^https?:\/\//, '').slice(0, 25)}</td>
      <td>${data.model || '—'}</td>
      <td>—</td>
      <td>...</td>
      <td>—</td>
    `;
  }

  tr.addEventListener('click', () => selectRow(data.session_id, data.seq));
  tbody.prepend(tr);

  while (tbody.children.length > PAGE_SIZE) {
    tbody.removeChild(tbody.lastChild);
  }
}

function refreshHeaderCount() {
  if (!healthData) return;
  healthData.total_requests++;
  renderHeader();
}

// Time filter buttons
document.querySelectorAll('.time-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    document.querySelector('.time-btn.active')?.classList.remove('active');
    btn.classList.add('active');
    timeWindow = btn.dataset.window;
    currentPage = 0;
    loadStats();
    renderRequestTable();
  });
});

// Helpers
function timeWindowToISO(w) {
  const d = timeWindowToDate(w);
  return d ? d.toISOString() : null;
}

function timeWindowToDate(w) {
  if (w === 'all') return null;
  const now = new Date();
  switch (w) {
    case '1h': return new Date(now - 3600000);
    case '24h': return new Date(now - 86400000);
    case '7d': return new Date(now - 604800000);
    default: return null;
  }
}

function formatUptime(seconds) {
  if (seconds < 60) return `${seconds}s`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
  return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
}

function formatNumber(n) {
  if (!n) return '0';
  if (n >= 1000000) return (n / 1000000).toFixed(1) + 'M';
  if (n >= 1000) return (n / 1000).toFixed(1) + 'k';
  return n.toString();
}

init();

// ===== Vue detail panel =====
(function mountDetailPanel() {
  if (!window.Vue) {
    console.warn('[ccs-daemon] Vue not loaded; detail panel falls back to JSON-only renderer.');
    return;
  }
  const { createApp, reactive, computed, watch, ref, h } = window.Vue;

  const detailStore = reactive({
    visible: false,
    target: null,
    loading: false,
    error: null,
    record: null,
    viewMode: 'structured',
    activeTab: 'overview',
  });
  window.__detailStore = detailStore;

  // ===== v-highlight directive =====
  const highlightDirective = {
    mounted(el) { highlightAll(el); },
    updated(el) { highlightAll(el); },
  };
  function highlightAll(el) {
    if (!window.hljs) return;
    el.querySelectorAll('pre code').forEach((node) => {
      if (node.dataset.highlighted === 'yes') return;
      try { window.hljs.highlightElement(node); } catch (_) { /* ignore */ }
    });
  }

  // ===== Markdown component =====
  const Markdown = {
    props: { text: { type: String, default: '' } },
    directives: { highlight: highlightDirective },
    computed: {
      html() {
        if (!window.marked || !window.DOMPurify) {
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

  // ===== JsonBlock component =====
  const JsonBlock = {
    props: { value: { required: true } },
    directives: { highlight: highlightDirective },
    computed: {
      text() {
        try { return JSON.stringify(this.value, null, 2); }
        catch (_) { return String(this.value); }
      },
    },
    template: `<pre class="json-block" v-highlight><code class="language-json">{{ text }}</code></pre>`,
  };

  // ===== ContentBlock =====
  const ContentBlock = {
    name: 'ContentBlock',
    props: { block: { required: true } },
    setup() { return { store: detailStore }; },
    template: `
      <div class="block" :class="blockClass">
        <!-- text -->
        <Markdown v-if="block.type === 'text'" :text="block.text || ''" />

        <!-- tool_use -->
        <details v-else-if="block.type === 'tool_use'" class="block tool-use" open>
          <summary>
            <span class="tool-name">{{ block.name }}</span>
            <span v-if="block.id" style="color: var(--muted); font-weight: normal;"> · {{ block.id }}</span>
          </summary>
          <div class="body">
            <JsonBlock :value="block.input ?? {}" />
          </div>
        </details>

        <!-- tool_result -->
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

        <!-- image -->
        <div v-else-if="block.type === 'image'" class="block image-placeholder">
          [image: {{ block.source?.media_type || 'unknown' }}]
        </div>

        <!-- unknown -->
        <JsonBlock v-else :value="block" />
      </div>
    `,
    computed: {
      blockClass() {
        return this.block && this.block.type ? 'type-' + this.block.type : 'type-unknown';
      },
    },
  };

  // ===== MessageItem =====
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

  // ===== MessageThread =====
  const MessageThread = {
    props: { messages: { type: Array, required: true } },
    template: `
      <div class="message-thread">
        <MessageItem v-for="(m, i) in messages" :key="i" :message="m" />
      </div>
    `,
  };

  // ===== SystemSection =====
  const SystemSection = {
    props: { system: { required: false, default: null } },
    computed: {
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

  // ===== ToolsSection =====
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

  // ===== OverviewTab =====
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

  // ===== RequestTab =====
  const RequestTab = {
    props: { record: { required: true } },
    setup() { return { store: detailStore }; },
    computed: {
      body() { return this.record?.request?.body ?? null; },
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

  // ===== ResponseTab =====
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

  // ===== DetailPanel =====
  const DetailPanel = {
    setup() {
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
          <OverviewTab v-if="store.activeTab === 'overview'" :record="store.record" />
          <RequestTab v-else-if="store.activeTab === 'request'" :record="store.record" />
          <ResponseTab v-else-if="store.activeTab === 'response'" :record="store.record" />
        </div>
      </div>
    `,
  };

  const app = createApp(DetailPanel);
  app.component('Markdown', Markdown);
  app.component('JsonBlock', JsonBlock);
  app.component('ContentBlock', ContentBlock);
  app.component('MessageItem', MessageItem);
  app.component('MessageThread', MessageThread);
  app.component('SystemSection', SystemSection);
  app.component('ToolsSection', ToolsSection);
  app.component('OverviewTab', OverviewTab);
  app.component('RequestTab', RequestTab);
  app.component('ResponseTab', ResponseTab);
  app.directive('highlight', highlightDirective);
  app.mount('#detail-mount');

  watch(() => detailStore.visible, (v) => {
    const panel = document.getElementById('detail-panel');
    if (!panel) return;
    if (v) panel.classList.remove('hidden');
    else panel.classList.add('hidden');
  });
})();
