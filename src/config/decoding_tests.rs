use super::*;

#[test]
fn unknown_configuration_fields_fail_at_root_and_nested_boundaries() {
    // Exercise user-facing decoding, including disabled services and list entries.
    // Misspellings must fail before validation can hide them behind defaults.
    for yaml in [
        "typo: true",
        "logging: {typo: true}",
        "http: {typo: true}",
        "http: {tls: {typo: true}}",
        "http: {tls: {acme: {dns: {typo: true}}}}",
        "http: {error_pages: {pages: [{status: 404, typo: true}]}}",
        "tcp: {listeners: [{name: game, bind: '127.0.0.1:9000', typo: true}]}",
        "udp: {listeners: [{name: game, bind: '127.0.0.1:9000', typo: true}]}",
        "script: {typo: true}",
        "plugins: {typo: true}",
        "admin: {monitor: {typo: true}}",
        "security: {typo: true}",
        "kubernetes: {typo: true}",
        "runtime: {performance: {typo: true}}",
        "runtime: {watchdog: {typo: true}}",
        "runtime: {hot_reload: {typo: true}}",
        "load_balance: {active_health: {typo: true}}",
        "services: {reverse_proxy: {routes: [{typo: true}]}}",
        "services: {domain_routes: [{typo: true}]}",
        "services: {ai_proxy: {typo: true}}",
        "services: {ai_proxy: {routes: [{typo: true}]}}",
        "services: {static_sites: [{typo: true}]}",
        "services: {response_policy: {cache: {typo: true}}}",
        "services: {rate_limit: {typo: true}}",
        "services: {webdav: {auth: {typo: true}}}",
        "services: {filecloud: {typo: true}}",
        "services: {ftp: {typo: true}}",
        "services: {service_discovery: {typo: true}}",
    ] {
        let error = serde_yaml::from_str::<GatewayConfig>(yaml).unwrap_err();
        assert!(
            error.to_string().contains("unknown field `typo`"),
            "{yaml}: {error}"
        );
    }
}

#[test]
fn strict_configuration_preserves_aliases_and_open_maps() {
    let config: GatewayConfig = serde_yaml::from_str(
        r#"
script:
  env: {GAME_REGION: apac, arbitrary_key: retained}
http:
  tls:
    acme:
      dns:
        credentials: {custom_vendor_key: retained}
services:
  access_control:
    http:
      whitelist: [127.0.0.1]
      blacklist: [192.0.2.1]
  reverse_proxy:
    routes:
      - name: api
        path_prefix: /api
        upstream: http://127.0.0.1:9000
        add_headers: {X-Game-Region: apac}
        remove_headers: [X-Private]
        upstream_weights: {'http://127.0.0.1:9000': 3}
  ftp:
    proxy_pass: 127.0.0.1:2121
"#,
    )
    .unwrap();
    config.validate().unwrap();
    let effective = serde_yaml::to_value(&config).unwrap();
    assert_eq!(effective["script"]["env"]["arbitrary_key"], "retained");
    assert_eq!(
        effective["http"]["tls"]["acme"]["dns"]["credentials"]["custom_vendor_key"],
        "retained"
    );
    assert_eq!(
        effective["services"]["access_control"]["http"]["allow"][0],
        "127.0.0.1"
    );
    assert_eq!(
        effective["services"]["access_control"]["http"]["deny"][0],
        "192.0.2.1"
    );
    let route = &effective["services"]["reverse_proxy"]["routes"][0];
    assert_eq!(route["set_headers"]["X-Game-Region"], "apac");
    assert_eq!(route["strip_headers"][0], "X-Private");
    assert_eq!(route["upstream_weights"]["http://127.0.0.1:9000"], 3);
    assert_eq!(config.services.ftp.upstream, "127.0.0.1:2121");
}

#[test]
fn config_file_error_exposes_the_unknown_field_to_callers() {
    let root = crate::verify::harness::temp_root("strict-config-error");
    let path =
        crate::verify::harness::write_config(&root, "admin: {enable_write_op: true}").unwrap();
    let error = GatewayConfig::load(&path).unwrap_err().to_string();
    assert!(error.contains("enable_write_op"), "{error}");
    assert!(error.contains(&path.display().to_string()), "{error}");
    crate::verify::harness::cleanup(&root);
}
