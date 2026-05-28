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
  const panel = document.getElementById('detail-panel');
  const content = document.getElementById('detail-content');
  panel.classList.remove('hidden');

  try {
    const resp = await fetch(`/api/requests/${sid}/${seq}`);
    const data = await resp.json();
    renderDetail(data);
  } catch (e) {
    content.innerHTML = '<p>Failed to load request detail.</p>';
  }
}

function renderDetail(data) {
  const content = document.getElementById('detail-content');
  content.innerHTML = `
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

// Detail panel tabs
document.querySelectorAll('#detail-tabs .tab').forEach(tab => {
  tab.addEventListener('click', () => {
    document.querySelector('#detail-tabs .tab.active')?.classList.remove('active');
    tab.classList.add('active');
  });
});

// Close detail
document.getElementById('close-detail')?.addEventListener('click', () => {
  document.getElementById('detail-panel').classList.add('hidden');
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
