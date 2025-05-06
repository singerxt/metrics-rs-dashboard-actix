use std::sync::Arc;

use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use log::info;
use metrics_actix_dashboard::create_metrics_actx_scope;

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello from Actix-Web!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Starting Actix-Web server with metrics at /metrics");
    HttpServer::new(|| {
        let metrics_actix_dashboard = create_metrics_actx_scope().unwrap();
        App::new()
            .route("/", web::get().to(hello))
            .service(metrics_actix_dashboard)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
