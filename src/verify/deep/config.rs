use anyhow::Result;

use crate::config::GatewayConfig;
use crate::verify::harness::{cleanup, temp_root, write_config};

#[tokio::test]
async fn integration_deep_legacy_include_config_rejected() -> Result<()> {
    let root = temp_root("proxysss-deep-legacy");
    let config_path = write_config(
        &root,
        "include:\n  enabled: true\n  files:\n    - ./extra.yaml\n",
    )?;
    let error = GatewayConfig::load(&config_path).expect_err("legacy include must fail");
    assert!(error.to_string().contains("legacy config"));
    cleanup(&root);
    Ok(())
}

#[test]
fn integration_deep_config_validation_matrix() {
    let mut config = GatewayConfig::default();
    assert!(config.validate().is_ok());

    config.http.plain_bind.clear();
    config.http.tls_bind.clear();
    config.http.h3_bind.clear();
    assert!(config.validate().is_err());

    config = GatewayConfig::default();
    config.load_balance.retries.max_retries = 32;
    assert!(config.validate().is_err());
}
