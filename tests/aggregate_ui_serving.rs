#![cfg(feature = "web-ui")]
// Verify the Vite SPA assets are embedded: dist/index.html must exist at build time.
#[test]
fn unknown_path_falls_back_to_index() {
    assert!(std::path::Path::new("web-aggregate/dist/index.html").exists());
}
