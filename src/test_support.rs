//! All Rust fixtures live inside this checkout, including direct `cargo test` runs.
use std::path::PathBuf;

pub fn temp_base() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp/tests");
    std::fs::create_dir_all(&path).expect("create project-local test directory");
    path
}

/// New tests use a guard so success, early returns and unwinding all clean up.
pub struct TestDir(PathBuf);
impl TestDir {
    pub fn new(prefix: &str) -> Self {
        assert!(prefix
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-'));
        let path = temp_base().join(format!("{prefix}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&path).expect("create test fixture");
        Self(path)
    }
}
impl std::ops::Deref for TestDir {
    type Target = std::path::Path;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
