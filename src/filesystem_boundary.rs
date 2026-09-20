//! Path checks for operator-owned file trees. These checks do not serialize local
//! filesystem mutations; they are not a sandbox against a concurrent local writer.
use std::io;
use std::path::{Path, PathBuf};

/// Resolve an existing path, or its nearest existing ancestor plus missing suffix.
/// symlink_metadata is essential: `exists()` silently skips dangling symlinks.
fn projected_path(path: &Path) -> io::Result<PathBuf> {
    let mut cursor = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut missing = Vec::new();
    loop {
        match std::fs::symlink_metadata(&cursor) {
            Ok(_) => {
                let mut canonical = cursor.canonicalize()?;
                for component in missing.iter().rev() {
                    canonical.push(component);
                }
                return Ok(canonical);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let name = cursor.file_name().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "invalid missing path ancestor",
                    )
                })?;
                missing.push(name.to_os_string());
                if !cursor.pop() {
                    return Err(error);
                }
            }
            Err(error) => return Err(error),
        }
    }
}

/// The returned canonical projection is for containment/alias comparisons only.
/// Rename/delete must operate on the original path to preserve symlink semantics.
pub fn confined_path(root: &Path, target: &Path) -> io::Result<PathBuf> {
    let root = projected_path(root)?;
    let target = projected_path(target)?;
    if !target.starts_with(root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "path escapes configured root",
        ));
    }
    Ok(target)
}

pub async fn confined_path_async(root: &Path, target: &Path) -> io::Result<PathBuf> {
    let root = root.to_path_buf();
    let target = target.to_path_buf();
    tokio::task::spawn_blocking(move || confined_path(&root, &target))
        .await
        .map_err(io::Error::other)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TestDir;

    #[test]
    fn boundary_supports_missing_roots_and_relative_config_paths() {
        let fixture = TestDir::new("filesystem-boundary");
        let root = fixture.join("not-created");
        assert!(confined_path(&root, &root.join("nested/file")).is_ok());
        assert!(confined_path(&root, &fixture.join("sibling")).is_err());
        let cwd = std::env::current_dir().unwrap();
        let relative = root.strip_prefix(&cwd).unwrap();
        assert!(confined_path(relative, &relative.join("new/file")).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn boundary_resolves_internal_links_without_skipping_dangling_links() {
        use std::os::unix::fs::symlink;
        let fixture = TestDir::new("filesystem-links");
        let root = fixture.join("served");
        std::fs::create_dir_all(root.join("nested")).unwrap();
        symlink(root.join("nested"), root.join("safe")).unwrap();
        symlink(fixture.join("outside-missing"), root.join("broken")).unwrap();
        symlink(&root, fixture.join("root-alias")).unwrap();
        assert_eq!(
            confined_path(&root, &root.join("safe/new/file")).unwrap(),
            root.canonicalize().unwrap().join("nested/new/file")
        );
        assert!(confined_path(
            &fixture.join("root-alias"),
            &fixture.join("root-alias/safe/new")
        )
        .is_ok());
        assert!(confined_path(&root, &root.join("broken")).is_err());
        assert!(confined_path(&root, &root.join("broken/new/file")).is_err());
    }
}
