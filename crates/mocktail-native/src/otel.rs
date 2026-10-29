//! OpenTelemetry initialization for Mocktail.
//!
//! Configures OTLP tracing (push) and OTLP metrics (push).

use opentelemetry::{global, metrics::Counter, KeyValue};
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::Resource;
use tracing_opentelemetry::OpenTelemetryLayer;
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};

pub struct OtelHandle {
    pub http_requests: Counter<u64>,
}

// We use the concrete types for the layer to avoid complex generics in the return type.
type OtelLayer = OpenTelemetryLayer<tracing_subscriber::Registry, opentelemetry_sdk::trace::SdkTracer>;

pub fn init(service_name: &str, cfg: &crate::config::OtelConfig) -> (OtelHandle, Option<OtelLayer>) {
    if !cfg.enabled {
        let meter = global::meter("mocktail");
        let http_requests = meter
            .u64_counter("http_requests_total")
            .with_description("Total number of HTTP requests")
            .build();
            
        return (OtelHandle { http_requests }, None);
    }

    let resource = Resource::builder()
        .with_service_name(service_name.to_string())
        .with_attribute(KeyValue::new("deployment.environment", cfg.environment.clone()))
        .build();

    let endpoint = if cfg.endpoint.starts_with("http") {
        cfg.endpoint.clone()
    } else {
        let scheme = if cfg.insecure { "http://" } else { "https://" };
        format!("{}{}", scheme, cfg.endpoint)
    };

    // 1. Trace Initialization (OTLP Push)
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint.clone())
        .build()
        .expect("failed to build OTLP span exporter");

    let tracer_provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource.clone())
        .build();

    global::set_tracer_provider(tracer_provider.clone());
    let tracer = tracer_provider.tracer("mocktail");

    // 2. Metrics Initialization (OTLP Push)
    let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .expect("failed to build OTLP metric exporter");

    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(metric_exporter)
        .build();

    let meter_provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource)
        .build();

    global::set_meter_provider(meter_provider);
    let meter = global::meter("mocktail");
    
    let http_requests = meter
        .u64_counter("http_requests_total")
        .with_description("Total number of HTTP requests")
        .build();

    // 3. Tracing Layer for tracing-subscriber
    let tracing_layer = OpenTelemetryLayer::new(tracer);

    (
        OtelHandle { http_requests },
        Some(tracing_layer),
    )
}

pub fn get_metrics() -> String {
    "OTLP push enabled. Metrics are no longer served via /metrics endpoint.".to_string()
}
