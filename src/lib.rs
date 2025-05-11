//! # Metrics Module
//!
//! This module provides Prometheus metrics integration for Actix web applications.
//! It exposes metrics via HTTP endpoints and includes a dashboard for visualization.
//!
//! ## Features
//! - Prometheus metrics collection and exposition
//! - Interactive metrics dashboard
//! - Custom histogram bucket configuration
//! - Seamless integration with Actix web applications

/// Re-export of the `metrics` crate for measuring and recording application metrics
pub use metrics;
/// Re-export of the `metrics_exporter_prometheus` crate for exposing metrics in Prometheus format
pub use metrics_exporter_prometheus;
/// Re-export of the `metrics_util` crate for utility functions related to metrics
pub use metrics_util;

use actix_web::{HttpResponse, Responder, Scope, web};
use anyhow::{Context, Result};
use log::debug;
use log_once::debug_once;
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use metrics_util::layers::FanoutBuilder;
use mime_guess::from_path;
use rust_embed::Embed;
use std::sync::{Mutex, OnceLock};

/// Global flag to track if metrics recorders have been configured
static IS_CONFIGURED: OnceLock<Mutex<bool>> = OnceLock::new();
/// Global Prometheus recorder instance
static PROMETHEUS_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

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
    pub buckets_for_metrics: Vec<(Matcher, &'a [f64])>,
}

/// Serves embedded files from the Asset struct
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
///
/// # Returns
///
/// Prometheus metrics in the standard text-based exposition format
#[actix_web::get("/prometheus")]
async fn get_prometheus_metrics() -> impl Responder {
    debug!("Gathering prometheus metrics...");
    let prometheus_handle = PROMETHEUS_HANDLE.get();

    if let Some(handle) = prometheus_handle {
        let metrics = handle.render();
        return HttpResponse::Ok().body(metrics);
    }

    HttpResponse::Ok().body(String::from(""))
}

/// Configures metrics recorders if they haven't been configured yet
///
/// This function is idempotent and safe to call multiple times.
/// Only the first call will actually configure the recorders, subsequent
/// calls will return early with success.
///
/// # Arguments
///
/// * `input` - Configuration options for the metrics system
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
    let mutex = IS_CONFIGURED.get_or_init(|| Mutex::new(false));
    let mut is_ok = mutex
        .lock()
        .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))
        .with_context(|| "Unable to lock IS_CONFIGURED")?;

    if *is_ok {
        debug_once!(
            "You have already configured the metrics recorder. This is a no-op. Multiple configuration attempts are safe because only the first one takes effect, preventing duplicate registrations."
        );
        return Ok(());
    }

    *is_ok = true;

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
