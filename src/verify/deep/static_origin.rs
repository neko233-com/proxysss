use anyhow::Result;
use reqwest::{Client, StatusCode};

use crate::config::GatewayConfig;
use crate::static_security::{sign, ORIGIN_TOKEN_HEADER};
use crate::test_support::TestDir;
use crate::verify::harness::{base_gateway_yaml, reserve_port, spawn_gateway, write_config};

const TOKEN: &str = "test-only-cdn-origin-token-at-least-32-bytes";
const SECRET: &str = "test-only-static-signing-secret-at-least-32-bytes";

#[tokio::test]
async fn integration_deep_static_origin_auth_http1_h2_https() -> Result<()> {
    let root = TestDir::new("static-origin-protocols");
    let public = root.join("public");
    std::fs::create_dir_all(&public)?;
    std::fs::write(public.join("asset.bin"), b"0123456789abcdef")?;
    let port = reserve_port().await?;
    let tls_port = reserve_port().await?;
    let base =
        base_gateway_yaml(port).replace("tls_bind: ''", &format!("tls_bind: 127.0.0.1:{tls_port}"));
    let yaml = format!(
        r#"{base}
services:
  static_sites:
    - name: private
      path_prefix: /assets
      root: '{}'
      security:
        origin_token: {TOKEN}
        allowed_peers: [127.0.0.1/32]
        signed_url:
          enabled: true
          secret: {SECRET}
          max_ttl_secs: 300
    - name: denied-peer
      path_prefix: /denied
      root: '{}'
      security:
        allowed_peers: [192.0.2.1/32]
"#,
        public.display().to_string().replace('\\', "/"),
        public.display().to_string().replace('\\', "/")
    );
    let config_path = write_config(&root, &yaml)?;
    let config = GatewayConfig::load(&config_path)?;
    let site = &config.services.static_sites[0];
    let (_gateway, runner) = spawn_gateway(config_path).await?;
    let now = crate::static_security::now_secs();
    let path = sign(site, "/assets/asset.bin", 300, now)?;
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
        let url = format!("{base}{path}");
        let response = client
            .get(&url)
            .header(ORIGIN_TOKEN_HEADER, TOKEN)
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::OK, "{base}");
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        assert_eq!(response.headers()["cdn-cache-control"], "no-store");
        let etag = response.headers()["etag"].to_str()?.to_string();
        assert_eq!(response.bytes().await?.as_ref(), b"0123456789abcdef");
        // Same client/pool, now without origin authorization: no warmed connection/cache bypass.
        assert_eq!(
            client.get(&url).send().await?.status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            client.head(&url).send().await?.status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            client
                .get(&url)
                .header("if-none-match", &etag)
                .send()
                .await?
                .status(),
            StatusCode::FORBIDDEN
        );
        let head = client
            .head(&url)
            .header(ORIGIN_TOKEN_HEADER, TOKEN)
            .send()
            .await?;
        assert_eq!(head.status(), StatusCode::OK);
        assert_eq!(head.headers()["content-length"], "16");
        assert!(head.bytes().await?.is_empty());
        let partial = client
            .get(&url)
            .header(ORIGIN_TOKEN_HEADER, TOKEN)
            .header("range", "bytes=4-9")
            .send()
            .await?;
        assert_eq!(partial.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(partial.headers()["content-range"], "bytes 4-9/16");
        assert_eq!(partial.bytes().await?.as_ref(), b"456789");
        assert_eq!(
            client
                .get(&url)
                .header(ORIGIN_TOKEN_HEADER, TOKEN)
                .header("if-none-match", &etag)
                .send()
                .await?
                .status(),
            StatusCode::NOT_MODIFIED
        );
        let invalid = format!("{url}&expires={now}");
        assert_eq!(
            client
                .get(invalid)
                .header(ORIGIN_TOKEN_HEADER, TOKEN)
                .send()
                .await?
                .status(),
            StatusCode::FORBIDDEN
        );
        let expired = sign(site, "/assets/asset.bin", 1, now - 2)?;
        assert_eq!(
            client
                .get(format!("{base}{expired}"))
                .header(ORIGIN_TOKEN_HEADER, TOKEN)
                .send()
                .await?
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            client
                .get(format!("{base}/denied/asset.bin"))
                .header("x-forwarded-for", "192.0.2.1")
                .header("x-real-ip", "192.0.2.1")
                .send()
                .await?
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    runner.abort();
    Ok(())
}

#[tokio::test]
async fn integration_deep_static_directory_and_cdn_cache() -> Result<()> {
    let root = TestDir::new("static-origin-directory");
    let public = root.join("public");
    std::fs::create_dir_all(public.join("folder"))?;
    std::fs::write(public.join("asset.bin"), b"0123456789abcdef")?;
    std::fs::File::options()
        .write(true)
        .open(public.join("asset.bin"))?
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(120))?;
    std::fs::write(public.join("recent.bin"), b"recent-file")?;
    std::fs::write(public.join(".env"), b"never-expose")?;
    std::fs::write(public.join("a #&中.txt"), b"special-name")?;
    let port = reserve_port().await?;
    let folder = public.display().to_string().replace('\\', "/");
    let yaml = format!(
        r#"{}
services:
  static_sites:
    - name: directory
      path_prefix: /files
      root: '{folder}'
      autoindex: true
    - name: cdn
      path_prefix: /cdn
      root: '{folder}'
      cache_control: public, max-age=120
    - name: limited
      path_prefix: /limited
      root: '{folder}'
      rate_limit:
        enabled: true
        zone: static-test
        requests: 1
        window_ms: 60000
        burst: 0
"#,
        base_gateway_yaml(port)
    );
    let config_path = write_config(&root, &yaml)?;
    let (_gateway, runner) = spawn_gateway(config_path).await?;
    let base = format!("http://127.0.0.1:{port}");
    let client = Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let redirect = client.get(format!("{base}/files?x=1")).send().await?;
    assert_eq!(redirect.status(), StatusCode::MOVED_PERMANENTLY);
    assert_eq!(redirect.headers()["location"], "/files/?x=1");
    let listing = client.get(format!("{base}/files/")).send().await?;
    assert_eq!(listing.status(), StatusCode::OK);
    assert!(listing.headers().contains_key("content-security-policy"));
    let body = listing.text().await?;
    assert!(body.contains("/files/folder/"));
    assert!(body.contains("a%20%23%26%E4%B8%AD.txt"));
    assert!(!body.contains(".env"));
    assert!(!body.contains("../"));
    assert_eq!(
        client
            .get(format!("{base}/files/a%20%23%26%E4%B8%AD.txt"))
            .send()
            .await?
            .text()
            .await?,
        "special-name"
    );
    assert_eq!(
        client
            .get(format!("{base}/files/.env"))
            .send()
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        client
            .get(format!("{base}/files/%2eenv"))
            .send()
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        client.get(format!("{base}/cdn/")).send().await?.status(),
        StatusCode::FORBIDDEN
    );
    let url = format!("{base}/cdn/asset.bin");
    let response = client.get(&url).send().await?;
    assert_eq!(response.headers()["cache-control"], "public, max-age=120");
    let etag = response.headers()["etag"].to_str()?.to_string();
    let modified = response.headers()["last-modified"].to_str()?.to_string();
    assert_eq!(
        client
            .get(&url)
            .header("if-none-match", &etag)
            .send()
            .await?
            .status(),
        StatusCode::NOT_MODIFIED
    );
    assert_eq!(
        client
            .get(&url)
            .header("if-modified-since", &modified)
            .send()
            .await?
            .status(),
        StatusCode::NOT_MODIFIED
    );
    let response = client
        .get(&url)
        .header("range", "bytes=1-2")
        .header("if-range", "\"stale\"")
        .send()
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.bytes().await?.len(), 16);
    for validator in [
        etag,
        httpdate::fmt_http_date(std::time::SystemTime::now() + std::time::Duration::from_secs(60)),
    ] {
        let response = client
            .get(&url)
            .header("range", "bytes=1-2")
            .header("if-range", validator)
            .send()
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.bytes().await?.len(), 16);
    }
    assert_eq!(
        client
            .get(&url)
            .header("range", "bytes=1-2")
            .header("if-range", modified)
            .send()
            .await?
            .status(),
        StatusCode::PARTIAL_CONTENT
    );
    let recent_url = format!("{base}/cdn/recent.bin");
    let recent = client.get(&recent_url).send().await?;
    let recent_modified = recent.headers()["last-modified"].to_str()?;
    assert_eq!(
        client
            .get(&recent_url)
            .header("range", "bytes=1-2")
            .header("if-range", recent_modified)
            .send()
            .await?
            .status(),
        StatusCode::OK
    );
    let limited = format!("{base}/limited/asset.bin");
    assert_eq!(client.get(&limited).send().await?.status(), StatusCode::OK);
    assert_eq!(
        client.get(&limited).send().await?.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    runner.abort();
    Ok(())
}
