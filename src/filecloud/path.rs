use std::path::{Component, Path, PathBuf};

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
    let decoded = percent_decode_path(relative.trim_start_matches('/'))?;
    let mut target = root.to_path_buf();

    for part in decoded.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err(anyhow!("filecloud path escapes root"));
        }
        let component_path = Path::new(part);
        if component_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
        {
            return Err(anyhow!("filecloud path escapes root"));
        }
        target.push(part);
    }

    Ok(target)
}

pub fn ensure_within_root(root: &Path, target: &Path) -> Result<()> {
    let root = root
        .canonicalize()
        .map_err(|error| anyhow!("filecloud root is unavailable: {error}"))?;

    let target = if target.is_absolute() {
        target.to_path_buf()
    } else {
        root.join(target)
    };

    if target.exists() {
        let canonical = target
            .canonicalize()
            .map_err(|error| anyhow!("filecloud path is invalid: {error}"))?;
        if !canonical.starts_with(&root) {
            return Err(anyhow!("filecloud path escapes configured root"));
        }
        return Ok(());
    }

    let mut cursor = target.clone();
    while !cursor.exists() {
        if !cursor.pop() {
            return Err(anyhow!("filecloud path escapes configured root"));
        }
    }
    let canonical = cursor
        .canonicalize()
        .map_err(|error| anyhow!("filecloud path parent is invalid: {error}"))?;
    if !canonical.starts_with(&root) {
        return Err(anyhow!("filecloud path escapes configured root"));
    }
    Ok(())
}

fn percent_decode_path(value: &str) -> Result<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                let hex = &bytes[index + 1..index + 3];
                let decoded =
                    u8::from_str_radix(std::str::from_utf8(hex)?, 16).map_err(|error| {
                        anyhow!("invalid percent-encoding in filecloud path: {error}")
                    })?;
                output.push(decoded);
                index += 3;
            }
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
        std::env::temp_dir().join(format!("proxysss-filecloud-{nanos}"))
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
}
