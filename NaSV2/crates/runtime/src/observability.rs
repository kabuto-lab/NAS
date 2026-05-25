//! Observability bootstrap · ENTITY §4
//!
//! Extracted from `apps/server/src/main.rs` (P1 S1 of AVTONOM 2026-05-25).
//!
//! Layered registry:
//! 1. JSON `fmt` layer + `EnvFilter` (always-on)
//! 2. OTLP exporter layer (active iff `OTLP_ENDPOINT` env is set)
//! 3. Prometheus HTTP exporter (active iff `PROMETHEUS_METRICS_PORT` env is set,
//!    default `9000` when [`init_with_prometheus`] is called explicitly)
//!
//! TLA layers:
//! - **L1 Correctness** — `EnvFilter` falls back to `info` so misconfiguration
//!   never silently drops logs.
//! - **L2 Performance** — single registry init; no per-request allocation.
//! - **L3 Scalability** — OTLP uses tonic/grpc with batch span processor.
//! - **L4 Operability** — exporters announce themselves via `tracing::info!`
//!   so deploy pipeline can grep for them.

use std::net::SocketAddr;

use eyre::{Result, WrapErr};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace::TracerProvider, Resource};
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

/// Default Prometheus listener port (ENTITY §4 — separate from the API port
/// so a misbehaving scraper cannot saturate request-handling capacity).
pub const DEFAULT_PROMETHEUS_PORT: u16 = 9000;

/// Boot all observability layers in one call.
///
/// Returns a guard whose `Drop` flushes any pending OTLP spans. Hold it in
/// `main` for the lifetime of the process.
///
/// # Errors
/// Returns an error if the tracing subscriber fails to install, the OTLP
/// exporter cannot be built, or the Prometheus listener cannot bind.
// `tracing::info!`/`debug!` macros + optional-layer chain inflate the score;
// the body is a flat sequence with no branching beyond env presence.
#[allow(clippy::cognitive_complexity)]
pub fn init(service_name: &'static str) -> Result<ObservabilityGuard> {
    let env_filter = EnvFilter::try_from_env("LOG_LEVEL")
        .or_else(|_| EnvFilter::try_new("info"))
        .wrap_err("EnvFilter init")?;

    let fmt_layer = fmt::layer().json();

    let otlp_provider = if let Ok(endpoint) = std::env::var("OTLP_ENDPOINT") {
        Some(build_otlp_provider(service_name, &endpoint)?)
    } else {
        tracing::debug!("OTLP_ENDPOINT not set — skipping OTLP exporter");
        None
    };

    let otel_layer = otlp_provider.as_ref().map(|provider| {
        let tracer = provider.tracer(service_name);
        tracing_opentelemetry::layer().with_tracer(tracer)
    });

    Registry::default()
        .with(env_filter)
        .with(fmt_layer)
        .with(otel_layer)
        .try_init()
        .wrap_err("tracing subscriber init")?;

    install_prometheus_if_configured()?;

    tracing::info!(service = service_name, "observability initialized");
    Ok(ObservabilityGuard {
        otlp: otlp_provider,
    })
}

fn build_otlp_provider(service_name: &'static str, endpoint: &str) -> Result<TracerProvider> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .wrap_err("OTLP exporter build")?;
    let provider = TracerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_resource(Resource::new(vec![opentelemetry::KeyValue::new(
            "service.name",
            service_name,
        )]))
        .build();
    tracing::info!(endpoint, "OTLP exporter configured");
    Ok(provider)
}

fn install_prometheus_if_configured() -> Result<()> {
    let Some(port_raw) = std::env::var("PROMETHEUS_METRICS_PORT").ok() else {
        tracing::debug!("PROMETHEUS_METRICS_PORT not set — skipping Prometheus exporter");
        return Ok(());
    };
    let port: u16 = port_raw.parse().wrap_err("PROMETHEUS_METRICS_PORT parse")?;
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()
        .wrap_err("prometheus install")?;
    tracing::info!(%addr, "prometheus exporter listening");
    Ok(())
}

/// Holds the OTLP tracer provider so its drop flushes pending spans.
pub struct ObservabilityGuard {
    otlp: Option<TracerProvider>,
}

impl Drop for ObservabilityGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.otlp.take() {
            if let Err(e) = provider.shutdown() {
                eprintln!("OTLP provider shutdown error: {e:?}");
            }
        }
    }
}
