mod auth;
pub mod path;

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use http::header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_LENGTH, SET_COOKIE};
use http::{HeaderMap, HeaderValue, Method, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::config::FileCloudConfig;
use crate::gateway::GatewayHttpResponse;

use auth::{derive_secret, issue_session, password_matches, verify_session, SESSION_COOKIE};
use path::{
    ensure_within_root, normalize_prefix, path_matches, relative_route, resolve_relative_path,
};

const SHAREFILE_HTML: &str = include_str!("../../templates/sharefile.html");

pub async fn dispatch_filecloud(
    config: &FileCloudConfig,
    method: &Method,
    request_path: &str,
    query: Option<&str>,
    headers: &HeaderMap,
    body: Bytes,
) -> Result<GatewayHttpResponse> {
    if !config.enabled {
        return Ok(GatewayHttpResponse::error(
            StatusCode::NOT_FOUND,
            "filecloud is disabled",
        ));
    }

    let prefix = normalize_prefix(&config.path_prefix);
    if !path_matches(&prefix, request_path) {
        return Ok(GatewayHttpResponse::error(
            StatusCode::NOT_FOUND,
            "filecloud path not found",
        ));
    }

    fs::create_dir_all(&config.root)
        .await
        .with_context(|| format!("failed to create filecloud root {}", config.root.display()))?;

    let relative = relative_route(&prefix, request_path).unwrap_or_default();
    let secret = derive_secret(&config.password, &config.root.to_string_lossy());

    if relative == "api/login" && method == Method::POST {
        return handle_login(config, &prefix, &secret, &body).await;
    }

    if relative.starts_with("api/") {
        if !is_authenticated(&secret, headers) {
            return Ok(json_error(
                StatusCode::UNAUTHORIZED,
                "filecloud authentication required",
            ));
        }
        return dispatch_api(config, method, &relative, query, &body).await;
    }

    if relative == "dl" || relative.starts_with("dl/") {
        let file_relative = relative
            .strip_prefix("dl/")
            .unwrap_or("")
            .trim_start_matches('/');
        return serve_download(config, file_relative, headers, &secret).await;
    }

    if method == Method::GET && (relative.is_empty() || relative == "index.html") {
        if !is_authenticated(&secret, headers) {
            return Ok(render_login_page(config, &prefix));
        }
        return Ok(render_sharefile_ui(config, &prefix));
    }

    Ok(GatewayHttpResponse::error(
        StatusCode::NOT_FOUND,
        "filecloud route not found",
    ))
}

async fn dispatch_api(
    config: &FileCloudConfig,
    method: &Method,
    relative: &str,
    query: Option<&str>,
    body: &Bytes,
) -> Result<GatewayHttpResponse> {
    match relative {
        "api/tree" if *method == Method::GET => handle_tree(config, parse_query(query)).await,
        "api/search" if *method == Method::GET => handle_search(config, parse_query(query)).await,
        "api/mkdir" if *method == Method::POST => handle_mkdir(config, body).await,
        "api/move" if *method == Method::POST => handle_move(config, body).await,
        "api/rename" if *method == Method::POST => handle_rename(config, body).await,
        "api/delete" if *method == Method::POST => handle_delete(config, body).await,
        "api/upload" if *method == Method::PUT => {
            handle_upload(config, parse_query(query), body).await
        }
        _ => Ok(json_error(
            StatusCode::NOT_FOUND,
            "unknown filecloud api route",
        )),
    }
}

async fn handle_login(
    config: &FileCloudConfig,
    prefix: &str,
    secret: &[u8; 32],
    body: &Bytes,
) -> Result<GatewayHttpResponse> {
    let payload: LoginRequest = serde_json::from_slice(body)
        .context("filecloud login payload must be json {\"password\":\"...\"}")?;
    if !password_matches(&config.password, payload.password.trim()) {
        return Ok(json_error(
            StatusCode::UNAUTHORIZED,
            "invalid filecloud password",
        ));
    }

    let token = issue_session(secret, config.session_ttl_secs);
    let cookie = format!(
        "{SESSION_COOKIE}={token}; Path={prefix}; HttpOnly; SameSite=Strict; Max-Age={}",
        config.session_ttl_secs
    );
    let mut response = json_ok(LoginResponse { ok: true })?;
    response.push_header(
        SET_COOKIE,
        HeaderValue::from_str(&cookie).context("invalid filecloud session cookie")?,
    );
    Ok(response)
}

async fn handle_tree(config: &FileCloudConfig, query: TreeQuery) -> Result<GatewayHttpResponse> {
    let target = resolve_relative_path(&config.root, &query.path)?;
    ensure_within_root(&config.root, &target)?;
    if !target.exists() {
        return Ok(json_error(StatusCode::NOT_FOUND, "directory not found"));
    }
    let metadata = fs::metadata(&target)
        .await
        .context("failed reading filecloud directory metadata")?;
    if !metadata.is_dir() {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "tree path must be a directory",
        ));
    }

    let mut nodes = Vec::new();
    let mut entries = fs::read_dir(&target)
        .await
        .context("failed listing filecloud directory")?;
    while let Some(entry) = entries.next_entry().await? {
        nodes.push(read_node(&config.root, &entry).await?);
    }
    nodes.sort_by(|left, right| match (left.is_dir, right.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => left.name.to_lowercase().cmp(&right.name.to_lowercase()),
    });

    json_ok(TreeResponse {
        path: normalize_virtual_path(&query.path),
        nodes,
    })
}

async fn handle_search(
    config: &FileCloudConfig,
    query: SearchQuery,
) -> Result<GatewayHttpResponse> {
    if query.q.trim().is_empty() {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "search query cannot be empty",
        ));
    }
    let base = resolve_relative_path(&config.root, &query.path)?;
    ensure_within_root(&config.root, &base)?;
    if !base.is_dir() {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "search path must be a directory",
        ));
    }

    let needle = query.q.trim().to_ascii_lowercase();
    let mut matches = Vec::new();
    search_recursive(&config.root, &base, &needle, &mut matches, 0, 500).await?;
    json_ok(SearchResponse { matches })
}

async fn handle_mkdir(config: &FileCloudConfig, body: &Bytes) -> Result<GatewayHttpResponse> {
    if !config.allow_mkdir {
        return Ok(json_error(
            StatusCode::FORBIDDEN,
            "filecloud mkdir is disabled",
        ));
    }
    let payload: PathRequest = serde_json::from_slice(body)?;
    let target = resolve_relative_path(&config.root, &payload.path)?;
    ensure_within_root(&config.root, &target)?;
    fs::create_dir_all(&target)
        .await
        .context("failed creating filecloud directory")?;
    json_ok(ActionResponse { ok: true })
}

async fn handle_move(config: &FileCloudConfig, body: &Bytes) -> Result<GatewayHttpResponse> {
    if !config.allow_move {
        return Ok(json_error(
            StatusCode::FORBIDDEN,
            "filecloud move is disabled",
        ));
    }
    let payload: MoveRequest = serde_json::from_slice(body)?;
    let from = resolve_relative_path(&config.root, &payload.from)?;
    let to = resolve_relative_path(&config.root, &payload.to)?;
    ensure_within_root(&config.root, &from)?;
    ensure_within_root(&config.root, &to)?;
    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent)
            .await
            .context("failed creating filecloud destination parent")?;
    }
    fs::rename(&from, &to)
        .await
        .context("failed moving filecloud path")?;
    json_ok(ActionResponse { ok: true })
}

async fn handle_rename(config: &FileCloudConfig, body: &Bytes) -> Result<GatewayHttpResponse> {
    if !config.allow_move {
        return Ok(json_error(
            StatusCode::FORBIDDEN,
            "filecloud rename is disabled",
        ));
    }
    let payload: RenameRequest = serde_json::from_slice(body)?;
    let from = resolve_relative_path(&config.root, &payload.path)?;
    let parent = from
        .parent()
        .ok_or_else(|| anyhow!("invalid rename path"))?;
    let to = parent.join(payload.name.trim());
    ensure_within_root(&config.root, &from)?;
    ensure_within_root(&config.root, &to)?;
    fs::rename(&from, &to)
        .await
        .context("failed renaming filecloud path")?;
    json_ok(ActionResponse { ok: true })
}

async fn handle_delete(config: &FileCloudConfig, body: &Bytes) -> Result<GatewayHttpResponse> {
    if !config.allow_delete {
        return Ok(json_error(
            StatusCode::FORBIDDEN,
            "filecloud delete is disabled",
        ));
    }
    let payload: PathRequest = serde_json::from_slice(body)?;
    let target = resolve_relative_path(&config.root, &payload.path)?;
    ensure_within_root(&config.root, &target)?;
    if !target.exists() {
        return Ok(json_error(StatusCode::NOT_FOUND, "path not found"));
    }
    let metadata = fs::metadata(&target).await?;
    if metadata.is_dir() {
        fs::remove_dir_all(&target)
            .await
            .context("failed deleting filecloud directory")?;
    } else {
        fs::remove_file(&target)
            .await
            .context("failed deleting filecloud file")?;
    }
    json_ok(ActionResponse { ok: true })
}

async fn handle_upload(
    config: &FileCloudConfig,
    query: UploadQuery,
    body: &Bytes,
) -> Result<GatewayHttpResponse> {
    if !config.allow_upload {
        return Ok(json_error(
            StatusCode::FORBIDDEN,
            "filecloud upload is disabled",
        ));
    }
    if query.name.trim().is_empty() {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "upload name query parameter is required",
        ));
    }
    if body.len() as u64 > config.max_upload_bytes {
        return Ok(json_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "upload exceeds filecloud max_upload_bytes",
        ));
    }
    let dir = resolve_relative_path(&config.root, &query.path)?;
    ensure_within_root(&config.root, &dir)?;
    fs::create_dir_all(&dir)
        .await
        .context("failed creating filecloud upload directory")?;
    let target = dir.join(query.name.trim());
    ensure_within_root(&config.root, &target)?;
    fs::write(&target, body)
        .await
        .context("failed writing uploaded filecloud file")?;
    json_ok(UploadResponse {
        ok: true,
        path: join_virtual_path(&query.path, query.name.trim()),
        size: body.len() as u64,
    })
}

async fn serve_download(
    config: &FileCloudConfig,
    relative: &str,
    headers: &HeaderMap,
    secret: &[u8; 32],
) -> Result<GatewayHttpResponse> {
    if config.require_auth_for_download && !is_authenticated(secret, headers) {
        return Ok(json_error(
            StatusCode::UNAUTHORIZED,
            "filecloud authentication required",
        ));
    }
    let target = resolve_relative_path(&config.root, relative)?;
    ensure_within_root(&config.root, &target)?;
    let metadata = match fs::metadata(&target).await {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(GatewayHttpResponse::error(
                StatusCode::NOT_FOUND,
                "file not found",
            ));
        }
        Err(error) => return Err(error.into()),
    };
    if metadata.is_dir() {
        return Ok(json_error(
            StatusCode::BAD_REQUEST,
            "cannot download a directory",
        ));
    }

    let bytes = fs::read(&target)
        .await
        .context("failed reading filecloud download file")?;
    let file_name = target
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("download");
    let mut response = GatewayHttpResponse::bytes(
        StatusCode::OK,
        filecloud_content_type(&target),
        Bytes::from(bytes),
        "proxysss://filecloud",
    );
    response.push_header(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("inline; filename=\"{file_name}\""))?,
    );
    response.push_header(
        CONTENT_LENGTH,
        HeaderValue::from_str(&metadata.len().to_string())?,
    );
    if config.cdn_cache_secs > 0 {
        response.push_header(
            CACHE_CONTROL,
            HeaderValue::from_str(&format!(
                "public, max-age={}, immutable",
                config.cdn_cache_secs
            ))?,
        );
    }
    Ok(response)
}

async fn search_recursive(
    root: &Path,
    current: &Path,
    needle: &str,
    matches: &mut Vec<FileNode>,
    depth: u32,
    limit: usize,
) -> Result<()> {
    if matches.len() >= limit || depth > 12 {
        return Ok(());
    }
    let mut entries = fs::read_dir(current).await?;
    while let Some(entry) = entries.next_entry().await? {
        if matches.len() >= limit {
            break;
        }
        let node = read_node(root, &entry).await?;
        if node.name.to_ascii_lowercase().contains(needle) {
            matches.push(node.clone());
        }
        if node.is_dir {
            let child = resolve_relative_path(root, node.path.trim_start_matches('/'))?;
            Box::pin(search_recursive(
                root,
                &child,
                needle,
                matches,
                depth + 1,
                limit,
            ))
            .await?;
        }
    }
    Ok(())
}

async fn read_node(root: &Path, entry: &fs::DirEntry) -> Result<FileNode> {
    let metadata = entry.metadata().await?;
    let file_name = entry.file_name().to_string_lossy().into_owned();
    let absolute = entry.path();
    let relative = absolute
        .strip_prefix(root)
        .unwrap_or(&absolute)
        .to_string_lossy()
        .replace('\\', "/");
    let path = if relative.is_empty() {
        "/".to_string()
    } else {
        format!("/{relative}")
    };
    Ok(FileNode {
        name: file_name,
        path,
        is_dir: metadata.is_dir(),
        size: if metadata.is_dir() { 0 } else { metadata.len() },
        modified: metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    })
}

fn render_sharefile_ui(config: &FileCloudConfig, prefix: &str) -> GatewayHttpResponse {
    let html = SHAREFILE_HTML
        .replace("__FILECLOUD_PREFIX__", prefix)
        .replace("__FILECLOUD_TITLE__", &config.title)
        .replace(
            "__FILECLOUD_MAX_UPLOAD__",
            &config.max_upload_bytes.to_string(),
        );
    GatewayHttpResponse::html(html, "proxysss://filecloud")
}

fn render_login_page(config: &FileCloudConfig, prefix: &str) -> GatewayHttpResponse {
    let html = format!(
        r#"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{title} · Login</title><style>body{{margin:0;min-height:100vh;display:grid;place-items:center;font:18px/1.5 Inter,system-ui,sans-serif;background:#f5f7fb;color:#1f2937}}.card{{width:min(420px,calc(100vw - 32px));background:#fff;border:1px solid #e5e7eb;border-radius:16px;padding:28px;box-shadow:0 16px 40px rgba(15,23,42,.08)}}h1{{margin:0 0 8px;font-size:28px}}p{{margin:0 0 20px;color:#6b7280;font-size:17px}}input{{width:100%;box-sizing:border-box;padding:14px 16px;border:1px solid #d1d5db;border-radius:12px;font-size:18px;margin-bottom:14px}}button{{width:100%;padding:14px 16px;border:0;border-radius:12px;background:#1677ff;color:#fff;font-size:18px;font-weight:600;cursor:pointer}}.err{{color:#dc2626;min-height:24px;margin-top:10px;font-size:16px}}</style></head><body><div class="card"><h1>{title}</h1><p>输入共享密码以进入 FileCloud。</p><input id="password" type="password" placeholder="共享密码" autofocus><button id="login">进入</button><div class="err" id="error"></div></div><script>const prefix="{prefix}";const password=document.getElementById("password");const error=document.getElementById("error");document.getElementById("login").onclick=async()=>{{error.textContent="";const res=await fetch(prefix+"/api/login",{{method:"POST",headers:{{"Content-Type":"application/json"}},body:JSON.stringify({{password:password.value}})}});if(!res.ok){{error.textContent="密码错误";return}}location.reload()}};password.addEventListener("keydown",event=>{{if(event.key==="Enter")document.getElementById("login").click()}});</script></body></html>"#,
        title = config.title,
        prefix = prefix,
    );
    GatewayHttpResponse::html(html, "proxysss://filecloud")
}

fn is_authenticated(secret: &[u8; 32], headers: &HeaderMap) -> bool {
    session_from_headers(headers)
        .map(|token| verify_session(secret, &token))
        .unwrap_or(false)
}

fn session_from_headers(headers: &HeaderMap) -> Option<String> {
    let cookie = headers
        .get("cookie")
        .and_then(|value| value.to_str().ok())?;
    cookie.split(';').find_map(|part| {
        let part = part.trim();
        part.strip_prefix(&format!("{SESSION_COOKIE}="))
            .map(str::to_string)
    })
}

fn parse_query<T: for<'de> Deserialize<'de> + Default>(query: Option<&str>) -> T {
    let Some(raw) = query.filter(|value| !value.is_empty()) else {
        return T::default();
    };
    let mut map = serde_json::Map::new();
    for pair in raw.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            map.insert(
                decode_query_value(key),
                serde_json::Value::String(decode_query_value(value)),
            );
        }
    }
    serde_json::from_value(serde_json::Value::Object(map)).unwrap_or_default()
}

fn decode_query_value(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(decoded) = u8::from_str_radix(
                std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or(""),
                16,
            ) {
                output.push(decoded);
            }
            index += 3;
        } else if bytes[index] == b'+' {
            output.push(b' ');
            index += 1;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).unwrap_or_else(|_| value.to_string())
}

fn normalize_virtual_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "/" {
        "/".to_string()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn join_virtual_path(base: &str, name: &str) -> String {
    let base = normalize_virtual_path(base);
    if base == "/" {
        format!("/{name}")
    } else {
        format!("{base}/{name}")
    }
}

fn filecloud_content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "json" => "application/json",
        "txt" => "text/plain; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

fn json_ok<T: Serialize>(value: T) -> Result<GatewayHttpResponse> {
    GatewayHttpResponse::json(StatusCode::OK, &value, "proxysss://filecloud")
}

fn json_error(status: StatusCode, message: &str) -> GatewayHttpResponse {
    GatewayHttpResponse::json(
        status,
        &ErrorResponse {
            ok: false,
            error: message.to_string(),
        },
        "proxysss://filecloud",
    )
    .unwrap_or_else(|_| GatewayHttpResponse::error(status, message))
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    password: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    ok: bool,
}

#[derive(Debug, Deserialize, Default)]
struct TreeQuery {
    #[serde(default)]
    path: String,
}

#[derive(Debug, Deserialize, Default)]
struct SearchQuery {
    #[serde(default)]
    path: String,
    #[serde(default)]
    q: String,
}

#[derive(Debug, Deserialize, Default)]
struct UploadQuery {
    #[serde(default)]
    path: String,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct PathRequest {
    path: String,
}

#[derive(Debug, Deserialize)]
struct MoveRequest {
    from: String,
    to: String,
}

#[derive(Debug, Deserialize)]
struct RenameRequest {
    path: String,
    name: String,
}

#[derive(Debug, Serialize)]
struct TreeResponse {
    path: String,
    nodes: Vec<FileNode>,
}

#[derive(Debug, Clone, Serialize)]
struct FileNode {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
    modified: u64,
}

#[derive(Debug, Serialize)]
struct SearchResponse {
    matches: Vec<FileNode>,
}

#[derive(Debug, Serialize)]
struct ActionResponse {
    ok: bool,
}

#[derive(Debug, Serialize)]
struct UploadResponse {
    ok: bool,
    path: String,
    size: u64,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    ok: bool,
    error: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;
    use std::path::PathBuf;
    use uuid::Uuid;

    fn test_config(root: PathBuf) -> FileCloudConfig {
        FileCloudConfig {
            enabled: true,
            path_prefix: "/filecloud".to_string(),
            root,
            password: "test-secret".to_string(),
            title: "Test FileCloud".to_string(),
            ..FileCloudConfig::default()
        }
    }

    #[tokio::test]
    async fn filecloud_login_upload_tree_download_flow() {
        let root = std::env::temp_dir().join(format!("proxysss-filecloud-flow-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&root).await.expect("create root");
        let config = test_config(root.clone());

        let login = dispatch_filecloud(
            &config,
            &Method::POST,
            "/filecloud/api/login",
            None,
            &HeaderMap::new(),
            Bytes::from(r#"{"password":"test-secret"}"#),
        )
        .await
        .expect("login dispatch");
        assert_eq!(login.status(), StatusCode::OK);
        let cookie = login
            .headers()
            .iter()
            .find(|(name, _)| *name == SET_COOKIE)
            .map(|(_, value)| value.to_str().expect("cookie").to_string())
            .expect("session cookie");

        let mut headers = HeaderMap::new();
        headers.insert(
            http::header::COOKIE,
            HeaderValue::from_str(&cookie).expect("cookie header"),
        );

        let upload = dispatch_filecloud(
            &config,
            &Method::PUT,
            "/filecloud/api/upload",
            Some("path=/&name=hello.txt"),
            &headers,
            Bytes::from_static(b"hello filecloud"),
        )
        .await
        .expect("upload dispatch");
        assert_eq!(upload.status(), StatusCode::OK);

        let tree = dispatch_filecloud(
            &config,
            &Method::GET,
            "/filecloud/api/tree",
            Some("path=/"),
            &headers,
            Bytes::new(),
        )
        .await
        .expect("tree dispatch");
        assert_eq!(tree.status(), StatusCode::OK);
        let body = String::from_utf8(tree.body().to_vec()).expect("utf8");
        assert!(body.contains("hello.txt"));

        let download = dispatch_filecloud(
            &config,
            &Method::GET,
            "/filecloud/dl/hello.txt",
            None,
            &HeaderMap::new(),
            Bytes::new(),
        )
        .await
        .expect("download dispatch");
        assert_eq!(download.status(), StatusCode::OK);
        assert_eq!(*download.body(), Bytes::from_static(b"hello filecloud"));

        let _ = tokio::fs::remove_dir_all(root).await;
    }
}
