//! # Metrics Module
//!
//! This module provides comprehensive Prometheus metrics integration for Actix web applications.
//! It enables robust monitoring capabilities with automatic metric collection, exposition,
//! and visualization through an integrated dashboard.
//!
//! ## Features
//! - **Prometheus Integration**: Full support for collecting and exposing metrics in Prometheus format
//! - **Interactive Dashboard**: Built-in web UI for visualizing metrics in real-time
//! - **Rate Metrics**: Automatic calculation and tracking of per-second rates from counter values
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
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

/// Global flag to track if metrics recorders have been configured
static IS_CONFIGURED: AtomicBool = AtomicBool::new(false);

/// Global Prometheus recorder instance
static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

/// Global storage for metric unit information
///
/// Maps metric names to their corresponding units, which is used
/// by the dashboard to correctly display unit information in charts
static UNITS_FOR_METRICS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

/// Global storage for rate trackers
///
/// Maps counter names to their rate tracking instances
static RATE_TRACKERS: OnceLock<Mutex<HashMap<String, RateTracker>>> = OnceLock::new();

/// Embedded assets for the metrics dashboard
#[derive(Embed)]
#[folder = "public/"]
struct Asset;

/// Rate tracking utility for calculating per-second rates from counter values
///
/// This struct tracks the last value and timestamp of a counter to calculate
/// the rate of change over time. It's used internally by the rate metric
/// functionality to provide per-second rate calculations.
#[derive(Debug, Clone)]
pub struct RateTracker {
    samples: Vec<(f64, Instant)>,
    window_duration: Duration,
    max_samples: usize,
}

impl Default for RateTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl RateTracker {
    /// Creates a new RateTracker with sliding window for high-frequency updates
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            window_duration: Duration::from_secs(2), // 2-second sliding window
            max_samples: 200,                        // Limit memory usage
        }
    }

    /// Updates the tracker with a new value and calculates the rate
    ///
    /// # Arguments
    /// * `new_value` - The new counter value
    ///
    /// # Returns
    /// The calculated rate per second based on sliding window analysis
    pub fn update(&mut self, new_value: f64) -> f64 {
        let now = Instant::now();

        // Add new sample
        self.samples.push((new_value, now));

        // Remove samples outside the window
        let cutoff = now - self.window_duration;
        self.samples.retain(|(_, timestamp)| *timestamp > cutoff);

        // Limit samples to prevent unbounded growth
        if self.samples.len() > self.max_samples {
            let excess = self.samples.len() - self.max_samples;
            self.samples.drain(0..excess);
        }

        // Need at least 2 samples to calculate rate
        if self.samples.len() < 2 {
            return 0.0;
        }

        // Calculate rate using oldest and newest samples in window
        let (first_value, first_time) = self.samples[0];
        let (last_value, last_time) = self.samples[self.samples.len() - 1];

        let time_diff = last_time.duration_since(first_time).as_secs_f64();

        if time_diff <= 0.0 {
            return 0.0;
        }

        let value_diff = last_value - first_value;

        // Ensure we don't return negative rates for counters
        (value_diff / time_diff).max(0.0)
    }
}

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
    fn describe_counter(
        &self,
        key: metrics::KeyName,
        unit: Option<metrics::Unit>,
        _description: metrics::SharedString,
    ) {
        self.register_unit(key, unit);
    }

    fn describe_gauge(
        &self,
        key: metrics::KeyName,
        unit: Option<metrics::Unit>,
        _description: metrics::SharedString,
    ) {
        self.register_unit(key, unit);
    }

    fn describe_histogram(
        &self,
        key: metrics::KeyName,
        unit: Option<metrics::Unit>,
        _description: metrics::SharedString,
    ) {
        self.register_unit(key, unit);
    }

    fn register_counter(
        &self,
        key: &metrics::Key,
        _metadata: &metrics::Metadata<'_>,
    ) -> metrics::Counter {
        Counter::from_arc(Arc::new(UnitRecorderHandle(key.clone())))
    }

    fn register_gauge(
        &self,
        key: &metrics::Key,
        _metadata: &metrics::Metadata<'_>,
    ) -> metrics::Gauge {
        Gauge::from_arc(Arc::new(UnitRecorderHandle(key.clone())))
    }

    fn register_histogram(
        &self,
        key: &metrics::Key,
        _metadata: &metrics::Metadata<'_>,
    ) -> metrics::Histogram {
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
    if IS_CONFIGURED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
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

    tokio::spawn(async move {
        let handle = PROMETHEUS_HANDLE.get();

        if let Some(handle) = handle {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                handle.run_upkeep();
            }
        } else {
            debug!("Prometheus handle not set. Skipping recorder cleanup.");
        }
    });

    metrics::set_global_recorder(fanout).map_err(|e| {
        anyhow::anyhow!(
            "Unable to register a recorder: {}. Did you call this function multiple times?",
            e
        )
    })?;

    Ok(())
}

/// Updates a rate tracker and returns the calculated rate
///
/// This function is used internally by the rate macros to calculate
/// and track per-second rates from counter values.
pub fn update_rate_tracker(_counter_name: &str, value: f64, tracker_key: String) -> f64 {
    let rate_trackers = RATE_TRACKERS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut trackers) = rate_trackers.lock() {
        let tracker = trackers.entry(tracker_key).or_insert_with(RateTracker::new);
        tracker.update(value)
    } else {
        0.0
    }
}

/// Macro for recording a counter with automatic rate tracking
///
/// This macro records both a counter value and its per-second rate.
///
/// # Example
///
/// ```rust
/// use metrics_rs_dashboard_actix::counter_with_rate;
///
/// // Simple counter with rate
/// counter_with_rate!("requests_total", 1.0);
///
/// // Counter with labels and rate
/// counter_with_rate!("requests_total", 1.0, "endpoint", "/api/users");
/// ```
#[macro_export]
macro_rules! counter_with_rate {
    ($name:expr, $value:expr) => {{
        use $crate::update_rate_tracker;

        // Record the counter
        metrics::counter!($name).increment($value as u64);

        // Calculate and record the rate
        let rate_name = format!("{}_rate_per_sec", $name);
        let tracker_key = format!("{}_default", $name);
        let rate = update_rate_tracker($name, $value, tracker_key);
        metrics::gauge!(rate_name).set(rate);
    }};
    ($name:expr, $value:expr, $label_key:expr, $label_value:expr) => {{
        use $crate::update_rate_tracker;

        // Record the counter with labels
        metrics::counter!($name, $label_key => $label_value).increment($value as u64);

        // Calculate and record the rate with labels
        let rate_name = format!("{}_rate_per_sec", $name);
        let tracker_key = format!("{}_{}_{}", $name, $label_key, $label_value);
        let rate = update_rate_tracker($name, $value, tracker_key);
        metrics::gauge!(rate_name, $label_key => $label_value).set(rate);
    }};
}

/// Macro for recording an absolute counter value with automatic rate tracking
///
/// This macro is similar to `counter_with_rate!` but sets the counter to an absolute value.
///
/// # Example
///
/// ```rust
/// use metrics_rs_dashboard_actix::absolute_counter_with_rate;
///
/// // Simple absolute counter with rate
/// absolute_counter_with_rate!("bytes_processed_total", 1024.0);
///
/// // Absolute counter with labels and rate
/// absolute_counter_with_rate!("db_queries_total", 42.0, "type", "SELECT");
/// ```
#[macro_export]
macro_rules! absolute_counter_with_rate {
    ($name:expr, $value:expr) => {{
        use $crate::update_rate_tracker;

        // Record the absolute counter
        metrics::counter!($name).absolute($value as u64);

        // Calculate and record the rate
        let rate_name = format!("{}_rate_per_sec", $name);
        let tracker_key = format!("{}_default", $name);
        let rate = update_rate_tracker($name, $value, tracker_key);
        metrics::gauge!(rate_name).set(rate);
    }};
    ($name:expr, $value:expr, $label_key:expr, $label_value:expr) => {{
        use $crate::update_rate_tracker;

        // Record the absolute counter with labels
        metrics::counter!($name, $label_key => $label_value).absolute($value as u64);

        // Calculate and record the rate with labels
        let rate_name = format!("{}_rate_per_sec", $name);
        let tracker_key = format!("{}_{}_{}", $name, $label_key, $label_value);
        let rate = update_rate_tracker($name, $value, tracker_key);
        metrics::gauge!(rate_name, $label_key => $label_value).set(rate);
    }};
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
/// use metrics_rs_dashboard_actix::{create_metrics_actx_scope, DashboardInput};
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_rate_tracker_new() {
        let tracker = RateTracker::new();
        assert!(tracker.samples.is_empty());
        assert_eq!(tracker.window_duration, Duration::from_secs(2));
        assert_eq!(tracker.max_samples, 200);
    }

    #[test]
    fn test_rate_tracker_default() {
        let tracker = RateTracker::default();
        assert!(tracker.samples.is_empty());
        assert_eq!(tracker.window_duration, Duration::from_secs(2));
        assert_eq!(tracker.max_samples, 200);
    }

    #[test]
    fn test_rate_tracker_first_update() {
        let mut tracker = RateTracker::new();

        let rate = tracker.update(10.0);

        // First update should return 0.0 (no previous sample)
        assert_eq!(rate, 0.0);
        assert_eq!(tracker.samples.len(), 1);
        assert_eq!(tracker.samples[0].0, 10.0);
    }

    #[test]
    fn test_rate_tracker_subsequent_updates() {
        let mut tracker = RateTracker::new();

        // First update
        tracker.update(10.0);

        // Wait a bit to ensure time difference
        thread::sleep(Duration::from_millis(20));

        // Second update
        let rate = tracker.update(20.0);

        // Rate should be positive (10 units over ~0.02 seconds = ~500 units/sec)
        assert!(rate > 0.0);
        assert!(rate > 100.0); // Should be high due to short time interval
        assert_eq!(tracker.samples.len(), 2);
    }

    #[test]
    fn test_rate_tracker_negative_rate_clamping() {
        let mut tracker = RateTracker::new();

        // First update with higher value
        tracker.update(20.0);

        thread::sleep(Duration::from_millis(20));

        // Second update with lower value (would normally give negative rate)
        let rate = tracker.update(10.0);

        // Rate should be clamped to 0.0 for counters (negative rates become 0.0)
        assert_eq!(rate, 0.0);
        assert_eq!(tracker.samples.len(), 2);
        assert_eq!(tracker.samples[1].0, 10.0);
    }

    #[test]
    fn test_rate_tracker_high_frequency_updates() {
        let mut tracker = RateTracker::new();

        // First update
        tracker.update(10.0);

        // Immediate second update (now handles high frequency)
        let rate = tracker.update(20.0);

        // Should calculate rate even for very fast updates
        assert!(rate >= 0.0);
        assert_eq!(tracker.samples.len(), 2);
        assert_eq!(tracker.samples[1].0, 20.0);
    }

    #[test]
    fn test_update_rate_tracker_function() {
        let tracker_key = "test_metric_default".to_string();

        // First call
        let rate1 = update_rate_tracker("test_metric", 10.0, tracker_key.clone());
        assert_eq!(rate1, 0.0); // First call should return 0

        thread::sleep(Duration::from_millis(200));

        // Second call
        let rate2 = update_rate_tracker("test_metric", 20.0, tracker_key);
        assert!(rate2 >= 0.0); // Should return a valid rate
    }

    #[test]
    fn test_counter_with_rate_macro_simple() {
        // This test verifies the macro compiles and doesn't panic
        // We can't easily test the actual metric recording without setting up the full recorder
        let result = std::panic::catch_unwind(|| {
            counter_with_rate!("test_counter", 1.0);
        });

        // The macro should complete without panicking
        // Note: In a real test environment, you'd verify the metrics were actually recorded
        assert!(result.is_ok());
    }

    #[test]
    fn test_counter_with_rate_macro_with_labels() {
        // This test verifies the macro with labels compiles and doesn't panic
        let result = std::panic::catch_unwind(|| {
            counter_with_rate!("test_counter_labeled", 2.0, "service", "api");
        });

        assert!(result.is_ok());
    }

    #[test]
    fn test_absolute_counter_with_rate_macro_simple() {
        // This test verifies the macro compiles and doesn't panic
        let result = std::panic::catch_unwind(|| {
            absolute_counter_with_rate!("test_absolute_counter", 42.0);
        });

        assert!(result.is_ok());
    }

    #[test]
    fn test_absolute_counter_with_rate_macro_with_labels() {
        // This test verifies the macro with labels compiles and doesn't panic
        let result = std::panic::catch_unwind(|| {
            absolute_counter_with_rate!("test_absolute_counter_labeled", 100.0, "type", "batch");
        });

        assert!(result.is_ok());
    }

    #[test]
    fn test_rate_calculation_accuracy() {
        let mut tracker = RateTracker::new();

        // Set initial value
        tracker.update(0.0);

        // Wait exactly 1 second
        thread::sleep(Duration::from_secs(1));

        // Add 10 units after 1 second
        let rate = tracker.update(10.0);

        // Rate should be approximately 10 units/second
        assert!(
            (rate - 10.0).abs() < 1.0,
            "Rate {} should be close to 10.0",
            rate
        );
    }

    #[test]
    fn test_multiple_rate_tracker_instances() {
        let key1 = "metric1_default".to_string();
        let key2 = "metric2_default".to_string();

        // Test that different tracker keys maintain separate state
        update_rate_tracker("metric1", 10.0, key1.clone());
        update_rate_tracker("metric2", 20.0, key2.clone());

        thread::sleep(Duration::from_millis(200));

        let rate1 = update_rate_tracker("metric1", 15.0, key1);
        let rate2 = update_rate_tracker("metric2", 30.0, key2);

        // Both should return valid rates
        assert!(rate1 >= 0.0);
        assert!(rate2 >= 0.0);

        // Rates should be different since the value changes are different
        // (5 units vs 10 units over the same time period)
        if rate1 > 0.0 && rate2 > 0.0 {
            assert!(
                (rate2 / rate1 - 2.0).abs() < 0.5,
                "Rate2 ({}) should be approximately twice rate1 ({})",
                rate2,
                rate1
            );
        }
    }

    #[test]
    fn test_dashboard_input_default() {
        let input = DashboardInput::default();
        assert!(input.buckets_for_metrics.is_empty());
    }

    #[test]
    fn test_dashboard_input_with_buckets() {
        let buckets = &[1.0, 5.0, 10.0];
        let input = DashboardInput {
            buckets_for_metrics: vec![(
                metrics_exporter_prometheus::Matcher::Full("test_metric".to_string()),
                buckets,
            )],
        };

        assert_eq!(input.buckets_for_metrics.len(), 1);
        assert_eq!(input.buckets_for_metrics[0].1, buckets);
    }

    #[test]
    fn test_rate_tracker_zero_value_update() {
        let mut tracker = RateTracker::new();

        thread::sleep(Duration::from_millis(150));

        // Update with 0.0 value
        let rate = tracker.update(0.0);

        // Should return 0.0 rate (first update)
        assert_eq!(rate, 0.0);
        assert_eq!(tracker.samples.len(), 1);
        assert_eq!(tracker.samples[0].0, 0.0);
    }

    #[test]
    fn test_rate_tracker_large_values() {
        let mut tracker = RateTracker::new();

        // First update
        tracker.update(500_000.0);

        thread::sleep(Duration::from_millis(20));

        // Test with large values
        let large_value = 1_000_000.0;
        let rate = tracker.update(large_value);

        assert!(rate > 0.0);
        assert_eq!(tracker.samples.len(), 2);
        assert_eq!(tracker.samples[1].0, large_value);
    }

    #[test]
    fn test_rate_tracker_fractional_values() {
        let mut tracker = RateTracker::new();

        // First update with fractional value
        tracker.update(1.5);

        thread::sleep(Duration::from_millis(20));

        // Second update with another fractional value
        let rate = tracker.update(3.7);

        // Should handle fractional values correctly
        assert!(rate > 0.0);
        assert_eq!(tracker.samples.len(), 2);
        assert_eq!(tracker.samples[1].0, 3.7);
    }

    #[test]
    fn test_update_rate_tracker_concurrent_access() {
        use std::thread;

        let handles: Vec<_> = (0..5)
            .map(|i| {
                thread::spawn(move || {
                    let tracker_key = format!("concurrent_test_{}", i);

                    // Each thread updates its own tracker
                    update_rate_tracker("concurrent_metric", 10.0, tracker_key.clone());

                    thread::sleep(Duration::from_millis(200));

                    update_rate_tracker("concurrent_metric", 20.0, tracker_key)
                })
            })
            .collect();

        // Wait for all threads to complete
        for handle in handles {
            let rate = handle.join().expect("Thread should complete successfully");
            assert!(rate >= 0.0);
        }
    }

    #[test]
    fn test_rate_tracker_consistent_timestamps() {
        let mut tracker = RateTracker::new();

        let start_time = std::time::Instant::now();

        thread::sleep(Duration::from_millis(20));

        tracker.update(5.0);

        // Check that the sample was recorded with a reasonable timestamp
        assert_eq!(tracker.samples.len(), 1);
        assert!(tracker.samples[0].1 > start_time);
    }
}
