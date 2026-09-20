use anyhow::Result;
use bytes::Buf;
use reqwest::{Client, StatusCode};

use crate::test_support::TestDir;
use crate::verify::harness::{base_gateway_yaml, reserve_port, spawn_gateway, write_config};

#[tokio::test]
async fn integration_deep_static_virtual_hosts_isolate_warmed_files_and_ranges() -> Result<()> {
    let root = TestDir::new("static-virtual-hosts");
    let a = root.join("a");
    let b = root.join("b");
    for (dir, body) in [(&a, "alpha-content"), (&b, "bravo-resource")] {
        std::fs::create_dir_all(dir)?;
        std::fs::write(dir.join("asset.txt"), body)?;
        std::fs::create_dir_all(dir.join("cold"))?;
        std::fs::write(dir.join("cold/asset.txt"), body)?;
    }
    let port = reserve_port().await?;
    let tls_port = reserve_port().await?;
    let base =
        base_gateway_yaml(port).replace("tls_bind: ''", &format!("tls_bind: 127.0.0.1:{tls_port}"));
    let yaml = format!(
        "{base}\nservices:\n  static_sites:\n    - name: alpha\n      hosts: [alpha.example.com]\n      path_prefix: /assets\n      root: '{}'\n    - name: bravo\n      hosts: [bravo.example.com, '*.cdn.example.com']\n      path_prefix: /assets\n      root: '{}'\n",
        a.display().to_string().replace('\\', "/"),
        b.display().to_string().replace('\\', "/")
    );
    let (_gateway, runner) = spawn_gateway(write_config(&root, &yaml)?).await?;
    let h1 = Client::builder()
        .http1_only()
        .danger_accept_invalid_certs(true)
        .build()?;
    let h2 = Client::builder()
        .http2_prior_knowledge()
        .danger_accept_invalid_certs(true)
        .build()?;
    for (client, scheme, listen) in [
        (&h1, "http", port),
        (&h2, "http", port),
        (&h1, "https", tls_port),
        (&h2, "https", tls_port),
    ] {
        for path in ["asset.txt", "cold/asset.txt"] {
            let url = format!("{scheme}://127.0.0.1:{listen}/assets/{path}");
            for _ in 0..3 {
                for (host, expected) in [
                    ("alpha.example.com", "alpha-content"),
                    ("bravo.example.com", "bravo-resource"),
                    ("ALPHA.EXAMPLE.COM:443", "alpha-content"),
                    ("edge.cdn.example.com", "bravo-resource"),
                ] {
                    let response = client.get(&url).header("host", host).send().await?;
                    assert_eq!(response.status(), StatusCode::OK, "{scheme} {host} {path}");
                    assert_eq!(response.text().await?, expected, "{scheme} {host} {path}");
                }
            }
            for (host, expected) in [
                ("alpha.example.com", "alpha-content"),
                ("bravo.example.com", "bravo-resource"),
            ] {
                let response = client
                    .get(&url)
                    .header("host", host)
                    .header("range", "bytes=0-4")
                    .send()
                    .await?;
                assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
                assert_eq!(response.text().await?, &expected[..5]);
            }
            for host in [
                "unknown.example.com",
                "cdn.example.com",
                "edge.cdn.example.com.attacker.test",
            ] {
                let response = client.get(&url).header("host", host).send().await?;
                assert_eq!(response.status(), StatusCode::NOT_FOUND, "{scheme} {host}");
            }
        }
    }
    runner.abort();
    Ok(())
}

#[test]
fn static_virtual_host_example_and_invalid_patterns_are_checked() {
    let config: crate::config::GatewayConfig = serde_yaml::from_str(include_str!(
        "../../../examples/static-virtual-hosts.example.yaml"
    ))
    .unwrap();
    config.validate().unwrap();
    assert_eq!(config.services.static_sites.len(), 2);
    assert_eq!(
        config.services.static_sites[1].hosts[1],
        "*.game-b.example.com"
    );
    let mut invalid = config;
    invalid.services.static_sites[0].hosts = vec!["https://assets.example.com:443/path".into()];
    assert!(invalid
        .validate()
        .unwrap_err()
        .to_string()
        .contains("invalid host pattern"));
}

#[tokio::test]
async fn integration_deep_static_virtual_hosts_http3_authority_and_ranges() -> Result<()> {
    let root = TestDir::new("static-host-http3");
    let a = root.join("a");
    let b = root.join("b");
    for (dir, body) in [(&a, "alpha-resource"), (&b, "bravo-content")] {
        std::fs::create_dir_all(dir)?;
        std::fs::write(dir.join("asset.txt"), body)?;
    }
    let port = reserve_port().await?;
    let h3_port = reserve_port().await?;
    let base =
        base_gateway_yaml(port).replace("h3_bind: ''", &format!("h3_bind: 127.0.0.1:{h3_port}"));
    let yaml = format!("{base}\nservices:\n  static_sites:\n    - name: alpha\n      hosts: [alpha.example.com]\n      path_prefix: /assets\n      root: '{}'\n    - name: bravo\n      hosts: [bravo.example.com]\n      path_prefix: /assets\n      root: '{}'\n", a.display().to_string().replace('\\', "/"), b.display().to_string().replace('\\', "/"));
    let path = write_config(&root, &yaml)?;
    let (_gateway, runner) = spawn_gateway(path.clone()).await?;
    let config = crate::config::GatewayConfig::load(&path)?;
    let certificate = std::fs::read(&config.http.tls.cert_path)?;
    let mut roots = rustls::RootCertStore::empty();
    for certificate in rustls_pemfile::certs(&mut std::io::Cursor::new(certificate)) {
        roots.add(certificate?)?;
    }
    let mut tls = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    tls.alpn_protocols = vec![b"h3".to_vec()];
    let mut endpoint = quinn::Endpoint::client("127.0.0.1:0".parse()?)?;
    endpoint.set_default_client_config(quinn::ClientConfig::new(std::sync::Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls)?,
    )));
    let connection = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        endpoint.connect(format!("127.0.0.1:{h3_port}").parse()?, "localhost")?,
    )
    .await??;
    let (mut driver, mut requests) = h3::client::new(h3_quinn::Connection::new(connection)).await?;
    let driver_task = tokio::spawn(async move { driver.wait_idle().await });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        for _ in 0..2 {
            for (host, expected) in [
                ("alpha.example.com", "alpha-resource"),
                ("bravo.example.com", "bravo-content"),
            ] {
                for range in [false, true] {
                    let mut request =
                        http::Request::get(format!("https://{host}/assets/asset.txt"));
                    if range {
                        request = request.header("range", "bytes=0-4");
                    }
                    let mut stream = requests.send_request(request.body(())?).await?;
                    stream.finish().await?;
                    let response = stream.recv_response().await?;
                    assert_eq!(
                        response.status(),
                        if range {
                            StatusCode::PARTIAL_CONTENT
                        } else {
                            StatusCode::OK
                        }
                    );
                    let mut body = Vec::new();
                    while let Some(mut chunk) = stream.recv_data().await? {
                        body.extend_from_slice(&chunk.copy_to_bytes(chunk.remaining()));
                    }
                    assert_eq!(
                        body,
                        if range {
                            &expected.as_bytes()[..5]
                        } else {
                            expected.as_bytes()
                        }
                    );
                }
            }
        }
        Ok::<_, anyhow::Error>(())
    })
    .await??;
    endpoint.close(0_u32.into(), b"test finished");
    driver_task.abort();
    runner.abort();
    Ok(())
}

#[tokio::test]
async fn integration_deep_static_virtual_hosts_reload_revokes_old_host() -> Result<()> {
    let root = TestDir::new("static-host-reload");
    let public = root.join("public");
    std::fs::create_dir_all(&public)?;
    std::fs::write(public.join("asset.txt"), "private-resource")?;
    let port = reserve_port().await?;
    let admin_port = reserve_port().await?;
    let base = base_gateway_yaml(port).replace("admin:\n  enabled: false", &format!("admin:\n  enabled: true\n  bind: 127.0.0.1:{admin_port}\n  enable_write_ops: true\n  bearer_token: test-static-host-reload-token"));
    let yaml = format!("{base}\nservices:\n  static_sites:\n    - name: private\n      hosts: [old.example.com]\n      path_prefix: /assets\n      root: '{}'\n", public.display().to_string().replace('\\', "/"));
    let path = write_config(&root, &yaml)?;
    let (_gateway, runner) = spawn_gateway(path.clone()).await?;
    let client = Client::builder().http1_only().build()?;
    let url = format!("http://127.0.0.1:{port}/assets/asset.txt");
    assert_eq!(
        client
            .get(&url)
            .header("host", "old.example.com")
            .send()
            .await?
            .text()
            .await?,
        "private-resource"
    );
    std::fs::write(path, yaml.replace("old.example.com", "new.example.com"))?;
    let reload = client
        .post(format!("http://127.0.0.1:{admin_port}/v1/reload"))
        .bearer_auth("test-static-host-reload-token")
        .send()
        .await?;
    assert_eq!(reload.status(), StatusCode::OK);
    for _ in 0..3 {
        let old = client
            .get(&url)
            .header("host", "old.example.com")
            .send()
            .await?;
        assert_eq!(old.status(), StatusCode::NOT_FOUND);
        let _ = old.bytes().await?;
        assert_eq!(
            client
                .get(&url)
                .header("host", "new.example.com")
                .send()
                .await?
                .text()
                .await?,
            "private-resource"
        );
    }
    runner.abort();
    Ok(())
}

#[tokio::test]
async fn integration_deep_static_virtual_hosts_sendfile_and_yaml_precedence() -> Result<()> {
    let root = TestDir::new("static-host-sendfile");
    let a = root.join("a");
    let b = root.join("b");
    std::fs::create_dir_all(&a)?;
    std::fs::create_dir_all(&b)?;
    for (path, byte) in [(&a, b'a'), (&b, b'b')] {
        std::fs::write(path.join("large.bin"), vec![byte; 9 * 1024 * 1024])?;
    }
    std::fs::write(b.join("only-second.txt"), "must-not-shadow-first")?;
    let port = reserve_port().await?;
    let base =
        base_gateway_yaml(port).replace("traffic_profile: small", "traffic_profile: balanced");
    let yaml = format!("{base}\nservices:\n  static_sites:\n    - name: first\n      hosts: [first.example.com, overlap.example.com]\n      path_prefix: /assets\n      root: '{}'\n    - name: second\n      hosts: [second.example.com, overlap.example.com]\n      path_prefix: /assets\n      root: '{}'\n", a.display().to_string().replace('\\', "/"), b.display().to_string().replace('\\', "/"));
    let (_gateway, runner) = spawn_gateway(write_config(&root, &yaml)?).await?;
    let client = Client::builder().http1_only().build()?;
    for _ in 0..2 {
        for (host, byte) in [
            ("first.example.com", b'a'),
            ("second.example.com", b'b'),
            ("overlap.example.com", b'a'),
        ] {
            let body = client
                .get(format!("http://127.0.0.1:{port}/assets/large.bin"))
                .header("host", host)
                .send()
                .await?
                .bytes()
                .await?;
            assert_eq!(body.len(), 9 * 1024 * 1024);
            assert!(
                body.iter().all(|value| *value == byte),
                "sendfile crossed site: {host}"
            );
        }
    }
    assert_eq!(
        client
            .get(format!("http://127.0.0.1:{port}/assets/only-second.txt"))
            .header("host", "overlap.example.com")
            .send()
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    runner.abort();
    Ok(())
}
