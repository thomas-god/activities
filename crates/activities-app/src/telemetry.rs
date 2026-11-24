use opentelemetry::{KeyValue, trace::TracerProvider as _};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::TelemetryConfig;

const DEFAULT_SERVICE_NAME: &str = "activities-app";

/// Handle returned by [`init`], kept alive for the app's lifetime and used to flush/shutdown the
/// OTel exporter. `None` when no OTLP endpoint was configured.
pub struct TelemetryGuard {
    tracer_provider: Option<SdkTracerProvider>,
}

impl TelemetryGuard {
    pub fn shutdown(self) {
        if let Some(provider) = self.tracer_provider
            && let Err(err) = provider.shutdown()
        {
            eprintln!("Error while shutting down the OpenTelemetry tracer provider: {err}");
        }
    }
}

/// Sets up the global `tracing` subscriber: always logs to stdout, and additionally exports
/// spans over OTLP/HTTP (protobuf) when `config.otlp_endpoint` is set (see [`TelemetryConfig`]).
pub fn init(config: &TelemetryConfig) -> anyhow::Result<TelemetryGuard> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_target(false);

    let (otel_layer, tracer_provider) = match &config.otlp_endpoint {
        Some(endpoint) if !endpoint.is_empty() => {
            let service_name = config
                .service_name
                .clone()
                .unwrap_or_else(|| DEFAULT_SERVICE_NAME.to_string());

            let traces_endpoint = format!("{}/v1/traces", endpoint.trim_end_matches('/'));
            let exporter = SpanExporter::builder()
                .with_http()
                .with_endpoint(&traces_endpoint)
                .build()?;

            let resource = Resource::builder()
                .with_attributes([KeyValue::new("service.name", service_name)])
                .build();

            let provider = SdkTracerProvider::builder()
                .with_resource(resource)
                .with_batch_exporter(exporter)
                .build();
            let tracer = provider.tracer(DEFAULT_SERVICE_NAME);

            (
                Some(tracing_opentelemetry::layer().with_tracer(tracer)),
                Some(provider),
            )
        }
        _ => (None, None),
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(otel_layer)
        .try_init()?;

    if let Some(endpoint) = &config.otlp_endpoint
        && !endpoint.is_empty()
    {
        tracing::info!("OpenTelemetry tracing enabled, exporting to {endpoint}");
    }

    Ok(TelemetryGuard { tracer_provider })
}
