use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(
    name = "ccs-proxy",
    version,
    about = "Local logging reverse-proxy + dashboard for Claude Code / Codex traffic"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Start the proxy + dashboard.
    Serve(ServeArgs),
}

#[derive(Args)]
struct ServeArgs {
    /// Port for the reverse-proxy listener (0 = OS-assigned).
    #[arg(long, default_value_t = 0)]
    proxy_port: u16,

    /// Port for the dashboard / API listener (0 = OS-assigned).
    #[arg(long, default_value_t = 0)]
    api_port: u16,

    /// Upstream provider kind: `claude` or `codex`.
    #[arg(long, value_parser = parse_provider)]
    provider: ccs_proxy::ProviderKind,

    /// Upstream base URL (e.g. https://api.anthropic.com).
    #[arg(long)]
    upstream: url::Url,

    /// Directory for persistent capture data (default: ~/.ccs-proxy).
    #[arg(long)]
    data_dir: Option<PathBuf>,

    /// Disable header / body redaction (dangerous; tokens land on disk).
    #[arg(long, default_value_t = false)]
    no_redact: bool,

    /// Do not auto-open the dashboard URL in the default browser.
    #[arg(long, default_value_t = false)]
    no_open: bool,

    /// Allow CORS from this origin (dev-only; do not enable on shared hosts).
    #[arg(long)]
    cors_allow: Option<String>,
}

fn parse_provider(value: &str) -> Result<ccs_proxy::ProviderKind, String> {
    value
        .parse::<ccs_proxy::ProviderKind>()
        .map_err(|err| err.to_string())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("ccs_proxy=info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Serve(args) => run_serve(args).await,
    }
}

async fn run_serve(args: ServeArgs) -> Result<()> {
    let data_dir = args
        .data_dir
        .or_else(|| dirs::home_dir().map(|home| home.join(".ccs-proxy")))
        .context("could not determine data dir (set --data-dir or $HOME)")?;

    let mut cfg = ccs_proxy::ServeConfig::new(args.provider, args.upstream, data_dir);
    cfg.proxy_port = args.proxy_port;
    cfg.api_port = args.api_port;
    cfg.redact = !args.no_redact;
    cfg.cors_allow = args.cors_allow;

    if cfg.cors_allow.is_some() {
        tracing::warn!("--cors-allow is dev-only; do not enable on shared machines");
    }
    if !cfg.redact {
        tracing::warn!("--no-redact disables redaction; secrets will be persisted to disk");
    }

    let handle = ccs_proxy::serve(cfg).await?;
    let api_port = handle
        .api_port
        .expect("api_server=true guarantees api_port");
    let dashboard_url = format!("http://127.0.0.1:{api_port}/");
    println!("ccs-proxy -> {}", handle.upstream);
    println!("  proxy:     http://127.0.0.1:{}", handle.proxy_port);
    println!("  dashboard: {dashboard_url}");

    if !args.no_open
        && let Err(err) = webbrowser::open(&dashboard_url)
    {
        tracing::info!(?err, "dashboard auto-open failed (continuing)");
    }

    tokio::signal::ctrl_c().await.ok();
    println!("\nshutting down...");
    handle.shutdown().await;
    Ok(())
}
