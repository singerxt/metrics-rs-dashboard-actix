//! # Metrics Module
//!
//! This module provides comprehensive Prometheus metrics integration for Actix web applications.
//! It enables robust monitoring capabilities with automatic metric collection, exposition,
//! and visualization through an integrated dashboard.
//!
//! ## Features
//! - **Prometheus Integration**: Full support for collecting and exposing metrics in Prometheus format
//! - **Interactive Dashboard**: Built-in web UI for visualizing metrics in real-time
//! - **Customizable Histograms**: Fine-grained control over histogram bucket configuration
//! - **Easy Integration**: Seamlessly integrates with Actix web applications via a simple API
//! - **Thread-Safe**: Designed for concurrent access with proper synchronization
//! - **Low Overhead**: Minimal performance impact on your application
//!
//! ## Architecture
//! The module uses a multi-recorder approach with a fanout pattern to capture both metric values
//! and their associated metadata (like units). This information is then made available both in
//! Prometheus format for scraping and through a dashboard for human-readable visualization.
//!
//! ## Getting Started
//! Simply add the metrics scope to your Actix application as shown in the examples below.

/// Re-export of the `metrics` crate for measuring and recording application metrics
pub use metrics;
use metrics::{Counter, CounterFn, Gauge, GaugeFn, Histogram, HistogramFn, Key, Recorder, Unit};
/// Re-export of the `metrics_exporter_prometheus` crate for exposing metrics in Prometheus format
pub use metrics_exporter_prometheus;
/// Re-export of the `metrics_util` crate for utility functions related to metrics
pub use metrics_util;

use actix_web::{HttpResponse, Responder, Scope, web};
use anyhow::Result;
use log::debug;
use log_once::debug_once;
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use metrics_util::layers::FanoutBuilder;
use mime_guess::from_path;
use rust_embed::Embed;
use std::{collections::HashMap, sync::{atomic::{AtomicBool, Ordering}, Arc, Mutex, OnceLock}};

/// Global flag to track if metrics recorders have been configured
static IS_CONFIGURED: AtomicBool = AtomicBool::new(false);

/// Global Prometheus recorder instance
static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Global storage for metric unit information
///
/// Maps metric names to their corresponding units, which is used
/// by the dashboard to correctly display unit information in charts
static UNITS_FOR_METRICS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

/// Embedded assets for the metrics dashboard
#[derive(Embed)]
#[folder = "public/"]
struct Asset;

/// Configuration options for the metrics dashboard
#[derive(Debug, Clone, Default)]
pub struct DashboardInput<'a> {
    /// Custom set of buckets for histogram metrics.
    ///
    /// Each tuple contains:
    /// - A `Matcher` to identify which metrics should use these buckets
    /// - A slice of f64 values representing the bucket boundaries
    ///
    /// This allows fine-tuning the histogram resolution for specific metrics.
    /// For example, setting different bucket ranges for latency metrics vs.
    /// memory usage metrics.
    ///
    /// # Example
    /// ```
    /// use metrics_exporter_prometheus::Matcher;
    /// let latency_buckets = &[0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0];
    /// let buckets = vec![(Matcher::Full("http_request_duration".to_string()), latency_buckets)];
    /// ```
    pub buckets_for_metrics: Vec<(Matcher, &'a [f64])>,
}

/// The UnitRecorder captures unit metadata from metrics registrations
///
/// This recorder doesn't actually record metric values - it only stores the
/// unit information associated with each metric in a global map. This information
/// is later used by the dashboard to correctly label and scale visualizations.
///
/// The unit information is sent to the client via a custom HTTP header when
/// metrics are requested from the dashboard.
///
/// Format of header: x-dashboard-metrics-unit: {"request_latency":"count","request_latency_gauge":"count","async_counter":"count","async_gauge":"milliseconds"}

#[derive(Debug)]
struct UnitRecorder;

/// Handle for the UnitRecorder
///
/// This is a no-op implementation that just stores the metric key
/// but doesn't actually record any values.
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct UnitRecorderHandle(Key);

impl CounterFn for UnitRecorderHandle {
    fn increment(&self, _value: u64) {
        // No-op
    }

    fn absolute(&self, _value: u64) {
        // No-op
    }
}

impl GaugeFn for UnitRecorderHandle {
    fn increment(&self, _value: f64) {
        // No-op
    }

    fn decrement(&self, _value: f64) {
        // No-op
    }

    fn set(&self, _value: f64) {
        // No-op
    }
}

impl HistogramFn for UnitRecorderHandle {
    fn record(&self, _value: f64) {
        // No-op
    }
}

impl Recorder for UnitRecorder {
    fn describe_counter(&self, key: metrics::KeyName, unit: Option<metrics::Unit>, _description: metrics::SharedString) {
        self.register_unit(key, unit);
    }

    fn describe_gauge(&self, key: metrics::KeyName, unit: Option<metrics::Unit>, _description: metrics::SharedString) {
        self.register_unit(key, unit);
    }

    fn describe_histogram(&self, key: metrics::KeyName, unit: Option<metrics::Unit>, _description: metrics::SharedString) {
        self.register_unit(key, unit);
    }

    fn register_counter(&self, key: &metrics::Key, _metadata: &metrics::Metadata<'_>) -> metrics::Counter {
        Counter::from_arc(Arc::new(UnitRecorderHandle(key.clone())))
    }

    fn register_gauge(&self, key: &metrics::Key, _metadata: &metrics::Metadata<'_>) -> metrics::Gauge {
        Gauge::from_arc(Arc::new(UnitRecorderHandle(key.clone())))
    }

    fn register_histogram(&self, key: &metrics::Key, _metadata: &metrics::Metadata<'_>) -> metrics::Histogram {
        Histogram::from_arc(Arc::new(UnitRecorderHandle(key.clone())))
    }
}

impl UnitRecorder {
    /// Registers a metric's unit in the global units map
    ///
    /// This method extracts the unit information from a metric registration
    /// and stores it in the global UNITS_FOR_METRICS map for later use.
    ///
    /// # Arguments
    ///
    /// * `key` - The name of the metric
    /// * `unit` - Optional unit of the metric (defaults to Count if None)
    fn register_unit(&self, key: metrics::KeyName, unit: Option<metrics::Unit>) {
        let key = key.as_str().to_owned();
        let unit = unit.unwrap_or(Unit::Count);
        let unit = unit.as_str().to_owned();
        let g_unit = UNITS_FOR_METRICS.get_or_init(|| Mutex::new(HashMap::new()));
        if let Ok(mut locked) = g_unit.lock() {
            locked.insert(key, unit);
        }
    }
}

/// Serves embedded files from the Asset struct
///
/// This helper function handles serving static files that are embedded
/// in the binary using rust-embed. It automatically sets the proper
/// content type based on file extension.
///
/// # Arguments
///
/// * `path` - Path to the file within the embedded assets
///
/// # Returns
///
/// HttpResponse containing the file content with appropriate MIME type,
/// or a 404 Not Found response if the asset doesn't exist
fn handle_embedded_file(path: &str) -> HttpResponse {
    match Asset::get(path) {
        Some(content) => HttpResponse::Ok()
            .content_type(from_path(path).first_or_octet_stream().as_ref())
            .body(content.data.into_owned()),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

/// Handler for the metrics dashboard index page
///
/// Serves the main HTML interface for the metrics dashboard.
/// This interactive dashboard provides visualizations of all
/// application metrics with auto-refreshing charts.
///
/// # Returns
///
/// The main dashboard HTML page
#[actix_web::get("/dashboard")]
async fn get_dashboard() -> impl Responder {
    handle_embedded_file("index.html")
}

/// Handler for serving dashboard assets (JS, CSS, etc.)
///
/// Handles requests for static assets needed by the dashboard UI.
/// This includes JavaScript files, stylesheets, images, and any
/// other resources required by the dashboard interface.
///
/// # Arguments
///
/// * `path` - Path to the requested asset, extracted from the URL
///
/// # Returns
///
/// The requested asset file with appropriate content type
#[actix_web::get("/dashboard/{_:.*}")]
async fn get_dashboard_assets(path: web::Path<String>) -> impl Responder {
    handle_embedded_file(path.as_str())
}

/// Endpoint for exposing Prometheus metrics
///
/// This endpoint is where Prometheus should scrape to collect metrics.
/// It returns all application metrics in the standard Prometheus text format.
/// Additionally, it includes unit information in a custom HTTP header for
/// use by the dashboard.
///
/// # Returns
///
/// Prometheus metrics in the standard text-based exposition format
/// with an additional "x-dashboard-metrics-unit" header containing
/// unit information for metrics
#[actix_web::get("/prometheus")]
async fn get_prometheus_metrics() -> impl Responder {
    debug!("Gathering prometheus metrics...");
    let prometheus_handle = PROMETHEUS_HANDLE.get();
    let metrics_units = UNITS_FOR_METRICS.get();
    let mut response = HttpResponse::Ok();

    if let Some(metrics_units) = metrics_units {
        let header = serde_json::to_string(metrics_units).unwrap_or_default();
        response.append_header(("x-dashboard-metrics-unit", header));
    }

    if let Some(handle) = prometheus_handle {
        let metrics = handle.render();
        return response.body(metrics);
    }

    HttpResponse::Ok().body(String::from(""))
}

/// Configures metrics recorders if they haven't been configured yet
///
/// This function is idempotent and safe to call multiple times.
/// Only the first call will actually configure the recorders, subsequent
/// calls will return early with success. This is achieved through thread-safe
/// synchronization using atomic operations.
///
/// The function sets up:
/// 1. A Prometheus recorder for actual metric values
/// 2. A UnitRecorder to capture unit metadata
/// 3. A FanoutBuilder to dispatch metrics to both recorders
///
/// # Arguments
///
/// * `input` - Configuration options for the metrics system, including custom histogram buckets
///
/// # Returns
///
/// Result indicating success or failure of configuration
///
/// # Errors
///
/// Returns an error if:
/// - Cannot acquire the configuration lock
/// - Failed to set custom histogram buckets
/// - Unable to set the Prometheus handle
/// - Unable to register the global recorder
fn configure_metrics_recorders_once(input: &DashboardInput) -> Result<()> {
    // Return early if already configured, using "Acquire" ordering to ensure
    // visibility of all operations performed before setting to true
    if IS_CONFIGURED.load(Ordering::Acquire) {
        debug_once!("Metrics recorder already configured. Skipping duplicate configuration.");
        return Ok(());
    }

    // Try to be the first thread to configure
    if IS_CONFIGURED.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
        // Another thread configured the metrics in the meantime
        debug_once!("Another thread configured metrics. Skipping duplicate configuration.");
        return Ok(());
    }

    let mut prometheus_recorder = PrometheusBuilder::new();

    if !input.buckets_for_metrics.is_empty() {
        for (matcher, buckets) in input.buckets_for_metrics.iter() {
            prometheus_recorder = prometheus_recorder
                .set_buckets_for_metric(matcher.to_owned(), buckets)
                .map_err(|e| anyhow::anyhow!("Failed to set buckets for metric: {}", e))?;
        }
    }

    let prometheus_recorder = prometheus_recorder
        .set_enable_unit_suffix(false)
        .build_recorder();

    PROMETHEUS_HANDLE
        .set(prometheus_recorder.handle())
        .map_err(|e| anyhow::anyhow!("Unable to set Prometheus handle: {}", e.render()))?;

    let fanout = FanoutBuilder::default()
        .add_recorder(UnitRecorder)
        .add_recorder(prometheus_recorder)
        .build();

    metrics::set_global_recorder(fanout).map_err(|e| {
        anyhow::anyhow!(
            "Unable to register a recorder: {}. Did you call this function multiple times?",
            e
        )
    })?;

    Ok(())
}

/// Creates an Actix web scope for metrics endpoints
///
/// This function configures metrics recorders and creates a scope with
/// all necessary routes for the metrics dashboard and Prometheus endpoint.
/// It's the main entry point for integrating metrics into your Actix application.
///
/// The function:
/// 1. Initializes the metrics system (if not already done)
/// 2. Creates an Actix web scope with path "/metrics"
/// 3. Registers all necessary endpoints (/prometheus, /dashboard, etc.)
///
/// # Arguments
///
/// * `input` - Configuration options for the metrics system
///
/// # Returns
///
/// Result containing the configured Actix web Scope that can be integrated
/// into an Actix web application
///
/// # Example
///
/// ```rust,no_run
/// use actix_web::{App, HttpServer};
/// use your_crate::metrics::{create_metrics_actx_scope, DashboardInput};
///
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     HttpServer::new(|| {
///         App::new()
///             .service(create_metrics_actx_scope(&DashboardInput::default()).unwrap())
///             // Your other services...
///     })
///     .bind(("127.0.0.1", 8080))?
///     .run()
///     .await
/// }
/// ```
pub fn create_metrics_actx_scope(input: &DashboardInput) -> Result<Scope> {
    configure_metrics_recorders_once(input)?;
    let scope = web::scope("/metrics")
        .service(get_prometheus_metrics)
        .service(get_dashboard)
        .service(get_dashboard_assets);
    Ok(scope)
}
