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
          <div v-if="store.activeTab === 'overview'" class="status-line">[overview placeholder — Task 6]</div>
          <div v-else-if="store.activeTab === 'request'" class="status-line">[request placeholder — Task 9]</div>
          <div v-else-if="store.activeTab === 'response'" class="status-line">[response placeholder — Task 10]</div>
        </div>
      </div>
    `,
  };

  const app = createApp(DetailPanel);
  app.component('Markdown', Markdown);
  app.component('JsonBlock', JsonBlock);
  app.directive('highlight', highlightDirective);
  app.mount('#detail-mount');

  watch(() => detailStore.visible, (v) => {
    const panel = document.getElementById('detail-panel');
    if (!panel) return;
    if (v) panel.classList.remove('hidden');
    else panel.classList.add('hidden');
  });
})();
