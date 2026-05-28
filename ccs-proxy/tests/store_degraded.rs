use ccs_proxy::store::FsStore;

#[test]
fn fresh_store_reports_zero_failures() {
    let dir = tempfile::tempdir().unwrap();
    let store = FsStore::open(dir.path().to_path_buf()).unwrap();
    assert_eq!(store.consecutive_write_failures(), 0);
}
