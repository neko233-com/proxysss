//! nginx `try_files` parity for static sites: ordered root-contained candidates.
//! `$uri` expands to the request path relative to the site prefix; a `=NNN`
//! candidate terminates with that status. The first existing file wins.
use std::path::{Path, PathBuf};

pub enum Outcome {
    File(PathBuf),
    Status(u16),
    None,
}

pub async fn resolve(
    root: &Path,
    relative: &str,
    candidates: &[String],
    hide_dotfiles: bool,
) -> Outcome {
    for candidate in candidates {
        let candidate = candidate.trim();
        if let Some(status) = candidate.strip_prefix('=') {
            if let Ok(code) = status.parse::<u16>() {
                return Outcome::Status(code);
            }
            continue;
        }
        let expanded = candidate.replace("$uri", relative);
        let expanded = expanded.trim_start_matches('/');
        if expanded.is_empty() {
            continue;
        }
        let joined = root.join(expanded);
        let Some(confined) = crate::static_security::confined_target(root, &joined, hide_dotfiles).await
        else {
            continue;
        };
        if tokio::fs::metadata(&confined)
            .await
            .map(|m| m.is_file())
            .unwrap_or(false)
        {
            return Outcome::File(confined);
        }
    }
    Outcome::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ordered_candidates_uri_expansion_and_terminal_status() {
        let dir = crate::test_support::TestDir::new("tryfiles");
        let root: &Path = &dir;
        tokio::fs::write(root.join("index.html"), b"i").await.unwrap();
        tokio::fs::write(root.join("app.html"), b"a").await.unwrap();

        let cands = vec!["$uri".to_string(), "/app.html".to_string(), "=404".to_string()];
        match resolve(root, "missing.html", &cands, true).await {
            Outcome::File(p) => assert_eq!(
                p,
                tokio::fs::canonicalize(root.join("app.html")).await.unwrap()
            ),
            _ => panic!("expected fallback file"),
        }
        match resolve(root, "index.html", &cands, true).await {
            Outcome::File(p) => assert_eq!(
                p,
                tokio::fs::canonicalize(root.join("index.html")).await.unwrap()
            ),
            _ => panic!("expected uri file"),
        }
        let only_status = vec!["=503".to_string()];
        match resolve(root, "x", &only_status, true).await {
            Outcome::Status(503) => {}
            _ => panic!("expected terminal status"),
        }
        assert!(matches!(resolve(root, "x", &[], true).await, Outcome::None));
    }

    #[tokio::test]
    async fn traversal_candidates_are_rejected() {
        let dir = crate::test_support::TestDir::new("tryfiles-escape");
        let root = dir.join("root");
        tokio::fs::create_dir_all(&root).await.unwrap();
        tokio::fs::write(dir.join("secret"), b"s").await.unwrap();
        let cands = vec!["../secret".to_string()];
        assert!(matches!(resolve(&root, "x", &cands, true).await, Outcome::None));
    }
}