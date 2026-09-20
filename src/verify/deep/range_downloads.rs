use anyhow::Result;
use reqwest::{Client, StatusCode};

use crate::test_support::TestDir;
use crate::verify::harness::{base_gateway_yaml, reserve_port, spawn_gateway, write_config};

#[tokio::test]
async fn integration_deep_sparse_http1_keepalive_refreshes_and_removes_static_assets() -> Result<()>
{
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;
    async fn fetch(stream: &mut BufReader<TcpStream>) -> Result<(u16, Vec<u8>)> {
        stream
            .get_mut()
            .write_all(b"GET /assets/asset.txt HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await?;
        let mut status_line = String::new();
        stream.read_line(&mut status_line).await?;
        let status = status_line
            .split_whitespace()
            .nth(1)
            .unwrap_or("0")
            .parse()?;
        let mut length = None;
        loop {
            let mut line = String::new();
            anyhow::ensure!(stream.read_line(&mut line).await? > 0, "truncated response");
            if line == "\r\n" {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    length = Some(value.trim().parse::<usize>()?);
                }
            }
        }
        let length = length.ok_or_else(|| anyhow::anyhow!("missing response length"))?;
        anyhow::ensure!(length <= 65536, "unexpected fixture body");
        let mut body = vec![0; length];
        stream.read_exact(&mut body).await?;
        Ok((status, body))
    }
    let root = TestDir::new("sparse-plain-static");
    let public = root.join("public");
    std::fs::create_dir_all(&public)?;
    let path = public.join("asset.txt");
    std::fs::write(&path, b"version-one")?;
    let port = reserve_port().await?;
    let yaml = format!("{}\nservices:\n  static_sites:\n    - name: downloads\n      path_prefix: /assets\n      root: '{}'\n", base_gateway_yaml(port), public.display().to_string().replace('\\', "/"));
    let (_gateway, runner) = spawn_gateway(write_config(&root, &yaml)?).await?;
    let mut stream = BufReader::new(TcpStream::connect(("127.0.0.1", port)).await?);
    assert_eq!(fetch(&mut stream).await?, (200, b"version-one".to_vec()));
    std::fs::write(&path, b"version-two")?;
    for removed in [false, true] {
        if removed {
            std::fs::remove_file(&path)?;
        }
        let mut observed = false;
        for _ in 0..60 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let (status, body) =
                tokio::time::timeout(std::time::Duration::from_secs(2), fetch(&mut stream))
                    .await??;
            if (removed && status == 404) || (!removed && status == 200 && body == b"version-two") {
                observed = true;
                break;
            }
        }
        assert!(
            observed,
            "sparse identical requests must observe the time-based cache expiry; removed={removed}"
        );
    }
    runner.abort();
    Ok(())
}

#[tokio::test]
async fn integration_deep_tls_static_snapshot_survives_publish_and_deletion() -> Result<()> {
    let root = TestDir::new("tls-static-publish");
    let public = root.join("public");
    std::fs::create_dir_all(&public)?;
    let path = public.join("asset.bin");
    let original = vec![b'x'; 2 * 1024 * 1024];
    std::fs::write(&path, &original)?;
    let port = reserve_port().await?;
    let tls_port = reserve_port().await?;
    let base = base_gateway_yaml(port)
        .replace("tls_bind: ''", &format!("tls_bind: 127.0.0.1:{tls_port}"))
        .replace("traffic_profile: small", "traffic_profile: balanced");
    let yaml = format!("{base}\nservices:\n  static_sites:\n    - name: downloads\n      path_prefix: /assets\n      root: '{}'\n", public.display().to_string().replace('\\', "/"));
    let (_gateway, runner) = spawn_gateway(write_config(&root, &yaml)?).await?;
    let client = Client::builder()
        .http2_prior_knowledge()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    let url = format!("https://127.0.0.1:{tls_port}/assets/asset.bin");
    assert_eq!(
        client.get(&url).send().await?.bytes().await?.as_ref(),
        original
    );
    std::fs::write(&path, b"replacement")?;
    let immediate = client.get(&url).send().await?.bytes().await?;
    assert!(immediate.as_ref() == original || immediate.as_ref() == b"replacement");
    let mut updated = false;
    for _ in 0..80 {
        if client.get(&url).send().await?.bytes().await?.as_ref() == b"replacement" {
            updated = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(
        updated,
        "H2 snapshot must refresh after in-place publication"
    );
    std::fs::remove_file(&path)?;
    let mut removed = false;
    for _ in 0..80 {
        let response = client.get(&url).send().await?;
        let status = response.status();
        let _ = response.bytes().await?;
        if status == StatusCode::NOT_FOUND {
            removed = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    assert!(
        removed,
        "cached H2 connection must not serve a removed file forever"
    );
    runner.abort();
    Ok(())
}

#[tokio::test]
async fn integration_deep_complex_downloads_http1_h2_tls() -> Result<()> {
    let root = TestDir::new("complex-downloads");
    let public = root.join("public");
    std::fs::create_dir_all(&public)?;
    std::fs::write(public.join("asset.bin"), b"0123456789abcdef")?;
    let port = reserve_port().await?;
    let tls_port = reserve_port().await?;
    let base =
        base_gateway_yaml(port).replace("tls_bind: ''", &format!("tls_bind: 127.0.0.1:{tls_port}"));
    let yaml = format!(
        "{base}\nservices:\n  static_sites:\n    - name: downloads\n      path_prefix: /assets\n      root: '{}'\n",
        public.display().to_string().replace('\\', "/"),
    );
    let (_gateway, runner) = spawn_gateway(write_config(&root, &yaml)?).await?;
    let clients = [
        (
            Client::builder().http1_only().build()?,
            format!("http://127.0.0.1:{port}"),
        ),
        (
            Client::builder().http2_prior_knowledge().build()?,
            format!("http://127.0.0.1:{port}"),
        ),
        (
            Client::builder()
                .http1_only()
                .danger_accept_invalid_certs(true)
                .build()?,
            format!("https://127.0.0.1:{tls_port}"),
        ),
        (
            Client::builder()
                .http2_prior_knowledge()
                .danger_accept_invalid_certs(true)
                .build()?,
            format!("https://127.0.0.1:{tls_port}"),
        ),
    ];
    for (client, base) in clients {
        let url = format!("{base}/assets/asset.bin");
        // Warm the same connection before exercising conditional/Range bypass.
        let warm = client.get(&url).send().await?;
        assert_eq!(warm.bytes().await?.as_ref(), b"0123456789abcdef");
        // Explicit validation enters the metadata path even when the immutable
        // fast lane omits optional validators from ordinary successful GETs.
        let full = client
            .get(&url)
            .header("if-none-match", "\"not-current\"")
            .send()
            .await?;
        let etag = full.headers()["etag"].to_str()?.to_string();
        assert_eq!(full.bytes().await?.as_ref(), b"0123456789abcdef");
        let response = client
            .get(&url)
            .header("range", "bytes=12-15,0-3,99-100")
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT, "{base}");
        assert!(!response.headers().contains_key("content-range"));
        assert!(!response.headers().contains_key("content-encoding"));
        let length = response.content_length().unwrap();
        let boundary = response.headers()["content-type"]
            .to_str()?
            .strip_prefix("multipart/byteranges; boundary=")
            .unwrap()
            .to_string();
        let expected = format!("--{boundary}\r\nContent-Type: application/octet-stream\r\nContent-Range: bytes 12-15/16\r\n\r\ncdef\r\n--{boundary}\r\nContent-Type: application/octet-stream\r\nContent-Range: bytes 0-3/16\r\n\r\n0123\r\n--{boundary}--\r\n");
        let body = response.bytes().await?;
        assert_eq!(body.len() as u64, length);
        assert_eq!(body.as_ref(), expected.as_bytes());

        for range in ["bytes=1-2", "bytes=1-2,6-7", "bytes=999-"] {
            let head = client.head(&url).header("range", range).send().await?;
            assert_eq!(head.status(), StatusCode::OK, "HEAD {range}: {base}");
            assert_eq!(head.headers()["content-length"], "16");
            assert!(!head.headers().contains_key("content-range"));
            assert!(head.bytes().await?.is_empty());
        }
        let merged = client
            .get(&url)
            .header("range", "bytes=0-3,2-5,6-7")
            .send()
            .await?;
        assert_eq!(merged.headers()["content-range"], "bytes 0-7/16");
        assert_eq!(merged.bytes().await?.as_ref(), b"01234567");
        let unsat = client
            .get(&url)
            .header("range", "bytes=99-100,-0")
            .send()
            .await?;
        assert_eq!(unsat.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        assert_eq!(unsat.headers()["content-range"], "bytes */16");
        let _ = unsat.bytes().await?;
        for range in [
            "bytes=3-1".to_string(),
            format!("bytes={}", vec!["0-1"; 17].join(",")),
        ] {
            let response = client.get(&url).header("range", range).send().await?;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.bytes().await?.as_ref(), b"0123456789abcdef");
        }
        let cached = client
            .get(&url)
            .header("if-none-match", &etag)
            .header("range", "bytes=0-1,8-9")
            .send()
            .await?;
        assert_eq!(cached.status(), StatusCode::NOT_MODIFIED);
        assert!(cached.bytes().await?.is_empty());
        let precondition = client
            .get(&url)
            .header("if-match", "\"missing\"")
            .header("if-none-match", &etag)
            .header("range", "bytes=0-1")
            .send()
            .await?;
        assert_eq!(precondition.status(), StatusCode::PRECONDITION_FAILED);
        let _ = precondition.bytes().await?;
        let unchanged = client
            .get(&url)
            .header("if-unmodified-since", "Sun, 06 Nov 1994 08:49:37 GMT")
            .send()
            .await?;
        assert_eq!(unchanged.status(), StatusCode::PRECONDITION_FAILED);
        let _ = unchanged.bytes().await?;
        // If-Match takes precedence over If-Unmodified-Since. A weak entity tag
        // cannot pass strong comparison, but '*' accepts this existing file.
        let exists = client
            .get(&url)
            .header("if-match", "*")
            .header("if-unmodified-since", "Sun, 06 Nov 1994 08:49:37 GMT")
            .header("range", "bytes=-4")
            .send()
            .await?;
        assert_eq!(exists.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(exists.bytes().await?.as_ref(), b"cdef");
        let weak = client.get(&url).header("if-match", &etag).send().await?;
        assert_eq!(weak.status(), StatusCode::PRECONDITION_FAILED);
        let _ = weak.bytes().await?;
    }
    runner.abort();
    Ok(())
}
