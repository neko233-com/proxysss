use super::*;
use crate::test_support::TestDir;

async fn static_response(
    site: &StaticSiteConfig,
    method: Method,
    path: &str,
    headers: HeaderMap,
) -> GatewayHttpResponse {
    dispatch_static_site(
        site,
        &method,
        &path.parse().unwrap(),
        &headers,
        &DashMap::new(),
        &AtomicU64::new(0),
        &DashMap::new(),
    )
    .await
    .unwrap()
}

#[test]
fn script_only_sites_never_match_native_or_h2_cache_keys() {
    let protected = StaticSiteConfig {
        script_only: true,
        path_prefix: "/".into(),
        ..Default::default()
    };
    let mut config = GatewayConfig::default();
    config.runtime.hot_reload.enabled = false;
    config.admin.enabled = false;
    config.services.static_sites = vec![protected.clone()];
    assert!(!static_site_matches(
        &protected,
        "fixture.example.com",
        "/index.html"
    ));
    assert!(static_route_cache_key(&config, "fixture.example.com", "/index.html").is_none());
    assert!(!plain_static_fast_path_allowed(&config));
    assert!(!hyper_static_success_fast_path_globally_allowed(&config));
    config.services.static_sites.push(StaticSiteConfig {
        name: "native".into(),
        path_prefix: "/assets".into(),
        ..Default::default()
    });
    assert_eq!(
        static_route_cache_key(&config, "fixture.example.com", "/assets/a.txt").unwrap(),
        "1:/assets/a.txt"
    );
}

#[tokio::test]
async fn script_only_preload_cannot_publish_private_files_to_shared_fast_caches() {
    let root = TestDir::new("script-static-preload");
    std::fs::write(root.join("index.html"), "protected fixture").unwrap();
    let site = StaticSiteConfig {
        script_only: true,
        root: root.to_path_buf(),
        path_prefix: "/".into(),
        ..Default::default()
    };
    let files = DashMap::new();
    let routes = DashMap::new();
    let h2 = DashMap::new();
    let loaded = preload_static_site_fast_lane_cache(
        &site,
        Some(0),
        RuntimePerformanceTrafficProfile::Balanced,
        1024,
        &files,
        &AtomicU64::new(0),
        &DashMap::new(),
        &routes,
        &h2,
    )
    .await
    .unwrap();
    assert_eq!(loaded, 0);
    assert!(files.is_empty() && routes.is_empty() && h2.is_empty());
}

#[test]
fn static_script_rewrite_preserves_query_and_rejects_non_origin_paths() {
    let original: Uri = "/old?fixture=one%2Ftwo".parse().unwrap();
    let mut route = RouteDecision {
        rewrite_path: Some("/new".into()),
        ..Default::default()
    };
    assert_eq!(
        script_static_uri(&route, &original).unwrap(),
        "/new?fixture=one%2Ftwo"
    );
    route.rewrite_path = Some("/new?explicit=yes".into());
    assert_eq!(
        script_static_uri(&route, &original).unwrap(),
        "/new?explicit=yes"
    );
    for invalid in ["//other/path", "https://other/path", "/x#fragment"] {
        route.rewrite_path = Some(invalid.into());
        assert!(script_static_uri(&route, &original).is_none());
    }
}

#[tokio::test]
async fn script_static_rewrite_preserves_authority_without_a_host_header() {
    let root = TestDir::new("script-static-h2-authority");
    std::fs::write(root.join("asset.txt"), "authority fixture").unwrap();
    let mut config = GatewayConfig {
        root_dir: root.to_path_buf(),
        ..Default::default()
    };
    config.runtime.performance.enabled = false;
    config.runtime.hot_reload.enabled = false;
    config.runtime.maintenance_state.enabled = false;
    config.admin.enabled = false;
    config.http.plain_bind = "127.0.0.1:0".into();
    config.http.tls_bind = "127.0.0.1:0".into();
    config.http.h3_bind.clear();
    config.http.tls.cert_path = root.join("fixture.pem");
    config.http.tls.key_path = root.join("fixture.key");
    config.services.static_sites = vec![StaticSiteConfig {
        name: "authority".into(),
        script_only: true,
        hosts: vec!["fixture.example.com".into()],
        path_prefix: "/".into(),
        root: root.to_path_buf(),
        ..Default::default()
    }];
    let config_path = root.join("fixture.yaml");
    std::fs::write(&config_path, serde_yaml::to_string(&config).unwrap()).unwrap();
    let gateway = Gateway::from_config(config_path, config).await.unwrap();
    let state = gateway.current_state().await;
    let route = RouteDecision {
        upstream: "proxysss://static-site/authority".into(),
        rewrite_path: Some("/asset.txt".into()),
        ..Default::default()
    };
    let headers = HeaderMap::new();
    for (host, expected) in [
        ("fixture.example.com", StatusCode::OK),
        ("unknown.example.com", StatusCode::NOT_FOUND),
    ] {
        // HTTP/2 carries :authority in the URI and need not send a Host header.
        let uri = format!("https://{host}/original?fixture=yes")
            .parse()
            .unwrap();
        let response = gateway
            .dispatch_script_static_site(
                &state,
                &route,
                &Method::GET,
                &uri,
                &headers,
                "127.0.0.1:1234".parse().unwrap(),
            )
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status, expected);
        if expected == StatusCode::OK {
            assert_eq!(response.body.as_ref(), b"authority fixture");
        }
    }
}

#[tokio::test]
async fn explicit_static_error_pages_ignore_range_and_conditions_but_keep_file_authorization() {
    let root = TestDir::new("script-static-error-page");
    std::fs::write(root.join("50x.html"), "fixture error page").unwrap();
    let mut site = StaticSiteConfig {
        root: root.to_path_buf(),
        path_prefix: "/".into(),
        ..Default::default()
    };
    site.security.origin_token = "fixture-error-page-origin-token-with-thirty-two-bytes".into();
    let mut headers = HeaderMap::new();
    headers.insert(RANGE, HeaderValue::from_static("bytes=1-3"));
    headers.insert(http::header::IF_NONE_MATCH, HeaderValue::from_static("*"));
    headers.insert("if-custom", HeaderValue::from_static("fixture"));
    let without_token = static_response(
        &site,
        Method::GET,
        "/50x.html",
        script_static_error_headers(&headers),
    )
    .await;
    assert_eq!(without_token.status, StatusCode::FORBIDDEN);
    headers.insert(
        crate::static_security::ORIGIN_TOKEN_HEADER,
        HeaderValue::from_str(&site.security.origin_token).unwrap(),
    );
    let filtered = script_static_error_headers(&headers);
    assert!(!filtered.contains_key(RANGE));
    assert!(!filtered.keys().any(|name| name.as_str().starts_with("if-")));
    assert_eq!(
        filtered[crate::static_security::ORIGIN_TOKEN_HEADER],
        site.security.origin_token
    );
    let response = static_response(&site, Method::GET, "/50x.html", filtered.clone()).await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.body.as_ref(), b"fixture error page");
    let missing = static_response(&site, Method::GET, "/missing", filtered).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert!(headers.contains_key(RANGE));
}

#[test]
fn static_script_fields_default_off_and_reject_unsafe_fallback_or_names() {
    let site: StaticSiteConfig = serde_yaml::from_str("name: fixture").unwrap();
    assert!(!site.script_only && site.spa_fallback.is_none());
    let mut config = GatewayConfig::default();
    config.services.static_sites = vec![StaticSiteConfig {
        name: "script-fixture".into(),
        script_only: true,
        spa_fallback: Some("index.html".into()),
        ..Default::default()
    }];
    config.validate().unwrap();
    for fallback in [
        "",
        "..",
        "../index.html",
        "/index.html",
        "nested/index.html",
        "C:index.html",
    ] {
        config.services.static_sites[0].spa_fallback = Some(fallback.into());
        assert!(config
            .validate()
            .unwrap_err()
            .to_string()
            .contains("spa_fallback"));
    }
    config.services.static_sites[0].spa_fallback = None;
    for name in [".", "..", "name/child", "name?query", "name#fragment"] {
        config.services.static_sites[0].name = name.into();
        assert!(config
            .validate()
            .unwrap_err()
            .to_string()
            .contains("URI segments"));
    }
}

#[tokio::test]
async fn spa_fallback_only_resolves_missing_files_and_retains_head_range_and_auth() {
    let root = TestDir::new("script-static-spa");
    std::fs::write(root.join("index.html"), "spa-index-fixture").unwrap();
    std::fs::write(root.join("asset.txt"), "real-asset").unwrap();
    std::fs::create_dir(root.join("empty")).unwrap();
    let mut site = StaticSiteConfig {
        root: root.to_path_buf(),
        path_prefix: "/".into(),
        spa_fallback: Some("index.html".into()),
        ..Default::default()
    };
    let asset = static_response(&site, Method::GET, "/asset.txt", HeaderMap::new()).await;
    assert_eq!(asset.status, StatusCode::OK);
    assert_eq!(asset.body.as_ref(), b"real-asset");
    let fallback = static_response(&site, Method::GET, "/missing/deep", HeaderMap::new()).await;
    assert_eq!(fallback.status, StatusCode::OK);
    assert_eq!(fallback.body.as_ref(), b"spa-index-fixture");
    assert_eq!(
        static_response(&site, Method::GET, "/empty/", HeaderMap::new())
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    for invalid in ["/.hidden-missing", "/%2e%2e/secret"] {
        assert_eq!(
            static_response(&site, Method::GET, invalid, HeaderMap::new())
                .await
                .status,
            StatusCode::NOT_FOUND
        );
    }
    let head = static_response(&site, Method::HEAD, "/missing", HeaderMap::new()).await;
    assert_eq!(head.status, StatusCode::OK);
    assert!(head.body.is_empty());
    assert!(head
        .headers
        .iter()
        .any(|(name, value)| name == CONTENT_LENGTH && value == "17"));
    let mut headers = HeaderMap::new();
    headers.insert(RANGE, HeaderValue::from_static("bytes=1-3"));
    let range = static_response(&site, Method::GET, "/missing", headers).await;
    assert_eq!(range.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(range.body.as_ref(), b"pa-");
    site.security.origin_token = "fixture-origin-token-with-at-least-thirty-two-bytes".into();
    assert_eq!(
        static_response(&site, Method::GET, "/missing", HeaderMap::new())
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        crate::static_security::ORIGIN_TOKEN_HEADER,
        HeaderValue::from_str(&site.security.origin_token).unwrap(),
    );
    assert_eq!(
        static_response(&site, Method::GET, "/missing", headers)
            .await
            .status,
        StatusCode::OK
    );
}

#[cfg(unix)]
#[tokio::test]
async fn spa_fallback_does_not_turn_outside_or_dangling_symlinks_into_success() {
    use std::os::unix::fs::symlink;
    let fixture = TestDir::new("script-static-links");
    let root = fixture.join("root");
    let outside = fixture.join("outside");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(root.join("index.html"), "index").unwrap();
    std::fs::write(outside.join("secret"), "outside").unwrap();
    symlink(&outside, root.join("escape")).unwrap();
    symlink(outside.join("not-created"), root.join("dangling")).unwrap();
    std::fs::create_dir(root.join(".hidden")).unwrap();
    symlink(root.join(".hidden"), root.join("hidden-alias")).unwrap();
    let mut site = StaticSiteConfig {
        root,
        path_prefix: "/".into(),
        spa_fallback: Some("index.html".into()),
        ..Default::default()
    };
    for path in [
        "/escape/secret",
        "/escape/not-created",
        "/dangling",
        "/hidden-alias/not-created",
    ] {
        assert_eq!(
            static_response(&site, Method::GET, path, HeaderMap::new())
                .await
                .status,
            StatusCode::NOT_FOUND
        );
    }
    symlink(
        outside.join("secret"),
        site.root.join("external-index.html"),
    )
    .unwrap();
    site.spa_fallback = Some("external-index.html".into());
    assert_eq!(
        static_response(&site, Method::GET, "/missing", HeaderMap::new())
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[test]
fn explicit_host_and_forwarded_headers_override_generated_defaults() {
    for forward in [false, true] {
        let mut route = RouteDecision::default();
        route
            .set_headers
            .insert("Host".into(), "origin.example.com".into());
        route
            .set_headers
            .insert("X-Forwarded-Proto".into(), "https".into());
        let headers = build_upstream_headers(
            &HeaderMap::new(),
            &route,
            "visitor.example.com",
            "127.0.0.1:1234".parse().unwrap(),
            "http",
            forward,
        )
        .unwrap();
        assert_eq!(headers[HOST], "origin.example.com");
        assert_eq!(headers["x-forwarded-proto"], "https");
    }
}

#[test]
fn script_request_flags_drop_visitor_credentials_and_body_metadata_but_keep_explicit_headers() {
    let mut original = HeaderMap::new();
    for (name, value) in [
        ("authorization", "fixture-only"),
        ("x-visitor", "fixture"),
        ("content-length", "8"),
        ("content-type", "text/plain"),
        ("content-encoding", "gzip"),
        ("expect", "100-continue"),
    ] {
        original.insert(
            HeaderName::from_bytes(name.as_bytes()).unwrap(),
            HeaderValue::from_static(value),
        );
    }
    let mut route = RouteDecision {
        pass_request_headers: false,
        pass_request_body: false,
        ..Default::default()
    };
    route.set_headers.insert("X-Approved".into(), "yes".into());
    let headers = build_upstream_headers(
        &original,
        &route,
        "visitor.example.com",
        "127.0.0.1:1234".parse().unwrap(),
        "http",
        false,
    )
    .unwrap();
    assert_eq!(headers[HOST], "visitor.example.com");
    assert_eq!(headers["x-approved"], "yes");
    for name in [
        "authorization",
        "x-visitor",
        "content-length",
        "content-type",
        "content-encoding",
        "expect",
    ] {
        assert!(!headers.contains_key(name));
    }
    route.pass_request_headers = true;
    let headers = build_upstream_headers(
        &original,
        &route,
        "visitor.example.com",
        "127.0.0.1:1234".parse().unwrap(),
        "http",
        false,
    )
    .unwrap();
    assert_eq!(headers["x-visitor"], "fixture");
    for name in [
        "content-length",
        "content-type",
        "content-encoding",
        "expect",
    ] {
        assert!(!headers.contains_key(name));
    }
}
