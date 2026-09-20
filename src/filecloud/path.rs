use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};

pub fn normalize_prefix(prefix: &str) -> String {
    let trimmed = prefix.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

pub fn path_matches(prefix: &str, path: &str) -> bool {
    let prefix = normalize_prefix(prefix);
    if prefix == "/" {
        return path.starts_with('/');
    }
    path == prefix || path.starts_with(&format!("{prefix}/"))
}

pub fn relative_route(prefix: &str, request_path: &str) -> Option<String> {
    let prefix = normalize_prefix(prefix);
    if !path_matches(&prefix, request_path) {
        return None;
    }
    let relative = request_path
        .strip_prefix(&prefix)
        .unwrap_or("")
        .trim_start_matches('/')
        .to_string();
    Some(relative)
}

pub fn resolve_relative_path(root: &Path, relative: &str) -> Result<PathBuf> {
    let mut target = root.to_path_buf();

    // API query parameters have already been decoded by serde_urlencoded;
    // JSON paths and directory names are literal filesystem names.
    for part in relative.trim_start_matches('/').split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if !crate::static_security::allowed_component(part, false) {
            return Err(anyhow!("filecloud path escapes root"));
        }
        target.push(part);
    }

    Ok(target)
}

pub fn ensure_within_root(root: &Path, target: &Path) -> Result<()> {
    crate::filesystem_boundary::confined_path(root, target)
        .map_err(|error| anyhow!("filecloud path is invalid: {error}"))?;
    Ok(())
}

pub fn percent_decode_path(value: &str) -> Result<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = &bytes[index + 1..index + 3];
                let decoded =
                    u8::from_str_radix(std::str::from_utf8(hex)?, 16).map_err(|error| {
                        anyhow!("invalid percent-encoding in filecloud path: {error}")
                    })?;
                output.push(decoded);
                index += 3;
            }
            b'%' => return Err(anyhow!("invalid percent-encoding in filecloud path")),
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    Ok(String::from_utf8(output)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        crate::test_support::temp_base().join(format!("proxysss-filecloud-{nanos}"))
    }

    #[test]
    fn resolve_relative_path_rejects_parent_segments() {
        let root = temp_root();
        fs::create_dir_all(&root).expect("create temp root");
        let error = resolve_relative_path(&root, "../secret.txt").expect_err("expected escape");
        assert!(error.to_string().contains("escapes root"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ensure_within_root_blocks_outside_target() {
        let root = temp_root();
        let outside = temp_root();
        fs::create_dir_all(&root).expect("create temp root");
        let error = ensure_within_root(&root, &outside.join("file.txt")).expect_err("outside root");
        assert!(error.to_string().contains("escapes"));
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(outside);
    }

    #[cfg(unix)]
    #[test]
    fn ensure_within_root_rejects_dangling_symlink_write_targets() {
        let fixture = crate::test_support::TestDir::new("filecloud-boundary");
        let root = fixture.join("served");
        fs::create_dir_all(&root).unwrap();
        std::os::unix::fs::symlink(fixture.join("new-outside-file"), root.join("dangling"))
            .unwrap();
        assert!(ensure_within_root(&root, &root.join("dangling")).is_err());
        assert!(ensure_within_root(&root, &root.join("dangling/child")).is_err());
    }
}
