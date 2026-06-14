//! First-class AI reverse proxy routing (New API, sub2api, OpenAI-compatible).

use std::collections::BTreeMap;

use http::Uri;

use crate::script::RouteDecision;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiProxyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_header_prefix")]
    pub header_prefix: String,
    #[serde(default)]
    pub routes: Vec<AiProxyRouteConfig>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AiProxyRouteConfig {
    pub name: String,
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub match_host: String,
    #[serde(default = "default_route_path_prefix")]
    pub path_prefix: String,
    pub upstream: String,
    #[serde(default)]
    pub rewrite_base_path: String,
    #[serde(default)]
    pub add_headers: BTreeMap<String, String>,
    #[serde(default)]
    pub strip_headers: Vec<String>,
    #[serde(default = "default_true")]
    pub forward_headers: bool,
    #[serde(default = "default_true")]
    pub emit_metadata_headers: bool,
}

impl Default for AiProxyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            header_prefix: default_header_prefix(),
            routes: Vec::new(),
        }
    }
}

fn default_header_prefix() -> String {
    "proxysss-".to_string()
}

fn default_route_path_prefix() -> String {
    "/".to_string()
}

pub fn route_matches(route: &AiProxyRouteConfig, host: &str, path: &str) -> bool {
    let host_ok =
        route.match_host.trim().is_empty() || host.eq_ignore_ascii_case(route.match_host.trim());
    if !host_ok {
        return false;
    }
    route_prefix_matches(&route.path_prefix, path)
}

pub fn build_route_decision(
    route: &AiProxyRouteConfig,
    uri: &Uri,
    header_prefix: &str,
) -> RouteDecision {
    let prefix = normalize_header_prefix(header_prefix);
    let mut set_headers = route.add_headers.clone();
    if route.emit_metadata_headers {
        set_headers.insert(format!("{prefix}ai-route"), route.name.clone());
        set_headers.insert(
            format!("{prefix}ai-provider"),
            if route.provider.is_empty() {
                route.name.clone()
            } else {
                route.provider.clone()
            },
        );
        set_headers.insert(format!("{prefix}ai-original-path"), uri.path().to_string());
    }

    RouteDecision {
        upstream: route.upstream.clone(),
        upstreams: Vec::new(),
        upstream_weights: BTreeMap::new(),
        affinity_key: None,
        rewrite_path: rewrite_path(route, uri),
        set_headers,
        strip_headers: route.strip_headers.clone(),
        status: None,
        content_type: None,
    }
}

fn default_true() -> bool {
    true
}

fn route_prefix_matches(prefix: &str, path: &str) -> bool {
    let prefix = prefix.trim().trim_end_matches('/');
    if prefix.is_empty() {
        return path.starts_with('/');
    }

    let suffix = if prefix.starts_with('/') {
        path.strip_prefix(prefix)
    } else {
        path.strip_prefix('/')
            .and_then(|path_without_slash| path_without_slash.strip_prefix(prefix))
    };

    match suffix {
        Some("") => true,
        Some(rest) => rest.starts_with('/'),
        None => false,
    }
}

fn route_prefix_suffix<'a>(prefix: &str, path: &'a str) -> Option<&'a str> {
    let prefix = prefix.trim().trim_end_matches('/');
    if prefix.is_empty() {
        return Some(path);
    }

    if prefix.starts_with('/') {
        path.strip_prefix(prefix)
    } else {
        path.strip_prefix('/')
            .and_then(|path_without_slash| path_without_slash.strip_prefix(prefix))
    }
}

fn normalize_header_prefix(prefix: &str) -> String {
    let trimmed = prefix.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed.trim_start_matches('/').to_string()
}

fn rewrite_path(route: &AiProxyRouteConfig, uri: &Uri) -> Option<String> {
    let rewrite_base = route.rewrite_base_path.trim();
    if rewrite_base.is_empty() {
        return None;
    }
    let suffix = route_prefix_suffix(&route.path_prefix, uri.path()).unwrap_or(uri.path());
    let suffix = if suffix.is_empty() {
        "/"
    } else if suffix.starts_with('/') {
        suffix
    } else {
        return None;
    };
    let base = rewrite_base.trim_end_matches('/');
    let mut path = format!("{base}{suffix}");
    if let Some(query) = uri.query() {
        path.push('?');
        path.push_str(query);
    }
    Some(path)
}

#[cfg(test)]
pub fn provider_presets() -> Vec<AiProxyRouteConfig> {
    vec![
        AiProxyRouteConfig {
            name: "new-api".to_string(),
            provider: "new-api".to_string(),
            match_host: "ai.local".to_string(),
            path_prefix: "/v1".to_string(),
            upstream: "http://127.0.0.1:3000".to_string(),
            rewrite_base_path: "/v1".to_string(),
            add_headers: BTreeMap::from([(
                "proxysss-ai-profile".to_string(),
                "new-api".to_string(),
            )]),
            strip_headers: Vec::new(),
            forward_headers: true,
            emit_metadata_headers: true,
        },
        AiProxyRouteConfig {
            name: "sub2api".to_string(),
            provider: "sub2api".to_string(),
            match_host: "sub2ai.local".to_string(),
            path_prefix: "/".to_string(),
            upstream: "http://127.0.0.1:3001".to_string(),
            rewrite_base_path: "/v1".to_string(),
            add_headers: BTreeMap::from([(
                "proxysss-ai-profile".to_string(),
                "sub2api".to_string(),
            )]),
            strip_headers: Vec::new(),
            forward_headers: true,
            emit_metadata_headers: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_new_api_host_and_path() {
        let route = provider_presets()[0].clone();
        assert!(route_matches(&route, "ai.local", "/v1/chat/completions"));
        assert!(!route_matches(
            &route,
            "other.local",
            "/v1/chat/completions"
        ));
    }

    #[test]
    fn rewrite_sub2api_to_upstream_v1() {
        let route = provider_presets()[1].clone();
        let uri: Uri = "/api/chat".parse().unwrap();
        let decision = build_route_decision(&route, &uri, "proxysss-");
        assert_eq!(decision.rewrite_path.as_deref(), Some("/v1/api/chat"));
        assert_eq!(
            decision
                .set_headers
                .get("proxysss-ai-provider")
                .map(String::as_str),
            Some("sub2api")
        );
    }

    #[test]
    fn metadata_headers_can_be_disabled_for_nginx_parity() {
        let mut route = provider_presets()[0].clone();
        route.emit_metadata_headers = false;
        let uri: Uri = "/v1/chat/completions".parse().unwrap();

        let decision = build_route_decision(&route, &uri, "proxysss-");

        assert_eq!(
            decision.rewrite_path.as_deref(),
            Some("/v1/chat/completions")
        );
        assert!(!decision.set_headers.contains_key("proxysss-ai-provider"));
        assert!(!decision.set_headers.contains_key("proxysss-ai-route"));
        assert!(!decision
            .set_headers
            .contains_key("proxysss-ai-original-path"));
    }
}
