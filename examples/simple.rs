use std::{sync::Arc, thread};

use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use log::info;
use metrics::{counter, describe_counter, describe_histogram};
use metrics_actix_dashboard::create_metrics_actx_scope;

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello from Actix-Web!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Starting Actix-Web server with metrics at /metrics");

    thread::spawn(|| {
        describe_counter!("real_time", "Real time counter");

        loop {
            thread::sleep(std::time::Duration::from_secs(1));
            counter!("real_time").increment(1);
        }
    });

    tokio::spawn(async {
        describe_counter!("async_counter", "Async counter");

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            counter!("async_counter").increment(1);
        }
    });

    // histogram
    tokio::spawn(async {
        describe_histogram!("async_histogram", "Async histogram");

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            metrics::histogram!("async_histogram").record(1.0);
        }
    });

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
