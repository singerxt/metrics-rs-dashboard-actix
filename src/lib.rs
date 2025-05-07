use std::sync::{Mutex, OnceLock};

use actix_web::{HttpResponse, Responder, Scope, web};
use anyhow::{Context, Result};
use log::debug;
use log_once::debug_once;
use metrics_prometheus::failure::strategy::{self, NoOp};
use metrics_util::layers::FanoutBuilder;
use mime_guess::from_path;
use rust_embed::Embed;

static IS_CONFIGURED: OnceLock<Mutex<bool>> = OnceLock::new();
static PROMETHEUS_RECORDER: OnceLock<metrics_prometheus::Recorder<NoOp>> = OnceLock::new();

#[derive(Embed)]
#[folder = "public/"]
struct Asset;

fn handle_embedded_file(path: &str) -> HttpResponse {
    match Asset::get(path) {
        Some(content) => HttpResponse::Ok()
            .content_type(from_path(path).first_or_octet_stream().as_ref())
            .body(content.data.into_owned()),
        None => HttpResponse::NotFound().body("404 Not Found"),
    }
}

#[actix_web::get("/dashboard")]
async fn get_dashboard() -> impl Responder {
    handle_embedded_file("index.html")
}

#[actix_web::get("/dashboard/{_:.*}")]
async fn get_dashboard_assets(path: web::Path<String>) -> impl Responder {
    handle_embedded_file(path.as_str())
}

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

fn get_prometheus_recorder() -> metrics_prometheus::Recorder<NoOp> {
    let prometheus_recorder = PROMETHEUS_RECORDER.get_or_init(|| {
        metrics_prometheus::Recorder::builder()
            .with_failure_strategy(strategy::NoOp)
            .build()
    });
    prometheus_recorder.clone()
}

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

pub fn create_metrics_actx_scope() -> Result<Scope> {
    configure_metrics_recorders_once()?;
    let scope = web::scope("/metrics")
        .service(get_prometheus_metrics)
        .service(get_dashboard)
        .service(get_dashboard_assets);
    Ok(scope)
}
