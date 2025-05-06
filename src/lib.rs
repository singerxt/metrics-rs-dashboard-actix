use std::sync::{Mutex, OnceLock};

use actix_web::{web, HttpResponse, Responder, Scope};
use anyhow::{Context, Result};
use log_once::debug_once;
use metrics_prometheus::failure::strategy;
use metrics_util::layers::FanoutBuilder;

static IS_CONFIGURED: OnceLock<Mutex<bool>> = OnceLock::new();

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello from Actix-Web!")
}

fn configure_metrics_recorders_once() -> Result<()> {
    let mutex = IS_CONFIGURED.get_or_init(|| Mutex::new(false));
    let mut is_ok = mutex
        .lock()
        .map_err(|e| anyhow::anyhow!("Mutex poisoned: {}", e))
        .with_context(|| "Unable to lock IS_CONFIGURED")?;

    if *is_ok {
        debug_once!("You have already configured the metrics recorder. This is a no-op. Multiple configuration attempts are safe because only the first one takes effect, preventing duplicate registrations.");
        return Ok(());
    }

    *is_ok = true;

    let prometeus_recorder = metrics_prometheus::Recorder::builder()
        .with_failure_strategy(strategy::NoOp)
        .build();

    let fanout = FanoutBuilder::default()
        .add_recorder(prometeus_recorder)
        .build();

    metrics::set_global_recorder(fanout).expect("Unable to register a recorder.Did you call this function multiple times?");

    Ok(())
}

pub fn create_metrics_actx_scope() -> Result<Scope> {
    configure_metrics_recorders_once()?;
    let scope = web::scope("/metrics")
        .service(web::resource("/").route(web::get().to(hello)));
    Ok(scope)
}
