use std::time::Duration;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::verify::harness::{
    base_gateway_yaml, cleanup, reserve_port, spawn_gateway, spawn_json_echo_upstream,
    spawn_ws_echo_upstream, temp_root, wait_http_ok, write_config,
};

#[tokio::test]
async fn integration_deep_builtin_welcome_and_docs() -> Result<()> {
    let root = temp_root("proxysss-deep-builtin");
    let gateway_port = reserve_port().await?;
    let config_path = write_config(&root, &base_gateway_yaml(gateway_port))?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let base = format!("http://127.0.0.1:{gateway_port}");
    wait_http_ok(&format!("{base}/")).await?;
    wait_http_ok(&format!("{base}/docs.html")).await?;
    let metrics = reqwest::get(format!("{base}/metrics")).await?;
    assert!(metrics.status().is_success());
    assert!(metrics.text().await?.contains("proxysss_"));

    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_deep_static_site_serves_files() -> Result<()> {
    let root = temp_root("proxysss-deep-static");
    let gateway_port = reserve_port().await?;
    let static_root = root.join("public");
    std::fs::create_dir_all(&static_root)?;
    std::fs::write(static_root.join("hello.txt"), b"static-ok")?;
    let static_fwd = static_root.display().to_string().replace('\\', "/");
    let yaml = format!(
        "{}\nservices:\n  static_sites:\n    - name: test-static\n      path_prefix: /assets\n      root: '{static_fwd}'\n      index_files: [index.html]\n      autoindex: false\n",
        base_gateway_yaml(gateway_port)
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let url = format!("http://127.0.0.1:{gateway_port}/assets/hello.txt");
    wait_http_ok(&url).await?;
    assert_eq!(reqwest::get(&url).await?.text().await?, "static-ok");

    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_deep_static_site_supports_range_downloads() -> Result<()> {
    let root = temp_root("proxysss-deep-static-range");
    let gateway_port = reserve_port().await?;
    let static_root = root.join("downloads");
    std::fs::create_dir_all(&static_root)?;
    std::fs::write(static_root.join("movie.bin"), b"0123456789abcdef")?;
    let static_fwd = static_root.display().to_string().replace('\\', "/");
    let yaml = format!(
        "{}\nservices:\n  static_sites:\n    - name: downloads\n      path_prefix: /dl\n      root: '{static_fwd}'\n      index_files: [index.html]\n      autoindex: false\n",
        base_gateway_yaml(gateway_port)
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let url = format!("http://127.0.0.1:{gateway_port}/dl/movie.bin");
    wait_http_ok(&url).await?;
    let response = reqwest::Client::new()
        .get(&url)
        .header(reqwest::header::RANGE, "bytes=4-9")
        .send()
        .await?;

    assert_eq!(response.status(), reqwest::StatusCode::PARTIAL_CONTENT);
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok()),
        Some("bytes 4-9/16")
    );
    assert_eq!(response.bytes().await?.as_ref(), b"456789");

    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_deep_webdav_put_get_delete() -> Result<()> {
    let root = temp_root("proxysss-deep-webdav");
    let gateway_port = reserve_port().await?;
    let dav_root = root.join("webdav");
    std::fs::create_dir_all(&dav_root)?;
    let dav_fwd = dav_root.display().to_string().replace('\\', "/");
    let yaml = format!(
        "{}\nservices:\n  webdav:\n    enabled: true\n    path_prefix: /dav\n    root: '{dav_fwd}'\n    allow_write: true\n",
        base_gateway_yaml(gateway_port)
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let base = format!("http://127.0.0.1:{gateway_port}/dav/item.txt");
    let client = reqwest::Client::new();
    assert!(client
        .put(&base)
        .body("webdav-payload")
        .send()
        .await?
        .status()
        .is_success());
    wait_http_ok(&base).await?;
    assert_eq!(
        client.get(&base).send().await?.text().await?,
        "webdav-payload"
    );
    assert_eq!(
        client.delete(&base).send().await?.status(),
        reqwest::StatusCode::NO_CONTENT
    );

    cleanup(&root);
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn integration_deep_webdav_rejects_symlink_escape_over_http() -> Result<()> {
    let fixture = crate::test_support::TestDir::new("webdav-http-boundary");
    let root = fixture.join("served");
    let sibling = fixture.join("sibling");
    std::fs::create_dir_all(&root)?;
    std::fs::create_dir_all(&sibling)?;
    std::fs::write(sibling.join("secret"), b"sibling-only")?;
    std::fs::write(root.join("source"), b"source")?;
    std::os::unix::fs::symlink(&sibling, root.join("outside"))?;
    std::os::unix::fs::symlink(sibling.join("missing"), root.join("dangling"))?;
    let port = reserve_port().await?;
    let yaml = format!("{}\nservices:\n  webdav:\n    enabled: true\n    path_prefix: /dav\n    root: '{}'\n    allow_write: true\n", base_gateway_yaml(port), root.display());
    let (_gateway, _runner) = spawn_gateway(write_config(&fixture, &yaml)?).await?;
    let base = format!("http://127.0.0.1:{port}");
    wait_http_ok(&format!("{base}/healthz")).await?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()?;
    for method in [
        "GET", "HEAD", "PROPFIND", "PUT", "DELETE", "MKCOL", "COPY", "MOVE",
    ] {
        let response = client
            .request(
                reqwest::Method::from_bytes(method.as_bytes())?,
                format!("{base}/dav/outside/secret"),
            )
            .header("destination", "/dav/copy")
            .body("overwrite")
            .send()
            .await?;
        assert_eq!(response.status().as_u16(), 403, "{method}");
        let _ = response.bytes().await?;
    }
    for method in ["COPY", "MOVE"] {
        let response = client
            .request(
                reqwest::Method::from_bytes(method.as_bytes())?,
                format!("{base}/dav/source"),
            )
            .header("destination", format!("{base}/dav/dangling"))
            .send()
            .await?;
        assert_eq!(response.status().as_u16(), 403);
        let _ = response.bytes().await?;
    }
    assert_eq!(
        client
            .put(format!("{base}/dav/dangling"))
            .body("new")
            .send()
            .await?
            .status()
            .as_u16(),
        403
    );
    assert!(!sibling.join("missing").exists());
    assert_eq!(std::fs::read(sibling.join("secret"))?, b"sibling-only");
    assert_eq!(
        client
            .get(format!("{base}/dav/source"))
            .send()
            .await?
            .bytes()
            .await?
            .as_ref(),
        b"source"
    );
    Ok(())
}

#[tokio::test]
async fn integration_deep_reverse_proxy_strip_prefix_and_host() -> Result<()> {
    let root = temp_root("proxysss-deep-proxy");
    let upstream_port = reserve_port().await?;
    let gateway_port = reserve_port().await?;
    let _upstream = spawn_json_echo_upstream(upstream_port).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let yaml = format!(
        r#"{base}
services:
  reverse_proxy:
    routes:
      - name: api
        path_prefix: /api/v1
        strip_prefix: true
        upstream: http://127.0.0.1:{upstream_port}
"#,
        base = base_gateway_yaml(gateway_port),
        upstream_port = upstream_port
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let url = format!("http://127.0.0.1:{gateway_port}/api/v1/users");
    wait_http_ok(&url).await?;
    let payload: serde_json::Value = reqwest::get(&url).await?.json().await?;
    assert_eq!(payload["path"], "/users");

    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_deep_domain_route_matches_host() -> Result<()> {
    let root = temp_root("proxysss-deep-domain");
    let upstream_port = reserve_port().await?;
    let gateway_port = reserve_port().await?;
    let _upstream = spawn_json_echo_upstream(upstream_port).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let yaml = format!(
        r#"{base}
services:
  domain_routes:
    - name: tenant-a
      domains: [tenant-a.test]
      path_prefix: /
      upstream: http://127.0.0.1:{upstream_port}
"#,
        base = base_gateway_yaml(gateway_port),
        upstream_port = upstream_port
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let url = format!("http://127.0.0.1:{gateway_port}/tenant");
    let response = reqwest::Client::new()
        .get(&url)
        .header("host", "tenant-a.test")
        .send()
        .await?;
    assert!(
        response.status().is_success(),
        "status {}",
        response.status()
    );
    let payload: serde_json::Value = response.json().await?;
    assert_eq!(payload["host"], "tenant-a.test");

    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_deep_cache_sets_x_cache_header() -> Result<()> {
    let root = temp_root("proxysss-deep-cache");
    let upstream_port = reserve_port().await?;
    let gateway_port = reserve_port().await?;
    let _upstream = spawn_json_echo_upstream(upstream_port).await?;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let yaml = format!(
        r#"{base}
services:
  reverse_proxy:
    routes:
      - name: cached-api
        path_prefix: /cached
        upstream: http://127.0.0.1:{upstream_port}
        cache:
          enabled: true
          behavior: override
          ttl_secs: 60
"#,
        base = base_gateway_yaml(gateway_port),
        upstream_port = upstream_port
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let url = format!("http://127.0.0.1:{gateway_port}/cached/item");
    wait_http_ok(&url).await?;
    let first = reqwest::get(&url).await?;
    let cache_header = first
        .headers()
        .get("x-cache")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(cache_header.contains("MISS") || cache_header.contains("HIT"));

    let second = reqwest::get(&url).await?;
    let cache_header = second
        .headers()
        .get("x-cache")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(cache_header.contains("HIT"), "got {cache_header}");

    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_deep_compression_negotiates_brotli_or_gzip() -> Result<()> {
    let root = temp_root("proxysss-deep-compress");
    let gateway_port = reserve_port().await?;
    let static_root = root.join("public");
    std::fs::create_dir_all(&static_root)?;
    std::fs::write(static_root.join("big.txt"), vec![b'a'; 4096])?;
    let static_fwd = static_root.display().to_string().replace('\\', "/");

    let yaml = format!(
        r#"{base}
services:
  response_policy:
    compression:
      enabled: true
      brotli: true
      gzip: true
      min_length: 128
  static_sites:
    - name: compress-static
      path_prefix: /big
      root: '{static_fwd}'
      index_files: []
      autoindex: false
"#,
        base = base_gateway_yaml(gateway_port)
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let url = format!("http://127.0.0.1:{gateway_port}/big/big.txt");
    wait_http_ok(&url).await?;
    let response = reqwest::Client::new()
        .get(&url)
        .header("accept-encoding", "br, gzip")
        .send()
        .await?;
    let encoding = response
        .headers()
        .get("content-encoding")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(encoding == "br" || encoding == "gzip", "got {encoding}");

    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_deep_monitoring_json_format() -> Result<()> {
    let root = temp_root("proxysss-deep-metrics-json");
    let gateway_port = reserve_port().await?;
    let yaml = format!(
        "{}\nmonitoring:\n  enabled: true\n  path: /metrics\n  format: json\n",
        base_gateway_yaml(gateway_port)
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let url = format!("http://127.0.0.1:{gateway_port}/metrics");
    wait_http_ok(&url).await?;
    let payload: serde_json::Value = reqwest::get(&url).await?.json().await?;
    assert!(payload.get("http_requests").is_some());

    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_deep_websocket_and_grpc_style_post() -> Result<()> {
    let root = temp_root("proxysss-deep-ws-grpc");
    let upstream_port = reserve_port().await?;
    let gateway_port = reserve_port().await?;
    let _upstream = spawn_ws_echo_upstream(upstream_port).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let yaml = format!(
        r#"{base}
services:
  reverse_proxy:
    routes:
      - name: ws-route
        path_prefix: /realtime
        upstream: ws://127.0.0.1:{upstream_port}
"#,
        base = base_gateway_yaml(gateway_port),
        upstream_port = upstream_port
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, _runner) = spawn_gateway(config_path).await?;

    let gateway_url = format!("ws://127.0.0.1:{gateway_port}/realtime/");
    let (mut client, _) = connect_async(&gateway_url).await?;
    client.send(Message::Text("deep-verify".into())).await?;
    let echoed = client.next().await.unwrap()?;
    assert_eq!(echoed, Message::Text("deep-verify".into()));

    cleanup(&root);
    Ok(())
}

#[tokio::test]
async fn integration_deep_http2_get_body_and_request_size_limits() -> Result<()> {
    use bytes::Bytes;
    use http_body_util::{BodyExt, StreamBody};
    use hyper::body::Frame;
    use hyper_util::rt::{TokioExecutor, TokioIo};
    let root = crate::test_support::TestDir::new("request-body-limits");
    let gateway_port = reserve_port().await?;
    let upstream_port = reserve_port().await?;
    let upstream = spawn_json_echo_upstream(upstream_port).await?;
    let base = base_gateway_yaml(gateway_port).replace(
        "http:\n",
        "http:\n  max_request_body_bytes: 5\n  request_body_timeout_ms: 1000\n",
    );
    let yaml = format!("{base}\nservices:\n  reverse_proxy:\n    routes:\n      - name: echo\n        path_prefix: /echo\n        upstream: http://127.0.0.1:{upstream_port}\n");
    let (_, runner) = spawn_gateway(write_config(&root, &yaml)?).await?;
    let socket = tokio::net::TcpStream::connect(("127.0.0.1", gateway_port)).await?;
    let (mut sender, connection) =
        hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(socket)).await?;
    let driver = tokio::spawn(connection);
    for (payload, expected) in [
        ("hello", http::StatusCode::OK),
        ("sixsix", http::StatusCode::PAYLOAD_TOO_LARGE),
    ] {
        let frames = futures::stream::iter([Ok::<_, std::convert::Infallible>(Frame::data(
            Bytes::copy_from_slice(payload.as_bytes()),
        ))]);
        let request = http::Request::get(format!("http://127.0.0.1:{gateway_port}/echo"))
            .body(StreamBody::new(frames))?;
        assert!(!request.headers().contains_key(http::header::CONTENT_LENGTH));
        let response =
            tokio::time::timeout(Duration::from_secs(5), sender.send_request(request)).await??;
        assert_eq!(response.status(), expected);
        let body = response.into_body().collect().await?.to_bytes();
        if expected.is_success() {
            let result: serde_json::Value = serde_json::from_slice(&body)?;
            assert_eq!(result["body_len"], 5);
        }
    }
    driver.abort();
    runner.abort();
    upstream.abort();
    Ok(())
}
