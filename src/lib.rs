//! # Metrics Module
//!
//! This module provides Prometheus metrics integration for Actix web applications.
//! It exposes metrics via HTTP endpoints and includes a dashboard for visualization.

use std::sync::{Mutex, OnceLock};
use actix_web::{HttpResponse, Responder, Scope, web};
use anyhow::{Context, Result};
use log::debug;
use log_once::debug_once;
use metrics_prometheus::failure::strategy::{self, NoOp};
use metrics_util::layers::FanoutBuilder;
use mime_guess::from_path;
use rust_embed::Embed;

/// Global flag to track if metrics recorders have been configured
static IS_CONFIGURED: OnceLock<Mutex<bool>> = OnceLock::new();
/// Global Prometheus recorder instance
static PROMETHEUS_RECORDER: OnceLock<metrics_prometheus::Recorder<NoOp>> = OnceLock::new();

/// Embedded assets for the metrics dashboard
#[derive(Embed)]
#[folder = "public/"]
struct Asset;

/// Serves embedded files from the Asset struct
///
/// # Arguments
///
/// * `path` - Path to the file within the embedded assets
///
/// # Returns
///
/// HttpResponse containing the file content or a 404 if not found
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
/// # Returns
///
/// The main dashboard HTML page
#[actix_web::get("/dashboard")]
async fn get_dashboard() -> impl Responder {
    handle_embedded_file("index.html")
}

/// Handler for serving dashboard assets (JS, CSS, etc.)
///
/// # Arguments
///
/// * `path` - Path to the requested asset
///
/// # Returns
///
/// The requested asset file
#[actix_web::get("/dashboard/{_:.*}")]
async fn get_dashboard_assets(path: web::Path<String>) -> impl Responder {
    handle_embedded_file(path.as_str())
}

/// Endpoint for exposing Prometheus metrics
///
/// # Returns
///
/// Prometheus metrics in text format
#[actix_web::get("/prometheus")]
async fn get_prometheus_metrics() -> impl Responder {
    debug!("Gathering prometheus metrics...");
    let prometheus_recorder = get_prometheus_recorder();
    let metrics = match prometheus::TextEncoder::new()
        .encode_to_string(&prometheus_recorder.registry().gather())
    {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to encode metrics: {}", e));
        }
    };
    HttpResponse::Ok().body(metrics)
}

/// Gets or initializes the Prometheus recorder
///
/// # Returns
///
/// A cloned instance of the global Prometheus recorder
fn get_prometheus_recorder() -> metrics_prometheus::Recorder<NoOp> {
    let prometheus_recorder = PROMETHEUS_RECORDER.get_or_init(|| {
        metrics_prometheus::Recorder::builder()
            .with_failure_strategy(strategy::NoOp)
            .build()
    });
    prometheus_recorder.clone()
}

/// Configures metrics recorders if they haven't been configured yet
///
/// This function is idempotent and safe to call multiple times.
/// Only the first call will actually configure the recorders.
///
/// # Returns
///
/// Result indicating success or failure of configuration
fn configure_metrics_recorders_once() -> Result<()> {
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

    let prometheus_recorder = get_prometheus_recorder();

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
/// # Returns
///
/// Result containing the configured Actix web Scope or an error
pub fn create_metrics_actx_scope() -> Result<Scope> {
    configure_metrics_recorders_once()?;
    let scope = web::scope("/metrics")
        .service(get_prometheus_metrics)
        .service(get_dashboard)
        .service(get_dashboard_assets);
    Ok(scope)
}
