# ccs-proxy

Local logging reverse-proxy + minimal web dashboard for Claude Code and
Codex traffic. Pure Rust, single binary.

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
