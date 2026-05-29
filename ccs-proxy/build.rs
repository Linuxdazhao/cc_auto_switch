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
    let bun = if cfg!(windows) { "bun.exe" } else { "bun" };
    let status = Command::new(bun)
        .args(["run", "--filter", "@ccs/app-proxy", "build"])
        .current_dir(web_dir)
        .status()
        .expect("failed to run bun");
    assert!(status.success(), "vite build failed");
}
