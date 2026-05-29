# Aggregate dashboard: filters & empty-session fix

**Date:** 2026-05-29
**Status:** Approved (pending implementation plan)
**Scope:** `ccs-proxy` store, `src/daemon/aggregate/*`, `web-aggregate/*`

## Problem

Three issues with the aggregate dashboard (`cc-switch daemon`'s web UI at
`http://127.0.0.1:<port>/`):

1. **No model filter.** The sidebar lets users filter by Upstream and Time
   window, but not by model. With multiple proxies routing to different
   model providers, users want to narrow the view to a specific model
   family (e.g. only Opus traffic).
2. **Clicking a session shows an empty table.** Confirmed root cause:
   every Claude Code startup calls `init_session` on **all** configured
   proxies, but only the proxy actually receiving traffic ever calls
   `append`. The other proxies end up with `meta.json` files whose
   `request_count` is `0` and no record files. On the user's machine,
   45 of 60 visible sessions are these empty ghosts. Clicking one drills
   into a request table with zero rows, which the user reads as
   "click does nothing."
3. **No working-directory filter.** Users run Claude Code in many
   projects. They want to see "all my conversations in repo X" without
   eyeballing session IDs and timestamps.

## Decisions (already aligned with user)

- **Empty sessions:** hide by default; sidebar checkbox to show them.
  Do *not* change `init_session` to defer creation — too risky.
- **cwd storage:** on `SessionMeta` (not per `CaptureRecord`).
  Extracted from the first request whose system prompt contains
  `Primary working directory: <path>` and then frozen.
- **Filter scope:** model and cwd filters apply to **both** the Requests
  view and the Sessions view.
- **Model filter UX:** sidebar multi-select checkboxes, dynamically
  generated like the existing Upstream filter.

Out of scope: Codex system-prompt cwd extraction, merging "ghost
sessions" across proxies into a single logical session, deleting empty
sessions from disk, surfacing cwd in the request detail panel.

## Architecture

### Data model

`ccs-proxy/src/store/mod.rs` — extend `SessionMeta`:

```rust
pub struct SessionMeta {
    // ...existing fields
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub models: Vec<String>,   // deduped, insertion-ordered
}
```

`#[serde(default)]` keeps old `meta.json` files (without these fields)
readable. They get backfilled on the next `append`.

### Extraction

`ccs-proxy/src/capture/extract.rs` — new pure function:

```rust
/// Extract the Claude Code cwd marker from a request body's `system`
/// field. Returns the path string if a line matches
/// `^Primary working directory:\s+(\S.*?)$`. Handles both
/// `system: "..."` (string) and `system: [{"type":"text","text":"..."}]`
/// (block list) shapes.
pub fn extract_cwd(body: &serde_json::Value) -> Option<String>;
```

Anchored to start-of-line (`(?m)^`) to avoid matching English prose like
"...the user's working directory...". Only the first match is taken.

### Persistence

`ccs-proxy/src/store/fs.rs` — fold the existing `bump_request_count`
into a wider `bump_meta(rec)` step inside `append`:

```rust
async fn bump_meta(&self, rec: &CaptureRecord) {
    // 1. read meta.json
    // 2. meta.request_count = max(meta.request_count, rec.seq)
    // 3. if meta.cwd.is_none() { meta.cwd = extract_cwd(&rec.request.body); }
    // 4. if let Some(m) = &rec.model {
    //        if !meta.models.contains(m) { meta.models.push(m.clone()); }
    //    }
    // 5. atomic_write
}
```

One `read + atomic_write` per request, same I/O cost as today (we
already do it for the request_count bump).

### API

`src/daemon/aggregate/routes.rs::list_sessions` — extend
`SessionsQuery`:

```rust
#[derive(Deserialize, Default)]
struct SessionsQuery {
    upstream: Option<String>,
    alias: Option<String>,
    model: Option<String>,           // comma-separated, OR within field
    cwd: Option<String>,             // comma-separated, OR within field
    include_empty: Option<bool>,     // default: false
    limit: Option<usize>,
    offset: Option<usize>,
}
```

Filtering semantics:
- All query parameters combine with AND.
- Within `model` and `cwd`, multiple values OR.
- A session matches a `model` value if it appears in `meta.models`.
- A session matches a `cwd` value if `meta.cwd == Some(value)`, OR the
  value is the literal string `"(unknown)"` AND `meta.cwd.is_none()`.
- When `include_empty` is unset or false, sessions with
  `request_count == 0` are dropped.

Response shape — each session gains `cwd` (nullable) and `models`
(array, possibly empty).

`src/daemon/aggregate/routes.rs` — new endpoint:

```text
GET /api/meta
→ {
    "models": ["claude-opus-4-7", ...],   // global dedupe, sorted
    "cwds":   ["/Users/...", ...]         // global dedupe, sorted
  }
```

Implementation: iterate `state.stores`, call `list_sessions()` on each,
union the `models` and `cwd` fields. O(N) over all `meta.json` files.
No record-file I/O.

### Frontend

`web-aggregate/index.html` — sidebar additions between Upstreams and
Time:

```html
<h3>Models</h3>
<div id="model-filters"></div>
<h3>Working dirs</h3>
<div id="cwd-filters"></div>
```

And under Time (visible only in Sessions view):

```html
<label id="show-empty-wrap">
  <input type="checkbox" id="show-empty-sessions">
  Show empty sessions
</label>
```

`web-aggregate/app.js` — new global state:

```js
let activeModels = new Set();   // empty == all selected
let activeCwds = new Set();
let availableModels = [];
let availableCwds = [];
let showEmptySessions = false;
```

`init()` fetches `/api/meta` and renders the Model/Cwd filter blocks.
Cwd labels use `basename(path)` for the primary text with the full
path shown in lighter gray to the right (and as a `title` tooltip).
Sessions whose `cwd` is null are bucketed into a virtual `"(unknown)"`
entry, default-checked.

`loadRequests()` / `loadSessions()` build the query string from
`activeUpstreams`, `activeModels`, `activeCwds`, and (for sessions)
`include_empty=showEmptySessions`. The redundant client-side
upstream filter loop in `renderSessionTable` is removed — server-side
filtering is authoritative for sessions. The time-window filter stays
client-side (cheap, no extra round-trip).

**Caveat for the Requests view:** server-side model/cwd filtering
matches at the **session** level (a session is returned if any of its
requests used the model). The Requests view then loads every request
in those matching sessions, which may include requests on *other*
models within the same session. To honor the user's "show only model
X" intent, `renderRequestTable` keeps a final per-request filter pass
that drops rows whose `model` isn't in `activeModels` (when
`activeModels` is non-empty). The cwd filter does not need a
per-request pass since cwd is session-scoped.

`connectStream()` — when a `request_started` event arrives, if its
`model` is new, push it into `availableModels`, re-render the model
filter (default-checked), then run the existing
`prependLiveRow(data, 'started')` after applying model/cwd guards
matching the active filters.

### Visual layout (sidebar)

```text
View   [Requests][Sessions]
Upstreams
  ☑ ● api.deepseek.com (deepseek)
  ☑ ● api.anthropic.com (official)
Models
  ☑ claude-opus-4-7
  ☑ claude-sonnet-4-6
  ☐ claude-haiku-4-5
Working dirs
  ☑ aie-server      /Users/jingzhao/Work/kry/aie-server
  ☐ cc_auto_switch  /Users/jingzhao/Work/github/cc_auto_switch
  ☑ (unknown)
Time   [1h][24h][7d][all]
  ☐ Show empty sessions       (Sessions view only)
```

## Data flow

1. Claude Code sends a request → ccs-proxy captures it.
2. `FsStore::append` writes `0001.json`, then `bump_meta`:
   - Bumps `request_count`.
   - If `meta.cwd` empty, extracts from `Primary working directory:` line.
   - If `rec.model` new, appends to `meta.models`.
   - `atomic_write` the new meta.
3. User opens dashboard → `init()`:
   - GET `/api/health` (existing).
   - GET `/api/meta` (new) → populate Model / Cwd filter sidebar.
   - GET `/api/stats` + `/api/sessions?limit=200` (existing pattern).
4. User toggles a checkbox → re-fetch with new query string → re-render.
5. SSE `request_started` arrives → if its `model` or session's `cwd`
   is new, update the sidebar; if matches active filters, prepend a row.

## Error handling

- Malformed system prompts: `extract_cwd` returns `None`; meta stays
  unchanged. Logged at `tracing::debug` level only — not a runtime
  problem.
- Old `meta.json` without new fields: `#[serde(default)]` yields
  `None` / `Vec::new()`. First subsequent `append` backfills.
- `/api/meta` failure: frontend logs and shows the sidebar without
  Model/Cwd sections (filters effectively "all"). User can still use
  Upstream + Time filters.
- Invalid `model` / `cwd` query value: treated as a value with zero
  matches → empty result, not an error.

## Testing

`ccs-proxy/src/capture/extract.rs` — unit tests on `extract_cwd`:

- System as string with the marker on its own line → matches.
- System as `Vec<Block>` where one block contains the marker → matches.
- System with the marker mid-sentence (`...the working directory...`)
  → does not match.
- No system field at all → `None`.

`ccs-proxy/tests/store_fs.rs` — integration tests:

- `append` extracts cwd and writes it back to `meta.json`; second
  `append` doesn't overwrite an existing cwd.
- `append` accumulates and deduplicates `models`.
- An old `meta.json` (manually written without `cwd`/`models`) can be
  re-read by `list_sessions` and shows `cwd: None`, `models: []`.

`tests/daemon_aggregate.rs` — HTTP-level tests:

- `GET /api/sessions` defaults to `include_empty=false` and drops
  zero-request sessions.
- `GET /api/sessions?include_empty=true` includes them.
- `GET /api/sessions?model=foo,bar` keeps only sessions whose
  `meta.models` intersects `{foo, bar}`.
- `GET /api/sessions?cwd=/a,/b` keeps only matching sessions; the
  special value `(unknown)` matches `cwd: None`.
- Combined filters AND together.
- `GET /api/meta` returns deduped, sorted lists.

Frontend behavior is verified manually after backend tests pass:
start the daemon with real data, open the dashboard, toggle the new
filters, click previously-empty sessions (should now be hidden by
default; visible and labeled when "Show empty sessions" is ticked).

## Migration

- No on-disk migration needed. `#[serde(default)]` handles existing
  `meta.json` files; fields backfill organically on the next request.
- The 45 existing empty sessions stay on disk but are hidden by the
  default `include_empty=false`. No automatic pruning in this change.
  A separate `cc-switch daemon prune-empty` subcommand can be added
  later if disk usage becomes a concern.

## Risks

1. **cwd regex false negatives** if Claude Code changes its system
   prompt wording. Mitigation: affected sessions fall into the
   `"(unknown)"` cwd bucket; the rest of the dashboard keeps working.
   We can extend `extract_cwd` to try additional patterns later.
2. **Codex requests have a different system-prompt shape** and won't
   yield a cwd. They bucket into `"(unknown)"`. Acceptable for v1;
   adding Codex extraction is a follow-up.
3. **Performance of `/api/meta`** scales O(sessions). At ~60 sessions
   today and unlikely growth beyond a few thousand per machine, this
   is sub-10ms. No caching needed in v1.
4. **Sidebar overflow** when many cwds exist. Tolerable until a user
   hits 10+ cwds; revisit with a scroll container then.

## Non-goals (explicit)

- Merging cross-proxy "ghost sessions" into one logical session.
- Deleting empty sessions from disk.
- Codex system-prompt cwd extraction.
- Exposing cwd in the request detail panel.
- Server-side time-window filtering (kept client-side, cheap and
  already works).
