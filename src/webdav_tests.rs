use super::*;
use crate::test_support::TestDir;

async fn request(
    config: &WebDavConfig,
    method: &str,
    path: &str,
    destination: Option<&str>,
) -> GatewayHttpResponse {
    let mut headers = HeaderMap::new();
    if method == "PROPFIND" {
        headers.insert("depth", HeaderValue::from_static("1"));
    }
    if let Some(destination) = destination {
        headers.insert("destination", destination.parse().unwrap());
    }
    let mut response = dispatch_webdav(
        config,
        &Method::from_bytes(method.as_bytes()).unwrap(),
        &path.parse().unwrap(),
        &headers,
        Bytes::from_static(b"new-content"),
    )
    .await
    .unwrap();
    if let Some(body) = response.stream_body.take() {
        response.body = body.collect().await.unwrap().to_bytes();
    }
    response
}

fn config(root: PathBuf) -> WebDavConfig {
    WebDavConfig {
        enabled: true,
        path_prefix: "/dav".into(),
        root,
        allow_write: true,
        ..Default::default()
    }
}

#[tokio::test]
async fn webdav_auth_precedes_filesystem_and_applies_to_every_method() {
    let fixture = TestDir::new("webdav-auth");
    let mut config = config(fixture.join("uncreated"));
    config.auth = crate::config::WebDavAuthConfig {
        enabled: true,
        username: "builder".into(),
        password: "unicode-密钥:with-colon".into(),
    };
    for method in [
        "OPTIONS", "GET", "HEAD", "PROPFIND", "PUT", "MKCOL", "DELETE", "COPY", "MOVE",
    ] {
        let denied = request(&config, method, "/dav/new", Some("/dav/dest")).await;
        assert_eq!(denied.status, StatusCode::UNAUTHORIZED, "{method}");
        assert!(denied
            .headers
            .iter()
            .any(|(name, value)| name == http::header::WWW_AUTHENTICATE
                && value.to_str().unwrap().starts_with("Basic ")));
    }
    assert!(!config.root.exists());
    let mut headers = HeaderMap::new();
    let credential = base64::engine::general_purpose::STANDARD
        .encode(format!("{}:{}", config.auth.username, config.auth.password));
    headers.insert(
        AUTHORIZATION,
        format!("Basic {credential}").parse().unwrap(),
    );
    let created = dispatch_webdav(
        &config,
        &Method::PUT,
        &"/dav/item".parse().unwrap(),
        &headers,
        Bytes::from_static(b"private-file"),
    )
    .await
    .unwrap();
    assert_eq!(created.status, StatusCode::CREATED);
    headers.insert(RANGE, HeaderValue::from_static("bytes=0-6"));
    let response = dispatch_webdav(
        &config,
        &Method::GET,
        &"/dav/item".parse().unwrap(),
        &headers,
        Bytes::new(),
    )
    .await
    .unwrap();
    assert_eq!(response.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(response.body.as_ref(), b"private");
    assert!(response
        .headers
        .iter()
        .any(|(name, value)| name == CACHE_CONTROL && value == "private, no-store"));
    headers.append(AUTHORIZATION, HeaderValue::from_static("Basic invalid"));
    assert!(!webdav_authenticated(&config, &headers));
    config.auth.password = "rotated-password".into();
    headers.remove(AUTHORIZATION);
    headers.insert(
        AUTHORIZATION,
        format!("Basic {credential}").parse().unwrap(),
    );
    assert!(!webdav_authenticated(&config, &headers));
}

#[tokio::test]
async fn webdav_depth_overwrite_parent_and_url_semantics() {
    let fixture = TestDir::new("webdav-methods");
    let config = config(fixture.to_path_buf());
    std::fs::create_dir(fixture.join("source")).unwrap();
    std::fs::write(fixture.join("source/a &+%.txt"), b"source").unwrap();
    std::fs::create_dir(fixture.join("dest")).unwrap();
    std::fs::write(fixture.join("dest/old"), b"old").unwrap();
    let propfind = Method::from_bytes(b"PROPFIND").unwrap();
    let copy = Method::from_bytes(b"COPY").unwrap();
    let mut headers = HeaderMap::new();
    assert_eq!(
        dispatch_webdav(
            &config,
            &propfind,
            &"/dav/source".parse().unwrap(),
            &headers,
            Bytes::new()
        )
        .await
        .unwrap()
        .status,
        StatusCode::FORBIDDEN
    );
    headers.insert("depth", HeaderValue::from_static("0"));
    let mut listing = dispatch_webdav(
        &config,
        &propfind,
        &"/dav/source".parse().unwrap(),
        &headers,
        Bytes::new(),
    )
    .await
    .unwrap();
    let body = listing
        .stream_body
        .take()
        .unwrap()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    assert_eq!(
        String::from_utf8(body.to_vec())
            .unwrap()
            .matches("<D:response>")
            .count(),
        1
    );
    let listing = request(&config, "PROPFIND", "/dav/source", None).await;
    let xml = String::from_utf8(listing.body.to_vec()).unwrap();
    assert!(xml.contains("a%20%26%2B%25.txt"));
    assert!(!xml.contains("&amp;amp;"));
    headers.insert("destination", HeaderValue::from_static("/dav/dest"));
    headers.insert("overwrite", HeaderValue::from_static("F"));
    assert_eq!(
        dispatch_webdav(
            &config,
            &copy,
            &"/dav/source".parse().unwrap(),
            &headers,
            Bytes::new()
        )
        .await
        .unwrap()
        .status,
        StatusCode::PRECONDITION_FAILED
    );
    assert!(fixture.join("dest/old").exists());
    headers.insert("overwrite", HeaderValue::from_static("T"));
    assert_eq!(
        dispatch_webdav(
            &config,
            &copy,
            &"/dav/source".parse().unwrap(),
            &headers,
            Bytes::new()
        )
        .await
        .unwrap()
        .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(std::fs::read_dir(fixture.join("dest")).unwrap().count(), 0);
    assert_eq!(
        request(&config, "PUT", "/dav/missing/item", None)
            .await
            .status,
        StatusCode::CONFLICT
    );
    assert!(!fixture.join("missing").exists());
    headers.insert(HOST, HeaderValue::from_static("files.example.test"));
    headers.insert(
        "destination",
        HeaderValue::from_static("https://other.example.test/dav/dest"),
    );
    assert_eq!(
        dispatch_webdav(
            &config,
            &copy,
            &"/dav/source".parse().unwrap(),
            &headers,
            Bytes::new()
        )
        .await
        .unwrap()
        .status,
        StatusCode::BAD_GATEWAY
    );
}

#[cfg(unix)]
#[tokio::test]
async fn webdav_root_boundary_covers_methods_destinations_and_dangling_links() {
    use std::os::unix::fs::symlink;
    let fixture = TestDir::new("webdav-boundary");
    let root = fixture.join("served");
    let sibling = fixture.join("sibling");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&sibling).unwrap();
    std::fs::write(sibling.join("secret"), b"sibling-only").unwrap();
    std::fs::write(root.join("source"), b"source").unwrap();
    symlink(&sibling, root.join("outside")).unwrap();
    symlink(sibling.join("missing"), root.join("dangling")).unwrap();
    let config = config(root.clone());
    for method in [
        "GET", "HEAD", "PROPFIND", "PUT", "MKCOL", "DELETE", "COPY", "MOVE",
    ] {
        let response = request(&config, method, "/dav/outside/secret", Some("/dav/copied")).await;
        assert_eq!(response.status, StatusCode::FORBIDDEN, "source {method}");
    }
    for destination in ["/dav/outside/new", "/dav/dangling", "/dav/dangling/child"] {
        for method in ["COPY", "MOVE"] {
            let response = request(&config, method, "/dav/source", Some(destination)).await;
            assert_eq!(
                response.status,
                StatusCode::FORBIDDEN,
                "{method} {destination}"
            );
        }
        assert_eq!(
            request(&config, "PUT", destination, None).await.status,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            request(&config, "MKCOL", destination, None).await.status,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        std::fs::read(sibling.join("secret")).unwrap(),
        b"sibling-only"
    );
    assert_eq!(std::fs::read(root.join("source")).unwrap(), b"source");
    assert!(!sibling.join("new").exists());
    assert!(!sibling.join("missing").exists());
    let listing = request(&config, "PROPFIND", "/dav", None).await;
    assert_eq!(listing.status.as_u16(), 207);
    let listing = String::from_utf8(listing.body.to_vec()).unwrap();
    assert!(listing.contains("source"));
    assert!(!listing.contains("outside"));
    assert!(!listing.contains("dangling"));
}

#[tokio::test]
async fn webdav_copy_rejects_self_and_descendants_before_creating_them() {
    let fixture = TestDir::new("webdav-copy");
    std::fs::create_dir_all(fixture.join("source")).unwrap();
    std::fs::write(fixture.join("source/item"), b"original").unwrap();
    let config = config(fixture.to_path_buf());
    for destination in ["/dav/source", "/dav/source/nested/copy", "/dav/source/item"] {
        assert_eq!(
            request(&config, "COPY", "/dav/source", Some(destination))
                .await
                .status,
            StatusCode::FORBIDDEN
        );
    }
    assert!(!fixture.join("source/nested").exists());
    assert_eq!(
        std::fs::read(fixture.join("source/item")).unwrap(),
        b"original"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn webdav_recursive_copy_checks_children_and_preserves_safe_links() {
    use std::os::unix::fs::symlink;
    let fixture = TestDir::new("webdav-recursive");
    let root = fixture.join("served");
    let sibling = fixture.join("sibling");
    std::fs::create_dir_all(root.join("source")).unwrap();
    std::fs::create_dir_all(root.join("safe")).unwrap();
    std::fs::create_dir_all(&sibling).unwrap();
    std::fs::write(sibling.join("secret"), b"outside").unwrap();
    std::fs::write(root.join("safe/item"), b"inside").unwrap();
    symlink(sibling.join("secret"), root.join("source/escape")).unwrap();
    symlink(root.join("safe"), root.join("alias")).unwrap();
    let config = config(root.clone());
    assert_eq!(
        request(&config, "COPY", "/dav/source", Some("/dav/copy"))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert!(!root.join("copy/escape").exists());
    assert_eq!(
        request(&config, "COPY", "/dav/safe", Some("/dav/alias/nested/copy"))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert!(!root.join("safe/nested").exists());
    // Overwrite replaces the destination collection without following its child links.
    std::fs::create_dir_all(root.join("dest")).unwrap();
    symlink(sibling.join("secret"), root.join("dest/item")).unwrap();
    assert_eq!(
        request(&config, "COPY", "/dav/safe", Some("/dav/dest"))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(std::fs::read(sibling.join("secret")).unwrap(), b"outside");
    // Internal links remain usable for reads, writes and recursive copies.
    assert_eq!(
        request(&config, "GET", "/dav/alias/item", None)
            .await
            .body
            .as_ref(),
        b"inside"
    );
    assert_eq!(
        request(&config, "PUT", "/dav/alias/new", None).await.status,
        StatusCode::CREATED
    );
    assert_eq!(
        request(&config, "COPY", "/dav/alias", Some("/dav/safe-copy"))
            .await
            .status,
        StatusCode::CREATED
    );
    assert_eq!(
        std::fs::read(root.join("safe-copy/item")).unwrap(),
        b"inside"
    );
    assert_eq!(
        request(
            &config,
            "MOVE",
            "/dav/alias/new",
            Some("/dav/safe-copy/moved")
        )
        .await
        .status,
        StatusCode::CREATED
    );
    assert_eq!(
        request(&config, "DELETE", "/dav/alias", None).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(std::fs::read(root.join("safe/item")).unwrap(), b"inside");
    symlink(root.join("safe"), root.join("safe/loop")).unwrap();
    assert_eq!(
        request(&config, "COPY", "/dav/safe", Some("/dav/cycle-copy"))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert!(!root.join("cycle-copy/loop").exists());
}

#[tokio::test]
async fn webdav_options_only_declares_implemented_methods() {
    let fixture = TestDir::new("webdav-options");
    let mut config = config(fixture.to_path_buf());
    for write in [false, true] {
        config.allow_write = write;
        let response = request(&config, "OPTIONS", "/dav", None).await;
        assert_eq!(response.status, StatusCode::NO_CONTENT);
        assert!(!response.headers.iter().any(|(name, _)| name == "dav"));
        let allow = response
            .headers
            .iter()
            .find(|(name, _)| name == http::header::ALLOW)
            .unwrap()
            .1
            .to_str()
            .unwrap();
        assert_eq!(allow.contains("PUT"), write);
        assert!(!allow.contains("LOCK"));
    }
}
