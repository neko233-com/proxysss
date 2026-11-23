//! nginx `gzip_static` / `brotli_static` parity: serve a pre-compressed sibling
//! (`.br` / `.gz`) when the client accepts it, before reading the original body.
//! The compressed file is confined to the site root and obeys dotfile rules; the
//! response keeps the original Content-Type and gains Content-Encoding + Vary.
use std::path::{Path, PathBuf};

use http::HeaderMap;

use crate::config::{CompressionAlgorithm, StaticPrecompressedConfig};

/// True when the client advertises `coding` with a non-zero q-value.
pub fn accepts(headers: &HeaderMap, coding: &str) -> bool {
    let Some(value) = headers.get(http::header::ACCEPT_ENCODING) else {
        return false;
    };
    let Ok(value) = value.to_str() else {
        return false;
    };
    for token in value.split(',') {
        let mut parts = token.split(';');
        let Some(name) = parts.next() else { continue };
        if !name.trim().eq_ignore_ascii_case(coding) {
            continue;
        }
        let mut q = 1.0f32;
        for param in parts {
            let param = param.trim();
            if let Some(rest) = param.strip_prefix("q=") {
                q = rest.trim().parse::<f32>().unwrap_or(1.0);
            }
        }
        if q > 0.0 {
            return true;
        }
    }
    false
}

fn extension(algorithm: CompressionAlgorithm) -> &'static str {
    match algorithm {
        CompressionAlgorithm::Brotli => "br",
        CompressionAlgorithm::Gzip => "gz",
        CompressionAlgorithm::Zstd => "zst",
    }
}

fn encoding(algorithm: CompressionAlgorithm) -> &'static str {
    match algorithm {
        CompressionAlgorithm::Brotli => "br",
        CompressionAlgorithm::Gzip => "gzip",
        CompressionAlgorithm::Zstd => "zstd",
    }
}

/// Pick the best existing pre-compressed sibling for `target`, honoring the
/// configured algorithm order (default brotli then gzip) and client Accept-Encoding.
pub async fn select(
    root: &Path,
    target: &Path,
    headers: &HeaderMap,
    config: &StaticPrecompressedConfig,
) -> Option<(PathBuf, &'static str)> {
    if !config.enabled {
        return None;
    }
    let order: Vec<CompressionAlgorithm> = if config.algorithms.is_empty() {
        vec![CompressionAlgorithm::Brotli, CompressionAlgorithm::Gzip]
    } else {
        config.algorithms.clone()
    };
    for algorithm in order {
        let coding = encoding(algorithm);
        if !accepts(headers, coding) {
            continue;
        }
        let mut candidate = target.as_os_str().to_os_string();
        candidate.push(".");
        candidate.push(extension(algorithm));
        let candidate = PathBuf::from(candidate);
        let Some(confined) =
            crate::static_security::confined_target(root, &candidate, true).await
        else {
            continue;
        };
        if tokio::fs::metadata(&confined)
            .await
            .map(|m| m.is_file())
            .unwrap_or(false)
        {
            return Some((confined, coding));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(http::header::ACCEPT_ENCODING, value.parse().unwrap());
        h
    }

    #[test]
    fn accept_encoding_respects_q_and_case() {
        assert!(accepts(&headers("gzip, br"), "br"));
        assert!(accepts(&headers("BR;q=0.8, gzip"), "br"));
        assert!(!accepts(&headers("br;q=0, gzip"), "br"));
        assert!(accepts(&headers("br;q=0, gzip"), "gzip"));
        assert!(!accepts(&headers("identity"), "gzip"));
        assert!(!accepts(&HeaderMap::new(), "gzip"));
    }

    #[tokio::test]
    async fn selects_best_sibling_within_root_and_falls_back() {
        let dir = crate::test_support::TestDir::new("precompressed");
        let root = &*dir;
        let original = root.join("a.js");
        tokio::fs::write(&original, b"console.log(1)").await.unwrap();
        tokio::fs::write(root.join("a.js.gz"), b"gzip-bytes").await.unwrap();
        tokio::fs::write(root.join("a.js.br"), b"br-bytes").await.unwrap();
        let cfg = StaticPrecompressedConfig { enabled: true, algorithms: vec![] };

        let (path, enc) = select(root, &original, &headers("gzip, br"), &cfg).await.unwrap();
        assert_eq!(enc, "br");
        assert_eq!(path, root.join("a.js.br"));

        let (path, enc) = select(root, &original, &headers("gzip"), &cfg).await.unwrap();
        assert_eq!(enc, "gzip");
        assert_eq!(path, root.join("a.js.gz"));

        assert!(select(root, &original, &headers("identity"), &cfg).await.is_none());
        assert!(select(root, &root.join("missing.js"), &headers("br"), &cfg).await.is_none());

        let off = StaticPrecompressedConfig { enabled: false, algorithms: vec![] };
        assert!(select(root, &original, &headers("br"), &off).await.is_none());
    }

    #[tokio::test]
    async fn sibling_symlink_escape_is_rejected() {
        let dir = crate::test_support::TestDir::new("precompressed");
        let root = dir.join("root");
        tokio::fs::create_dir_all(&root).await.unwrap();
        let outside = dir.join("outside");
        tokio::fs::write(&outside, b"secret").await.unwrap();
        let original = root.join("a.js");
        tokio::fs::write(&original, b"x").await.unwrap();
        // a.js.br -> outside file must not be served.
        #[cfg(unix)]
        tokio::fs::symlink(&outside, root.join("a.js.br")).await.unwrap();
        #[cfg(windows)]
        tokio::fs::symlink_file(&outside, root.join("a.js.br")).await.unwrap();
        let cfg = StaticPrecompressedConfig { enabled: true, algorithms: vec![] };
        assert!(select(&root, &original, &headers("br"), &cfg).await.is_none());
    }
}