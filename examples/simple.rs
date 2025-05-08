use std::thread;

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

    tokio::spawn(async {
        println!("Starting async thread");
        describe_counter!("async_counter", "Incrementing by random number");

        loop {
            let random_number = rand::random_range(1..10);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            counter!("async_counter").increment(random_number);
        }
    });

    thread::spawn(|| {
        describe_counter!("counter", "Incrementing by random number");

        loop {
            let random_number = rand::random_range(1..10);
            thread::sleep(std::time::Duration::from_secs(1));
            counter!("counter").increment(random_number);
        }
    });

    // histogram
    tokio::spawn(async {
        println!("Starting async histogram thread");
        describe_histogram!("async_histogram", "tokio async histogram");

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
