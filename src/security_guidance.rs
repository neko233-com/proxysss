//! Read-only, secret-free configuration advice for operators and agents.
use serde::Serialize;
use serde_json::Value;

use crate::config::{GatewayConfig, StaticSiteConfig};

#[derive(Serialize)]
struct Control {
    path: String,
    current: Value,
    default: Value,
    switch: String,
    recommendation: String,
}

fn value_at(value: &Value, path: &str) -> Value {
    path.split('.')
        .try_fold(value, |value, part| value.get(part))
        .cloned()
        .unwrap_or(Value::Null)
}

pub fn report(config: &GatewayConfig) -> anyhow::Result<String> {
    let current = serde_json::to_value(config)?;
    let defaults = serde_json::to_value(GatewayConfig::default())?;
    let mut controls = Vec::new();
    let mut add = |path: &str, switch: &str, recommendation: &str| {
        controls.push(Control {
            path: path.into(),
            current: value_at(&current, path),
            default: value_at(&defaults, path),
            switch: switch.into(),
            recommendation: recommendation.into(),
        });
    };
    for (path, recommendation) in [
        ("security.validate_admin_mutations", "生产保持 true；对管理 API 的路由和上游变更做输入校验。"),
        ("security.block_ssrf_targets", "生产保持 true；按 blocked_upstream_hosts/CIDRs 约束管理 API 上游目标；不是 DNS 重绑定的完整防线。"),
        ("security.reject_ambiguous_http1", "保持 true；拒绝额外的 HTTP/1 歧义组合。关闭不绕过底层 HTTP 解析器的协议校验。"),
        ("security.ddos.enabled", "公网入口可开启；先按真实连接峰值设置窗口、burst 和封禁时间，CDN 出口共享 IP 会合并计数。"),
        ("security.dynamic_blacklist.enabled", "需要运行时封禁时开启；保护黑名单文件写权限，管理写接口仍需认证。"),
        ("admin.enabled", "只在运维需要时开启；默认绑定 loopback:7777。"),
        ("admin.loopback_only", "本地管理保持 true；远程优先使用限定 hosts 的 HTTPS 管理入口。"),
        ("admin.expose_config", "保持 false；确需自动化读配置时再开启，敏感字段仍脱敏。"),
        ("admin.enable_write_ops", "保持 false；授权自动化写配置时开启并配置独立强令牌。"),
        ("admin.https.enabled", "公网管理按需开启，配置 hosts 并使用受信 TLS；不要公开默认管理凭据。"),
        ("admin.auth_rate_limit.enabled", "保持 true；降低暴力尝试，阈值需兼顾多人共用出口。"),
        ("http.allow_insecure_upstreams", "生产 HTTPS/WSS 上游建议 false，校验证书；自签测试环境才使用 true。"),
        ("http.tls.on_demand.enabled", "只在动态域名场景开启，同时设置 allow/ask_url 和签发速率上限。"),
        ("services.access_control.http.enabled", "内网 API 或受控入口开启；allow/deny 使用真实连接 IP，不把可伪造 header 当身份。"),
        ("services.access_control.stream.enabled", "数据库、MQTT、游戏运维端口建议限定对端 IP/CIDR。"),
        ("services.rate_limit.http.enabled", "公网按容量启用；先观测业务峰值再选择 token_bucket 等算法，避免误伤共享出口。"),
        ("services.rate_limit.stream.enabled", "公网长连接按握手峰值启用；不能代替运营商的链路 DDoS 清洗。"),
        ("services.webdav.enabled", "按需开启；使用独立目录与访问控制。"),
        ("services.webdav.allow_write", "只读分发设置 false；开启写入前另行配置入口认证/ACL。"),
        ("services.filecloud.enabled", "按需开启，配置非空强密码并使用 HTTPS。"),
        ("services.filecloud.require_auth_for_download", "私有下载设 true；公开 CDN 下载才设 false。"),
        ("services.filecloud.allow_upload", "无上传需求设 false，并限制 max_upload_bytes。"),
        ("services.filecloud.allow_delete", "默认部署前审查；仅在确需远程删除时开启。"),
        ("services.filecloud.allow_move", "仅在确需远程改名/移动时开启。"),
        ("services.filecloud.allow_mkdir", "只读资源站设 false。"),
        ("services.ftp.enabled", "按需开启；FTP 明文链路应限于可信网络，优先使用加密的文件服务。"),
        ("services.ftp.access_control_enabled", "有 allow/deny 规则时保持 true；关闭保留规则但不执行 FTP IP 限制。"),
        ("services.ftp.command_policy_enabled", "保持 true 并显式允许需要的 FTP 命令；false 同时跳过该类用户命令规则。"),
        ("services.ftp.transfer_policy_enabled", "保持 true；只读账号仅允许下载相关传输命令。false 同时跳过用户传输规则。"),
        ("services.ftp.user_policy_enabled", "配置了 user_policies 时保持 true；关闭只跳过按用户规则，全局命令/传输策略仍执行。"),
        ("services.response_policy.cache.allow_purge", "公网缓存建议 false；需要 PURGE 时通过 ACL 或受控路由保护。"),
        ("script.enabled", "有业务扩展需求时启用；脚本目录属于可信运维边界。"),
        ("plugins.enabled", "只加载可信插件；普通网关不需要为防盗刷强制调用脚本。"),
        ("plugins.allow_admin_manage", "不需要远程管理插件时设 false；开启后仍需管理员认证和对应写权限。"),
    ] { add(path, "true 开启，false 关闭；所属服务未启用时不执行", recommendation); }
    for (path, switch, recommendation) in [
        (
            "http.request_timeout_ms",
            "必须大于 0；按毫秒设置",
            "按 API/上传/SSE 场景设置；过短会中断合法长请求。",
        ),
        (
            "security.ddos.max_connections",
            "由 security.ddos.enabled 控制",
            "按连接建立速率设定，不是总存量连接上限。",
        ),
        (
            "security.ddos.window_secs",
            "由 security.ddos.enabled 控制",
            "结合 max_connections 和 burst 设定检测窗口。",
        ),
        (
            "security.ddos.ban_secs",
            "由 security.ddos.enabled 控制",
            "先使用短封禁，观察误封率后调整。",
        ),
        (
            "services.rate_limit.http.max_connections",
            "0 关闭该上限；正数限制并发连接",
            "流式响应/长连接要按真实并发预算配置，HTTP 限流总开关须启用。",
        ),
        (
            "services.ftp.max_login_attempts",
            "0 不设上限；正数限制单连接登录失败次数",
            "公网 FTP 建议非零，并结合上游认证防护。",
        ),
        (
            "services.ftp.limit_rate",
            "0 不限速；正数为字节/秒",
            "依据带宽预算控制传输；不能代替每用户的下载授权。",
        ),
        (
            "services.filecloud.max_upload_bytes",
            "正数设定上传上限；无上传需求关闭 allow_upload",
            "按最大合法文件大小设置，避免恶意占用磁盘。",
        ),
        (
            "services.filecloud.session_ttl_secs",
            "由 filecloud 服务和密码认证控制",
            "私有资源使用较短会话并配合 HTTPS。",
        ),
        (
            "security.ddos.burst",
            "由 security.ddos.enabled 控制",
            "为短时合法突发留出余量，不盲目放大封禁阈值。",
        ),
        (
            "admin.auth_rate_limit.max_failures",
            "由 admin.auth_rate_limit.enabled 控制",
            "按共享出口的管理员人数设置失败次数。",
        ),
        (
            "admin.auth_rate_limit.window_secs",
            "由 admin.auth_rate_limit.enabled 控制",
            "结合失败次数定义暴力尝试检测窗口。",
        ),
        (
            "admin.auth_rate_limit.lockout_secs",
            "由 admin.auth_rate_limit.enabled 控制",
            "先设置有限锁定时间，避免长期误封运维入口。",
        ),
        (
            "script.timeout_ms",
            "由 script.enabled 控制，保留执行超时",
            "按脚本最大合法耗时设置；防止无限循环占用执行线程。",
        ),
        (
            "script.memory_limit_mb",
            "由 script.enabled 控制，保留内存上限",
            "给可信脚本合理的内存预算，避免插件无限增长。",
        ),
        (
            "script.max_stack_size_kb",
            "由 script.enabled 控制，保留栈上限",
            "限制递归深度；不通过关闭资源边界提升脚本性能。",
        ),
    ] {
        add(path, switch, recommendation);
    }

    for site in &config.services.static_sites {
        let prefix = format!("services.static_sites[{}]", site.name);
        let current = serde_json::to_value(site)?;
        let default = serde_json::to_value(StaticSiteConfig::default())?;
        for (field, recommendation) in [
            ("security.enabled", "站点鉴权总开关；false 会同时停用回源令牌、对端白名单与 URL 签名，站点限流仍独立执行。"),
            ("security.origin_token_enabled", "CDN 回源建议 true 且配置独立强令牌；空令牌表示未配置认证。"),
            ("security.allowed_peers_enabled", "CDN 提供稳定回源 CIDR 时开启并维护列表；空列表不限制对端。"),
            ("security.signed_url.enabled", "源站直接交付私有文件时开启；CDN 缓存命中的访客授权仍在 CDN 边缘验证。"),
            ("rate_limit.enabled", "为该站点覆盖全局限流参数；关闭此处不会关闭已开启的全局 HTTP 限流。"),
            ("autoindex", "默认关闭；仅为公开下载目录显式开启，不能与生效的签名鉴权同时开启。"),
            ("hide_dotfiles", "保持 true，隐藏 .env/.git 等；关闭会使根目录内点文件可下载。"),
        ] {
            controls.push(Control { path: format!("{prefix}.{field}"), current: value_at(&current, field), default: value_at(&default, field), switch: "true 开启，false 关闭；保留其他参数".into(), recommendation: recommendation.into() });
        }
        controls.push(Control {
            path: format!("{prefix}.security.effective"),
            current: serde_json::json!({ "origin_token": crate::static_security::token_enabled(site), "peer_allowlist": site.security.enabled && site.security.allowed_peers_enabled && !site.security.allowed_peers.is_empty(), "signed_url": crate::static_security::signed_url_enabled(site) }),
            default: serde_json::json!({"origin_token": false, "peer_allowlist": false, "signed_url": false}),
            switch: "只读计算结果".into(), recommendation: "只显示实际生效状态，不输出密钥。".into(),
        });
    }

    // Route policies also have independent switches. Never serialize upstream credentials.
    for (group, routes) in [
        (
            "reverse_proxy.routes",
            current["services"]["reverse_proxy"]["routes"].as_array(),
        ),
        (
            "domain_routes",
            current["services"]["domain_routes"].as_array(),
        ),
    ] {
        for route in routes.into_iter().flatten() {
            let name = route
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unnamed");
            for field in [
                "access_control.enabled",
                "rate_limit.enabled",
                "cache.allow_purge",
            ] {
                let value = value_at(route, field);
                if value.is_null() {
                    continue;
                }
                controls.push(Control {
                    path: format!("services.{group}[{name}].{field}"),
                    current: value,
                    default: Value::Null,
                    switch: "按路由配置；关闭覆盖项仍可能继承全局策略".into(),
                    recommendation: "检查全局与路由的合并结果，PURGE 仅用于受控入口。".into(),
                });
            }
        }
    }
    for route in &config.tcp.stream_routes {
        controls.push(Control {
            path: format!("tcp.stream_routes[{}].access_control.enabled", route.name),
            current: serde_json::json!(route.access_control.enabled),
            default: Value::Bool(false),
            switch: "true/false".into(),
            recommendation: "SNI 数据库/MQTT 入口建议限定真实对端；全局 stream ACL 仍可能执行。"
                .into(),
        });
    }
    #[derive(Serialize)]
    struct Report {
        controls: Vec<Control>,
        notes: Vec<&'static str>,
    }
    Ok(serde_yaml::to_string(&Report { controls, notes: vec![
        "建议不会自动修改 YAML；默认值用于兼容通用网关场景，不等同于适用于每个公网部署的安全基线。",
        "管理 API 必须认证；没有跳过认证的总开关。配置显示只输出开关和数值，不输出令牌、密码、私钥。",
        "路径 confinement、HTTP 解析正确性、HTML 转义和内存边界属于不可关闭的实现约束。",
        "安全开关不是为所有服务同时关闭认证的总开关；逐服务、逐功能修改并保留审计。",
        "security.mac_deny 从未接入数据路径，现在非空配置明确报错；请使用 IP/CIDR 或主机 L2 防火墙。",
        "Referer/User-Agent/bot-score、CSRF 与业务身份规则由应用、CDN WAF 或脚本代理路由负责；不声称内置完整规则库。",
        "静态站点按 YAML 顺序匹配；具体路径在 / 前，私有数据不能位于公共根目录。",
        "多数安全策略在热重载后供新请求/连接使用；admin.enabled/bind、TLS mode、监听身份及性能平台参数需重启。",
    ] })?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn guidance_has_switches_defaults_and_never_prints_secrets() {
        let mut config = GatewayConfig::default();
        config.admin.password = "secret-admin-password".into();
        config.services.filecloud.password = "secret-filecloud-password".into();
        let mut site = StaticSiteConfig::default();
        site.security.origin_token = "secret-origin-token-with-enough-length".into();
        site.security.signed_url.secret = "secret-signing-key-with-enough-length".into();
        config.services.static_sites.push(site);
        let rendered = report(&config).unwrap();
        assert!(!rendered.contains("secret-"));
        for key in [
            "default:",
            "switch:",
            "recommendation:",
            "http.allow_insecure_upstreams",
            "services.ftp.command_policy_enabled",
            "security.effective",
        ] {
            assert!(rendered.contains(key), "{key}");
        }
        serde_yaml::from_str::<serde_yaml::Value>(&rendered).unwrap();
    }

    #[test]
    fn security_defaults_match_omitted_and_partial_yaml_sections() {
        for yaml in [
            "{}",
            "http: {}\nplugins: {}",
            "http:\n  request_timeout_ms: 5000\nplugins:\n  allow_admin_manage: false",
        ] {
            let config: GatewayConfig = serde_yaml::from_str(yaml).unwrap();
            assert!(!config.http.allow_insecure_upstreams, "{yaml}");
            assert!(!config.plugins.enabled, "{yaml}");
        }
        let config: GatewayConfig = serde_yaml::from_str(
            "http:\n  allow_insecure_upstreams: true\nplugins:\n  enabled: true",
        )
        .unwrap();
        assert!(config.http.allow_insecure_upstreams);
        assert!(config.plugins.enabled);
    }
}
