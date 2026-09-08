use super::*;
use crate::test_support::TestDir;

#[test]
fn ftp_switches_do_not_erase_or_accidentally_skip_sibling_policies() {
    let mut ftp = crate::config::FtpConfig {
        command_deny: vec!["DELE".into()],
        transfer_deny: vec!["STOR".into()],
        user_policies: vec![FtpUserPolicy {
            user: "reader".into(),
            command_allow: vec![],
            command_deny: vec!["SITE".into()],
            transfer_allow: vec![],
            transfer_deny: vec!["RETR".into()],
        }],
        ..Default::default()
    };
    assert!(!ftp_command_allowed_for_user(&ftp, "SITE", "reader"));
    ftp.user_policy_enabled = false;
    assert!(ftp_command_allowed_for_user(&ftp, "SITE", "reader"));
    assert!(!ftp_command_allowed_for_user(&ftp, "DELE", "reader"));
    ftp.command_policy_enabled = false;
    assert!(ftp_command_allowed_for_user(&ftp, "DELE", "reader"));
    assert!(!ftp_transfer_allowed_for_user(&ftp, "STOR", "reader"));
    ftp.transfer_policy_enabled = false;
    assert!(ftp_transfer_allowed_for_user(&ftp, "STOR", "reader"));
    assert_eq!(ftp.command_deny, ["DELE"]);
}

async fn dispatch(site: &StaticSiteConfig, method: Method, path: &str) -> GatewayHttpResponse {
    dispatch_static_site(
        site,
        &method,
        &path.parse().unwrap(),
        &HeaderMap::new(),
        &DashMap::new(),
        &AtomicU64::new(0),
        &DashMap::new(),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn autoindex_head_limit_and_hidden_paths() {
    let root = TestDir::new("autoindex-bounds");
    std::fs::write(root.join("a.txt"), b"hello").unwrap();
    let mut site = StaticSiteConfig {
        root: root.to_path_buf(),
        path_prefix: "/".into(),
        autoindex: true,
        autoindex_max_entries: 1,
        ..Default::default()
    };
    let get = dispatch(&site, Method::GET, "/").await;
    let head = dispatch(&site, Method::HEAD, "/").await;
    assert_eq!(get.status, StatusCode::OK);
    assert!(head.body.is_empty());
    assert!(head
        .headers
        .iter()
        .any(|(name, value)| name == CONTENT_LENGTH
            && value.to_str().unwrap() == get.body.len().to_string()));
    std::fs::write(root.join("b.txt"), b"second").unwrap();
    assert_eq!(
        dispatch(&site, Method::GET, "/").await.status,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    site.autoindex = false;
    assert_eq!(
        dispatch(&site, Method::GET, "/").await.status,
        StatusCode::FORBIDDEN
    );
    for path in [
        "/.env",
        "/%2eenv",
        "/a%5cb",
        "/a%3astream",
        "/../a",
        "/%2e%2e/a",
        "/a%00",
        "//outside",
    ] {
        assert!(
            static_site_filesystem_path(&site, path).unwrap().is_none(),
            "{path}"
        );
    }
}

#[tokio::test]
async fn preload_excludes_hidden_and_encodes_names() {
    let root = TestDir::new("static-preload-security");
    std::fs::write(root.join(".env"), b"secret").unwrap();
    std::fs::write(root.join("a #.txt"), b"public").unwrap();
    let site = StaticSiteConfig {
        root: root.to_path_buf(),
        path_prefix: "/assets".into(),
        ..Default::default()
    };
    let routes = DashMap::new();
    let h2 = DashMap::new();
    preload_static_site_fast_lane_cache(
        &site,
        RuntimePerformanceTrafficProfile::Small,
        u64::MAX,
        &DashMap::new(),
        &AtomicU64::new(0),
        &DashMap::new(),
        &routes,
        &h2,
    )
    .await
    .unwrap();
    assert!(!routes.contains_key("/assets/.env"));
    assert!(!h2.contains_key("/assets/.env"));
    assert!(routes.contains_key("/assets/a%20%23.txt"));
}

#[cfg(unix)]
#[tokio::test]
async fn static_symlinks_cannot_leave_root_in_dispatch_preload_or_listing() {
    let root = TestDir::new("static-symlink-security");
    let public = root.join("public");
    std::fs::create_dir_all(&public).unwrap();
    std::fs::write(root.join("secret"), b"outside-secret").unwrap();
    std::fs::write(public.join(".env"), b"hidden-secret").unwrap();
    std::os::unix::fs::symlink(public.join(".env"), public.join("hidden-alias")).unwrap();
    std::os::unix::fs::symlink(root.join("secret"), public.join("escape")).unwrap();
    std::os::unix::fs::symlink(root.join("secret"), public.join("index.html")).unwrap();
    let site = StaticSiteConfig {
        root: public,
        path_prefix: "/assets".into(),
        autoindex: true,
        ..Default::default()
    };
    assert_eq!(
        dispatch(&site, Method::GET, "/assets/escape").await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        dispatch(&site, Method::GET, "/assets/hidden-alias")
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let listing = dispatch(&site, Method::GET, "/assets/").await;
    assert!(!String::from_utf8_lossy(&listing.body).contains("escape"));
    assert!(!String::from_utf8_lossy(&listing.body).contains("outside-secret"));
    let routes = DashMap::new();
    preload_static_site_fast_lane_cache(
        &site,
        RuntimePerformanceTrafficProfile::Small,
        u64::MAX,
        &DashMap::new(),
        &AtomicU64::new(0),
        &DashMap::new(),
        &routes,
        &DashMap::new(),
    )
    .await
    .unwrap();
    assert!(routes.is_empty());
}

#[test]
fn static_html_errors_preserve_cache_rejection_headers() {
    let mut headers = HeaderMap::new();
    headers.insert(http::header::ACCEPT, HeaderValue::from_static("text/html"));
    let response = decorate_error_response(&GatewayConfig::default(), &headers, static_denied());
    assert_eq!(response.status, StatusCode::FORBIDDEN);
    assert!(response
        .headers
        .iter()
        .any(|(name, value)| name == http::header::CACHE_CONTROL && value == "private, no-store"));
    assert!(response
        .headers
        .iter()
        .any(|(name, value)| name == "cdn-cache-control" && value == "no-store"));
}

#[test]
fn protected_sites_and_reload_never_take_unauthenticated_fast_lanes() {
    let mut config = GatewayConfig::default();
    config.logging.access_log = false;
    config.admin.enabled = false;
    config.runtime.hot_reload.enabled = false;
    config
        .services
        .static_sites
        .push(StaticSiteConfig::default());
    assert!(plain_static_fast_path_allowed(&config));
    assert!(hyper_static_success_fast_path_globally_allowed(&config));
    config.services.static_sites[0].security.origin_token =
        "test-only-origin-token-with-32-bytes".into();
    assert!(!plain_static_fast_path_allowed(&config));
    assert!(!hyper_static_success_fast_path_globally_allowed(&config));
    config.services.static_sites[0]
        .security
        .origin_token
        .clear();
    config.runtime.hot_reload.enabled = true;
    assert!(!plain_static_fast_path_allowed(&config));
    assert!(!hyper_static_success_fast_path_globally_allowed(&config));
    for header in [
        "If-None-Match: *",
        "If-Modified-Since: Sun, 06 Nov 1994 08:49:37 GMT",
        "If-Range: stale",
    ] {
        assert!(parse_static_fast_path_request(
            format!("GET /assets/a HTTP/1.1\r\nHost: localhost\r\n{header}\r\n\r\n").as_bytes()
        )
        .is_none());
    }
}

#[test]
fn static_config_validation_and_inspection_mask_secrets() {
    let mut config = GatewayConfig::default();
    let mut site = StaticSiteConfig::default();
    site.security.origin_token = "do-not-disclose-this-origin-token-32".into();
    site.security.signed_url.secret = "do-not-disclose-this-signing-secret-32".into();
    site.security.signed_url.enabled = true;
    config.services.static_sites.push(site.clone());
    assert!(config.validate().is_ok());
    let inspected = sanitize_config(&config).to_string();
    let yaml = crate::render_redacted_config_yaml(&config).unwrap();
    for value in [&inspected, &yaml] {
        assert!(!value.contains(&site.security.origin_token));
        assert!(!value.contains(&site.security.signed_url.secret));
    }
    site.autoindex = true;
    assert!(crate::static_security::validate(&site).is_err());
    site.autoindex = false;
    site.security.allowed_peers = vec!["127.0.0.1/99".into()];
    assert!(crate::static_security::validate(&site).is_err());
}

#[tokio::test]
async fn static_reload_revokes_cached_access_without_new_listener() {
    let root = TestDir::new("static-reload-security");
    std::fs::write(root.join("file.txt"), b"cached-static").unwrap();
    let port = crate::verify::harness::reserve_port().await.unwrap();
    let base = crate::verify::harness::base_gateway_yaml(port).replace(
        "hot_reload:\n    enabled: false",
        "hot_reload:\n    enabled: true",
    );
    let yaml = format!("{base}\nservices:\n  static_sites:\n    - name: files\n      path_prefix: /assets\n      root: '{}'\n", root.display().to_string().replace('\\', "/"));
    let path = crate::verify::harness::write_config(&root, &yaml).unwrap();
    let (gateway, runner) = crate::verify::harness::spawn_gateway(path.clone())
        .await
        .unwrap();
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/assets/file.txt");
    assert_eq!(
        client.get(&url).send().await.unwrap().status(),
        StatusCode::OK
    );
    let token = "test-reloaded-token-at-least-32-bytes";
    std::fs::write(
        &path,
        format!("{yaml}      security:\n        origin_token: {token}\n"),
    )
    .unwrap();
    gateway.reload_from_disk().await.unwrap();
    assert_eq!(
        client.get(&url).send().await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        client
            .get(&url)
            .header(crate::static_security::ORIGIN_TOKEN_HEADER, token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    runner.abort();
}
