const requestsEl = document.getElementById('requests');
const detailEl = document.getElementById('detail');
const detailEmptyEl = document.getElementById('detail-empty');
const metaEl = document.getElementById('meta');
const tabButtons = document.querySelectorAll('.tabs button');
const tabPanes = {
  overview: document.getElementById('tab-overview'),
  request: document.getElementById('tab-request'),
  response: document.getElementById('tab-response'),
  headers: document.getElementById('tab-headers'),
  usage: document.getElementById('tab-usage'),
};
let currentSessionId = null;
let selectedSeq = null;

async function init() {
  const health = await fetch('/api/health').then(r => r.json());
  currentSessionId = health.session_id;
  metaEl.textContent = `${health.provider}  ${health.upstream}  session=${health.session_id}`;
  await loadList();
  connectStream();
}

async function loadList() {
  if (!currentSessionId) return;
  const data = await fetch(`/api/sessions/${currentSessionId}`).then(r => r.json());
  requestsEl.innerHTML = '';
  for (const r of data.requests) renderRow(r);
}

function renderRow(r) {
  const li = document.createElement('li');
  li.dataset.seq = r.seq;
  if (r.has_error || (r.status && r.status >= 400)) li.classList.add('error');
  li.innerHTML = `
    <div class="row1">
      <span>${new Date(r.started_at).toLocaleTimeString()}</span>
      <span class="status">${r.status ?? '...'}</span>
    </div>
    <div>${r.model ?? ''}  ${r.duration_ms ?? ''}ms  in:${r.input_tokens ?? '-'} out:${r.output_tokens ?? '-'}</div>
    <div class="request-id">${r.request_id ?? ''}</div>
  `;
  li.addEventListener('click', () => selectRow(r.seq));
  // Insert in seq order:
  const existing = requestsEl.querySelector(`li[data-seq="${r.seq}"]`);
  if (existing) existing.replaceWith(li);
  else requestsEl.appendChild(li);
}

async function selectRow(seq) {
  selectedSeq = seq;
  for (const li of requestsEl.querySelectorAll('li')) {
    li.classList.toggle('selected', String(li.dataset.seq) === String(seq));
  }
  const rec = await fetch(`/api/requests/${currentSessionId}/${seq}`).then(r => r.json());
  detailEmptyEl.hidden = true;
  detailEl.hidden = false;
  renderDetail(rec);
}

function renderDetail(rec) {
  tabPanes.overview.innerHTML = `
    <dl class="kv">
      <dt>request-id</dt><dd>${rec.request_id ?? ''} <button class="copy" data-copy="${rec.request_id ?? ''}">copy</button></dd>
      <dt>status</dt><dd>${rec.response?.status ?? ''}</dd>
      <dt>model</dt><dd>${rec.model ?? ''}</dd>
      <dt>duration</dt><dd>${rec.duration_ms ?? ''}ms</dd>
      <dt>ttft</dt><dd>${rec.ttft_ms ?? ''}ms</dd>
      <dt>started</dt><dd>${rec.started_at}</dd>
      <dt>tokens</dt><dd>in:${rec.usage?.input_tokens ?? '-'} out:${rec.usage?.output_tokens ?? '-'}</dd>
      <dt>error</dt><dd>${rec.error ? `${rec.error.kind}: ${rec.error.message}` : ''}</dd>
    </dl>
  `;
  tabPanes.request.innerHTML = `<pre>${escapeHtml(JSON.stringify(rec.request.body, null, 2))}</pre>`;
  tabPanes.response.innerHTML = `<pre>${escapeHtml(JSON.stringify(rec.response?.body_reassembled ?? null, null, 2))}</pre>`;
  tabPanes.headers.innerHTML = `
    <h3>Request</h3><pre>${escapeHtml(JSON.stringify(rec.request.headers, null, 2))}</pre>
    <h3>Response</h3><pre>${escapeHtml(JSON.stringify(rec.response?.headers ?? {}, null, 2))}</pre>
  `;
  tabPanes.usage.innerHTML = `<pre>${escapeHtml(JSON.stringify(rec.usage ?? {}, null, 2))}</pre>`;
  for (const btn of tabPanes.overview.querySelectorAll('button.copy')) {
    btn.addEventListener('click', e => navigator.clipboard.writeText(e.target.dataset.copy));
  }
}

for (const btn of tabButtons) {
  btn.addEventListener('click', () => {
    for (const b of tabButtons) b.classList.toggle('active', b === btn);
    for (const [name, el] of Object.entries(tabPanes)) el.hidden = name !== btn.dataset.tab;
  });
}

function connectStream() {
  const es = new EventSource('/api/stream');
  es.addEventListener('request_started', e => {
    const d = JSON.parse(e.data);
    renderRow({ seq: d.seq, session_id: d.session_id, started_at: d.started_at, model: d.model, request_id: null, status: null });
  });
  es.addEventListener('request_completed', e => {
    const d = JSON.parse(e.data);
    renderRow({ seq: d.seq, session_id: d.session_id, started_at: new Date().toISOString(), model: null, request_id: d.request_id, status: d.status, duration_ms: d.duration_ms, input_tokens: d.usage?.input_tokens, output_tokens: d.usage?.output_tokens, has_error: d.has_error });
    if (selectedSeq === d.seq) selectRow(d.seq);
  });
}

function escapeHtml(s) {
  return s.replace(/[&<>"']/g, ch => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'})[ch]);
}

init().catch(err => { metaEl.textContent = 'init error: ' + err.message; });
