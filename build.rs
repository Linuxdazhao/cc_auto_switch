use std::path::Path;
use std::process::Command;

fn main() {
    // Feature off → complete no-op so publishers / docs.rs / downstream need no Node.
    if std::env::var_os("CARGO_FEATURE_WEB_UI").is_none() {
        return;
    }
    let web_dir = Path::new("web");
    if !web_dir.exists() {
        return; // source package without web/ (excluded) → skip
    }
    println!("cargo:rerun-if-changed=web/apps/aggregate/src");
    println!("cargo:rerun-if-changed=web/packages/ui/src");
    println!("cargo:rerun-if-changed=web/packages/api/src");

    let bun = if cfg!(windows) { "bun.exe" } else { "bun" };
    let status = Command::new(bun)
        .args(["run", "--filter", "@ccs/app-aggregate", "build"])
        .current_dir(web_dir)
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => panic!("vite build failed with status {s}"),
        Err(e) => panic!("failed to run bun (is bun installed?): {e}"),
    }
}
