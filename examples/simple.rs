use std::thread;

use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use log::info;
use metrics::{Unit, counter, describe_counter, describe_gauge, describe_histogram, gauge};
use metrics_actix_dashboard::{DashboardInput, create_metrics_actx_scope};
use metrics_exporter_prometheus::Matcher;

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
            let random_number = rand::random_range(0..10);
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
        println!("Starting simulated request latency thread");
        describe_histogram!(
            "request_latency",
            Unit::Milliseconds,
            "Simulated latency of HTTP requests in milliseconds"
        );

        loop {
            // Simulate variable latency between 10-500ms
            let latency = rand::random::<f64>() * 490.0 + 10.0;

            // Record the simulated latency
            metrics::histogram!("request_latency").record(latency);

            // Occasionally simulate slower requests (simulate spikes)
            if rand::random::<f64>() < 0.1 {
                // 10% chance of a slow request (500-2000ms)
                let spike_latency = rand::random::<f64>() * 1500.0 + 500.0;
                metrics::histogram!("request_latency").record(spike_latency);
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    });

    tokio::spawn(async {
        println!("Starting simulated request latency thread");
        describe_gauge!(
            "request_latency_gauge",
            "Simulated latency of HTTP requests in milliseconds"
        );

        loop {
            // Simulate variable latency between 10-500ms
            let latency = rand::random::<f64>() * 490.0 + 10.0;

            gauge!("request_latency_gauge").set(latency);
            // Occasionally simulate slower requests (simulate spikes)
            if rand::random::<f64>() < 0.1 {
                // 10% chance of a slow request (500-2000ms)
                let spike_latency = rand::random::<f64>() * 1500.0 + 500.0;
                gauge!("request_latency_gauge").set(spike_latency);
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    });

    HttpServer::new(|| {
        let dashboard_input = DashboardInput {
            buckets_for_metrics: vec![(
                Matcher::Prefix("request_latency".to_string()),
                &[10.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0],
            )],
        };

        let metrics_actix_dashboard = create_metrics_actx_scope(&dashboard_input).unwrap();
        App::new()
            .route("/", web::get().to(hello))
            .service(metrics_actix_dashboard)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
