# ccs-proxy

Local logging reverse-proxy + optional web dashboard for Claude Code and
Codex traffic. Pure Rust, single binary.

## Web dashboard (optional `web-ui` feature)

The web dashboard is gated behind the `web-ui` cargo feature, which is **off by
default**. Pure-Rust downstream consumers need **no Node/bun** — building or
depending on `ccs-proxy` without `web-ui` embeds no web assets and shells out to
no JS tooling.

When enabled, the dashboard is a Svelte 5 + Vite SPA built from the repo's
`web/` bun workspace (`web/apps/proxy`) and embedded into the binary at build
time. Building with the feature requires bun and a prior `bun install`
in `web/`:

    cargo build --release --features web-ui

The **published crate excludes** all web assets (`exclude = ["web/",
"tests/fixtures/"]`), and docs.rs builds with the feature off, so neither needs
Node.

## Quick start

    cargo install ccs-proxy
    ccs-proxy serve --provider claude --upstream https://api.anthropic.com

Then point your client at `ANTHROPIC_BASE_URL=http://127.0.0.1:<proxy_port>`
(printed at startup) and open the dashboard URL.

## Library use

ccs-proxy is also designed to be embedded — see the `serve()` function
returning a `ProxyHandle`. This is how
[cc-switch](https://github.com/Linuxdazhao/cc_auto_switch) integrates it
as a daemon.

## Design

See the design doc at
`cc_auto_switch/docs/superpowers/specs/2026-05-28-ccs-proxy-design.md`.
