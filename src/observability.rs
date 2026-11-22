//! OpenTelemetry performance observability (config-gated).
//!
//! Exports traces and metrics over OTLP/HTTP (reusing the reqwest stack). The
//! whole layer is opt-in via `observability.enabled` and is never initialized
//! for benchmarks: `proxysss bench` does not call this path, and setting
//! `PROXYSSS_DISABLE_TELEMETRY=1` forces it off for load-test runs.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Result};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::{MetricExporter, SpanExporter, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::runtime;
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler, SdkTracerProvider};
use opentelemetry_sdk::Resource;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

use crate::config::{ObservabilityConfig, OtelProtocol};

/// Keeps the SDK providers alive for the process lifetime and flushes on drop.
pub struct TelemetryGuard {
    tracer_provider: Option<SdkTracerProvider>,
    meter_provider: Option<SdkMeterProvider>,
}

impl TelemetryGuard {
    fn inert() -> Self {
        Self {
            tracer_provider: None,
            meter_provider: None,
        }
    }
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.tracer_provider.take() {
            let _ = provider.shutdown();
        }
        if let Some(provider) = self.meter_provider.take() {
            let _ = provider.shutdown();
        }
    }
}

/// Whether telemetry is force-disabled by the environment (used for benchmarks).
pub fn telemetry_disabled_by_env() -> bool {
    matches!(
        std::env::var("PROXYSSS_DISABLE_TELEMETRY").as_deref(),
        Ok("1") | Ok("true") | Ok("yes") | Ok("on")
    )
}

fn resolve_env(value: &str) -> String {
    // Expand ${VAR} references so secrets can stay out of the YAML file.
    let mut out = String::new();
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find('}') {
            let key = &after[..end];
            out.push_str(&std::env::var(key).unwrap_or_default());
            rest = &after[end + 1..];
        } else {
            out.push_str("${");
            rest = after;
        }
    }
    out.push_str(rest);
    out
}

fn build_resource(config: &ObservabilityConfig) -> Resource {
    let mut attrs = vec![
        KeyValue::new("service.version", config.service_version.clone()),
        KeyValue::new("deployment.environment.name", config.environment.clone()),
        KeyValue::new("telemetry.sdk.name", "opentelemetry"),
        KeyValue::new("telemetry.sdk.language", "rust"),
    ];
    for (key, value) in &config.resource_attributes {
        attrs.push(KeyValue::new(key.clone(), resolve_env(value)));
    }
    Resource::builder()
        .with_service_name(config.service_name.clone())
        .with_attributes(attrs)
        .build()
}

fn http_headers(config: &ObservabilityConfig) -> HashMap<String, String> {
    config
        .headers
        .iter()
        .map(|(key, value)| (key.clone(), resolve_env(value)))
        .collect()
}

/// Build the optional OpenTelemetry tracing layer plus its lifetime guard.
///
/// Returns `(None, inert guard)` when disabled, force-off via env, or when the
/// (reserved) gRPC protocol is selected. The returned layer is boxed so it can
/// be composed into the logging registry uniformly with the other layers.
pub fn layer<S>(
    config: &ObservabilityConfig,
) -> Result<(
    Option<Box<dyn Layer<S> + Send + Sync>>,
    TelemetryGuard,
)>
where
    S: tracing::Subscriber + for<'span> LookupSpan<'span> + Send + Sync + 'static,
{
    if !config.enabled || telemetry_disabled_by_env() {
        return Ok((None, TelemetryGuard::inert()));
    }
    if config.protocol == OtelProtocol::Grpc {
        anyhow::bail!("observability.protocol=grpc is reserved but not yet implemented; use http");
    }

    let resource = build_resource(config);
    let timeout = Duration::from_secs(config.timeout_secs.max(1));
    let interval = Duration::from_secs(config.export_interval_secs.max(1));
    let headers = http_headers(config);

    let tracer_provider = if config.traces_enabled {
        let exporter = SpanExporter::builder()
            .with_http()
            .with_endpoint(config.endpoint.clone())
            .with_timeout(timeout)
            .with_headers(headers.clone())
            .build()
            .context("failed to build OTLP span exporter")?;
        let provider = SdkTracerProvider::builder()
            .with_batch_exporter(exporter, runtime::Tokio)
            .with_sampler(Sampler::TraceIdRatioBased(config.sample_ratio.clamp(0.0, 1.0)))
            .with_id_generator(RandomIdGenerator::default())
            .with_resource(resource.clone())
            .build();
        Some(provider)
    } else {
        None
    };

    let meter_provider = if config.metrics_enabled {
        let exporter = MetricExporter::builder()
            .with_http()
            .with_endpoint(config.endpoint.clone())
            .with_timeout(timeout)
            .with_headers(headers)
            .build()
            .context("failed to build OTLP metric exporter")?;
        let reader = PeriodicReader::builder(exporter, runtime::Tokio)
            .with_interval(interval)
            .build();
        let provider = SdkMeterProvider::builder()
            .with_reader(reader)
            .with_resource(resource)
            .build();
        Some(provider)
    } else {
        None
    };

    let otel_layer = tracer_provider.as_ref().map(|provider| {
        let tracer = provider.tracer("proxysss");
        tracing_opentelemetry::layer()
            .with_tracer(tracer)
            .with_filter(tracing_subscriber::filter::LevelFilter::INFO)
            .boxed()
    });

    Ok((
        otel_layer,
        TelemetryGuard {
            tracer_provider,
            meter_provider,
        },
    ))
}
