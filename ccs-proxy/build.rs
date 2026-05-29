use std::path::Path;
use std::process::Command;

fn main() {
    if std::env::var_os("CARGO_FEATURE_WEB_UI").is_none() {
        return; // default + downstream consumers: zero Node
    }
    let web_dir = Path::new("../web");
    if !web_dir.exists() {
        return;
    }
    println!("cargo:rerun-if-changed=../web/apps/proxy/src");
    println!("cargo:rerun-if-changed=../web/packages/ui/src");
    let pnpm = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };
    let status = Command::new(pnpm)
        .args(["--filter", "@ccs/app-proxy", "build"])
        .current_dir(web_dir)
        .status()
        .expect("failed to run pnpm");
    assert!(status.success(), "vite build failed");
}
