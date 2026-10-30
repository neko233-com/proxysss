use super::*;

#[test]
fn synthetic_status_body_and_conditional_response_headers_are_real() {
    let config = GatewayConfig::default();
    let mut route: RouteDecision = serde_json::from_value(serde_json::json!({
        "upstream":"proxysss://response", "status":403, "response_body":"denied",
        "response_headers": {
            "X-Always":"yes", "X-Success":{"value":"yes","statuses":[200,204]},
            "X-Cache-Copy":{"from_header":"x-cache"}
        }
    }))
    .unwrap();
    let mut response = dispatch_internal_http(&config, &route);
    response.headers.push((
        HeaderName::from_static("x-cache"),
        HeaderValue::from_static("HIT"),
    ));
    apply_route_response_headers(&route, &mut response).unwrap();
    assert_eq!(response.status, StatusCode::FORBIDDEN);
    assert_eq!(response.body, Bytes::from_static(b"denied"));
    assert!(response
        .headers
        .iter()
        .any(|(name, value)| name == "x-always" && value == "yes"));
    assert!(!response.headers.iter().any(|(name, _)| name == "x-success"));
    assert!(response
        .headers
        .iter()
        .any(|(name, value)| name == "x-cache-copy" && value == "HIT"));
    route.status = Some(204);
    assert!(dispatch_internal_http(&config, &route).body.is_empty());
    route.status = Some(101);
    assert_eq!(
        dispatch_internal_http(&config, &route).status,
        StatusCode::BAD_GATEWAY
    );
    route.response_headers.insert(
        "Content-Length".into(),
        crate::script::ResponseHeader::Always("1000".into()),
    );
    assert!(route.validate_http_policy().is_err());
}

#[test]
fn yaml_authorization_is_not_eligible_for_plain_or_websocket_bypass() {
    let mut config = GatewayConfig::default();
    let policy: crate::auth_request::AuthRequestConfig =
        serde_json::from_value(serde_json::json!({"url":"http://127.0.0.1:9/auth"})).unwrap();
    let route = ReverseProxyRouteConfig {
        auth_request: Some(policy.clone()),
        ..Default::default()
    };
    assert!(!reverse_proxy_route_fast_path_eligible(&route));
    assert!(!websocket_route_fast_path_eligible(&route));
    config.services.reverse_proxy.routes.push(route);
    assert!(!simple_http_proxy_fast_path_allowed(&config));
    let decision =
        configured_reverse_proxy_route(&config, "synthetic.example", &"/".parse().unwrap())
            .unwrap();
    assert!(decision.decision.auth_request.is_some());
}

#[tokio::test]
async fn invalid_certificate_rollover_retains_validated_state_and_listener_keys() {
    let root = crate::verify::harness::temp_root("proxysss-auth-tls-rollover");
    std::fs::create_dir_all(&root).unwrap();
    let mut config = GatewayConfig {
        root_dir: root.clone(),
        ..Default::default()
    };
    config.runtime.performance.enabled = false;
    config.http.plain_bind = "127.0.0.1:0".into();
    config.http.tls_bind = "127.0.0.1:0".into();
    config.http.h3_bind.clear();
    config.admin.enabled = false;
    config.http.tls.cert_path = root.join("default.pem");
    config.http.tls.key_path = root.join("default.key");
    let cert = root.join("sni.pem");
    let key = root.join("sni.key");
    crate::install::ensure_cert_pair(&cert, &key, "synthetic.example", true).unwrap();
    config.http.tls.certificates = vec![TlsCertificateConfig {
        domains: vec!["synthetic.example".into()],
        cert_path: cert.clone(),
        key_path: key.clone(),
    }];
    config.http.tls.reject_unknown_sni = true;
    let config_path = root.join("proxysss.yaml");
    std::fs::write(&config_path, serde_yaml::to_string(&config).unwrap()).unwrap();
    let gateway = Gateway::from_config(config_path.clone(), config)
        .await
        .unwrap();
    let original = gateway.current_state().await.tls_fingerprint.clone();
    let original_reload = reload_fingerprint(&config_path).unwrap();
    let next_cert = root.join("next.pem");
    let next_key = root.join("next.key");
    crate::install::ensure_cert_pair(&next_cert, &next_key, "synthetic.example", true).unwrap();
    std::fs::copy(&next_cert, &cert).unwrap();
    assert_ne!(reload_fingerprint(&config_path).unwrap(), original_reload);
    assert!(gateway.reload_from_disk().await.is_err());
    let state = gateway.current_state().await;
    assert_eq!(state.tls_fingerprint, original);
    let old_keys = listener_specs_with_tls_fingerprint(&state.config, &state.tls_fingerprint)
        .iter()
        .map(ListenerSpec::key)
        .collect::<Vec<_>>();
    assert!(old_keys.iter().any(|key| key.contains(&original)));
    drop(state);
    std::fs::copy(&next_key, &key).unwrap();
    gateway.reload_from_disk().await.unwrap();
    assert_ne!(gateway.current_state().await.tls_fingerprint, original);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn route_timeout_covers_upstream_headers_and_body_idle_without_changing_global() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let root = crate::verify::harness::temp_root("proxysss-route-timeout");
    std::fs::create_dir_all(&root).unwrap();
    let mut config = GatewayConfig::default();
    config.http.request_timeout_ms = 500;
    config.http.tls.cert_path = root.join("cert.pem");
    config.http.tls.key_path = root.join("key.pem");
    config.script.enabled = false;
    config.plugins.enabled = false;
    config.runtime.performance.enabled = false;
    let gateway = Gateway::from_config(root.join("unused.yaml"), config)
        .await
        .unwrap();
    let state = gateway.current_state().await;
    for (head_delay, body_delay) in [(80, 0), (0, 80), (0, 0)] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let uri: Uri = format!("http://{}/health", listener.local_addr().unwrap())
            .parse()
            .unwrap();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 4096];
            let _ = stream.read(&mut request).await;
            tokio::time::sleep(Duration::from_millis(head_delay)).await;
            let _ = stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\na")
                .await;
            tokio::time::sleep(Duration::from_millis(body_delay)).await;
            let _ = stream.write_all(b"b").await;
        });
        let result = state
            .dispatch_upstream_http(
                Method::GET,
                UpstreamHttpTarget::Hyper(uri),
                HeaderMap::new(),
                Bytes::new(),
                Some(20),
            )
            .await;
        if head_delay > 0 {
            assert!(result.is_err());
        } else if body_delay > 0 {
            assert!(result.unwrap().bytes().await.is_err());
        } else {
            assert_eq!(
                result.unwrap().bytes().await.unwrap(),
                Bytes::from_static(b"ab")
            );
        }
        task.await.unwrap();
    }
    assert_eq!(state.config.http.request_timeout_ms, 500);
    let invalid: RouteDecision = serde_json::from_value(
        serde_json::json!({"upstream":"http://127.0.0.1:9", "request_timeout_ms":0}),
    )
    .unwrap();
    assert!(invalid.validate_http_policy().is_err());
    drop(state);
    drop(gateway);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn websocket_handshake_respects_explicit_host_forwarding_and_request_flags() {
    let mut headers = HeaderMap::new();
    headers.insert(HOST, HeaderValue::from_static("visitor.example"));
    headers.insert(
        http::header::AUTHORIZATION,
        HeaderValue::from_static("Bearer synthetic-fixture"),
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let mut route = RouteDecision {
        pass_request_headers: false,
        pass_request_body: false,
        ..Default::default()
    };
    route
        .set_headers
        .insert("Host".into(), "chosen.example".into());
    route
        .set_headers
        .insert("X-Forwarded-Proto".into(), "https".into());
    route
        .set_headers
        .insert("Upgrade".into(), "websocket".into());
    let actual = build_websocket_upstream_headers(
        &headers,
        &route,
        "upstream.example",
        "127.0.0.1:1234".parse().unwrap(),
        "http",
        "visitor.example",
        true,
    )
    .unwrap();
    assert_eq!(actual[HOST], "chosen.example");
    assert_eq!(actual["x-forwarded-proto"], "https");
    assert_eq!(actual[http::header::UPGRADE], "websocket");
    assert!(!actual.contains_key(http::header::AUTHORIZATION));
    assert!(!actual.contains_key(CONTENT_TYPE));
}

#[test]
fn cookie_secure_preserves_independent_cookies_and_other_attributes() {
    let route = RouteDecision {
        cookie_secure: true,
        ..Default::default()
    };
    let mut response = GatewayHttpResponse::error(StatusCode::OK, "fixture");
    for cookie in [
        "a=one; Path=/; HttpOnly; SameSite=Lax",
        "b=two; Expires=Wed, 21 Oct 2030 07:28:00 GMT; sEcUrE",
        "c=three; SameSite=None",
    ] {
        response.headers.push((
            http::header::SET_COOKIE,
            HeaderValue::from_str(cookie).unwrap(),
        ));
    }
    apply_route_response_headers(&route, &mut response).unwrap();
    apply_route_response_headers(&route, &mut response).unwrap();
    let cookies: Vec<_> = response
        .headers
        .iter()
        .filter(|(name, _)| name == http::header::SET_COOKIE)
        .map(|(_, value)| value.to_str().unwrap())
        .collect();
    assert_eq!(
        cookies,
        [
            "a=one; Path=/; HttpOnly; SameSite=Lax; Secure",
            "b=two; Expires=Wed, 21 Oct 2030 07:28:00 GMT; sEcUrE",
            "c=three; SameSite=None; Secure"
        ]
    );
}

#[tokio::test]
async fn aggregate_body_budget_denies_before_copy_and_refunds_errors_timeout_and_cancel() {
    use crate::body_budget::BufferedBodyBudget;
    use http_body_util::StreamBody;
    use hyper::body::Frame;
    let budget = BufferedBodyBudget::new(5);
    let mut full = Full::new(Bytes::from_static(b"1234"));
    let body = collect_request_body_with_budget(
        &Method::POST,
        &HeaderMap::new(),
        &mut full,
        100,
        Duration::from_secs(1),
        &budget,
    )
    .await
    .unwrap();
    assert_eq!(budget.used(), 4);
    let clone = body.clone();
    drop(body);
    let mut second = Full::new(Bytes::from_static(b"56"));
    assert_eq!(
        collect_request_body_with_budget(
            &Method::POST,
            &HeaderMap::new(),
            &mut second,
            100,
            Duration::from_secs(1),
            &budget
        )
        .await
        .unwrap_err()
        .status,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(budget.used(), 4);
    drop(clone);
    assert_eq!(budget.used(), 0);
    let mut invalid = Full::new(Bytes::from_static(b"123"));
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_LENGTH, HeaderValue::from_static("4"));
    assert!(collect_request_body_with_budget(
        &Method::POST,
        &headers,
        &mut invalid,
        100,
        Duration::from_secs(1),
        &budget
    )
    .await
    .is_err());
    assert_eq!(budget.used(), 0);
    for cancel in [false, true] {
        let chunks = futures::stream::iter([Ok::<_, anyhow::Error>(Frame::data(
            Bytes::from_static(b"123"),
        ))])
        .chain(futures::stream::pending());
        let mut pending = StreamBody::new(chunks);
        let headers = HeaderMap::new();
        let collect = collect_request_body_with_budget(
            &Method::POST,
            &headers,
            &mut pending,
            100,
            Duration::from_millis(if cancel { 1000 } else { 10 }),
            &budget,
        );
        if cancel {
            assert!(tokio::time::timeout(Duration::from_millis(10), collect)
                .await
                .is_err());
        } else {
            assert_eq!(
                collect.await.unwrap_err().status,
                StatusCode::REQUEST_TIMEOUT
            );
        }
        assert_eq!(budget.used(), 0);
    }
    let mut after = Full::new(Bytes::from_static(b"12345"));
    let after = collect_request_body_with_budget(
        &Method::POST,
        &HeaderMap::new(),
        &mut after,
        100,
        Duration::from_secs(1),
        &budget,
    )
    .await
    .unwrap();
    assert_eq!(budget.used(), 5);
    drop(after);
    assert_eq!(budget.used(), 0);
}

#[test]
fn http_upgrade_switch_is_strict_default_true_and_roundtrips_false() {
    let default: GatewayConfig = serde_yaml::from_str("http: {}\n").unwrap();
    assert!(default.http.redirect_http_to_https);
    let explicit: GatewayConfig =
        serde_yaml::from_str("http:\n  redirect_http_to_https: false\n").unwrap();
    assert!(!explicit.http.redirect_http_to_https);
    let roundtrip: GatewayConfig =
        serde_yaml::from_str(&serde_yaml::to_string(&explicit).unwrap()).unwrap();
    assert!(!roundtrip.http.redirect_http_to_https);
    assert!(serde_yaml::from_str::<GatewayConfig>(
        "http:\n  redirect_http_to_https: false\n  redirect_http_to_https_typo: false\n"
    )
    .is_err());
}

#[tokio::test]
async fn explicit_http_upgrade_disable_reaches_script_without_changing_tls_sni() {
    let root = crate::test_support::TestDir::new("http-upgrade-script");
    let script = root.join("gateway.ts");
    std::fs::write(
        &script,
        r#"export default {
      name: "gateway",
      access(message) {
        const ctx = message.ctx;
        if (ctx.path === "/redirect") return { upstream:"proxysss://response", status:302,
          response_headers:{ Location:"/chosen?" + ctx.query } };
        return { upstream:"proxysss://response", status:201,
          response_body:"script:" + ctx.scheme + ":" + ctx.path + ":" + ctx.query };
      }
    };"#,
    )
    .unwrap();
    for enabled in [true, false] {
        let mut config = GatewayConfig {
            root_dir: root.to_path_buf(),
            ..Default::default()
        };
        config.http.tls.cert_path = root.join("cert.pem");
        config.http.tls.key_path = root.join("key.pem");
        config.http.tls.certificates = vec![TlsCertificateConfig {
            domains: vec!["synthetic.example".into()],
            cert_path: config.http.tls.cert_path.clone(),
            key_path: config.http.tls.key_path.clone(),
        }];
        config.http.tls.reject_unknown_sni = true;
        config.http.redirect_http_to_https = enabled;
        config.http.h3_bind.clear();
        config.script.enabled = true;
        config.script.entry = script.clone();
        config.script.cwd = Some(root.to_path_buf());
        config.plugins.enabled = false;
        config.admin.enabled = false;
        config.monitoring.enabled = false;
        config.runtime.performance.enabled = false;
        let gateway = Gateway::from_config(root.join("unused.yaml"), config)
            .await
            .unwrap();
        assert_eq!(
            http_to_https_redirect_can_apply(&gateway.bootstrap_config),
            enabled
        );
        assert!(
            gateway
                .current_state()
                .await
                .config
                .http
                .tls
                .reject_unknown_sni
        );
        for path in ["/plain?foo=bar", "/redirect?foo=bar"] {
            let mut headers = HeaderMap::new();
            headers.insert(HOST, HeaderValue::from_static("synthetic.example"));
            let response = gateway
                .dispatch_http(
                    Method::GET,
                    path.parse().unwrap(),
                    headers,
                    Bytes::new(),
                    "127.0.0.1:1234".parse().unwrap(),
                    "http",
                    "HTTP/1.1",
                    None,
                )
                .await
                .unwrap();
            if enabled {
                assert_eq!(response.status, StatusCode::PERMANENT_REDIRECT);
                assert!(response
                    .headers
                    .iter()
                    .any(|(name, value)| name == http::header::LOCATION
                        && value.to_str().unwrap() == format!("https://synthetic.example{path}")));
            } else if path.starts_with("/plain") {
                assert_eq!(response.status, StatusCode::CREATED);
                assert_eq!(
                    response.body,
                    Bytes::from_static(b"script:http:/plain:foo=bar")
                );
            } else {
                assert_eq!(response.status, StatusCode::FOUND);
                assert!(response
                    .headers
                    .iter()
                    .any(|(name, value)| name == http::header::LOCATION
                        && value == "/chosen?foo=bar"));
            }
        }
    }
}

#[tokio::test]
async fn actual_http_error_pages_disabled_and_explicit_script_static_bodies_are_preserved() {
    let root = crate::test_support::TestDir::new("explicit-error-http");
    std::fs::write(root.join("50x.html"), "fixture static error body").unwrap();
    let script = root.join("gateway.ts");
    std::fs::write(&script, r#"export default { name:"gateway", access(message) {
      if (message.ctx.path === "/static-error") return { upstream:"proxysss://static-site/errors", rewrite_path:"/50x.html", status:500 };
      return { upstream:"proxysss://response", status:403, response_body:"fixture synthetic body", response_headers:{"X-Always":"kept"} };
    }};"#).unwrap();
    for enabled in [false, true] {
        let mut config = GatewayConfig {
            root_dir: root.to_path_buf(),
            ..Default::default()
        };
        config.http.tls.cert_path = root.join("cert.pem");
        config.http.tls.key_path = root.join("key.pem");
        config.http.tls_bind.clear();
        config.http.h3_bind.clear();
        config.http.error_pages.enabled = enabled;
        config.services.static_sites = vec![StaticSiteConfig {
            name: "errors".into(),
            root: root.to_path_buf(),
            path_prefix: "/".into(),
            script_only: true,
            ..Default::default()
        }];
        config.script.enabled = true;
        config.script.entry = script.clone();
        config.script.cwd = Some(root.to_path_buf());
        config.plugins.enabled = false;
        config.admin.enabled = false;
        config.monitoring.enabled = false;
        config.runtime.performance.enabled = false;
        let gateway = Gateway::from_config(root.join("unused.yaml"), config)
            .await
            .unwrap();
        // Exercise the real Hyper request/finish path with wildcard and HTML Accept clients.
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            loop {
                let (stream, peer) = listener.accept().await.unwrap();
                let gateway = gateway.clone();
                tokio::spawn(async move {
                    gateway
                        .serve_plain_hyper_connection(stream, Bytes::new(), peer, 0)
                        .await;
                });
            }
        });
        let client = reqwest::Client::builder().no_proxy().build().unwrap();
        for accept in [None, Some("text/html")] {
            for (path, status, expected) in [
                ("/deny", 403, "fixture synthetic body"),
                ("/static-error", 500, "fixture static error body"),
            ] {
                let mut request = client.get(format!("http://{address}{path}"));
                if let Some(accept) = accept {
                    request = request.header("Accept", accept);
                }
                let response = request.send().await.unwrap();
                assert_eq!(response.status().as_u16(), status);
                if path == "/deny" {
                    assert_eq!(response.headers()["x-always"], "kept");
                }
                assert_eq!(response.text().await.unwrap(), expected);
            }
        }
        task.abort();
        let _ = task.await;
    }
    let mut config = GatewayConfig::default();
    config.http.error_pages.enabled = false;
    config.http.error_pages.pages = vec![crate::config::HttpErrorPageConfig {
        status: 403,
        body: "configured replacement".into(),
        ..Default::default()
    }];
    let original = GatewayHttpResponse::error(StatusCode::FORBIDDEN, "original native error");
    assert_eq!(
        decorate_error_response(&config, &HeaderMap::new(), original).body,
        Bytes::from_static(b"original native error")
    );
    config.http.error_pages.enabled = true;
    let original = GatewayHttpResponse::error(StatusCode::FORBIDDEN, "original native error");
    assert_eq!(
        decorate_error_response(&config, &HeaderMap::new(), original).body,
        Bytes::from_static(b"configured replacement")
    );
}
