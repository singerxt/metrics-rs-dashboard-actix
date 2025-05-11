use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use log::info;
use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, Unit};
use metrics_rs_dashboard_actix::{DashboardInput, create_metrics_actx_scope};
use metrics_exporter_prometheus::Matcher;

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello from Actix-Web!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Starting Actix-Web server with metrics at /metrics");

    tokio::spawn(async {
        describe_counter!("async_counter", "Incrementing by random number");

        loop {
            let random_number = rand::random_range(0..10);
            let another_random_number = rand::random_range(0..10);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            counter!("async_counter", "type" => "test").increment(random_number);
            counter!("async_counter", "type" => "test_2").increment(another_random_number);
        }
    });

    tokio::spawn(async {
        describe_gauge!("async_gauge", Unit::Milliseconds, "Random number gauge");

        loop {
            let random_number = rand::random_range(0..10);
            let another_random_number = rand::random_range(0..10);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            gauge!("async_gauge", "type" => "gauge_1").set(random_number);
            gauge!("async_gauge", "type" => "gauge_2").set(another_random_number);
        }
    });

    tokio::spawn(async {
        describe_histogram!(
            "request_latency",
            "Simulated latency of HTTP requests in milliseconds"
        );
        describe_gauge!(
            "request_latency_gauge",
            "Simulated latency of HTTP requests in milliseconds"
        );

        loop {
            // Simulate variable latency between 10-500ms
            let latency = rand::random::<f64>() * 490.0 + 10.0;

            gauge!("request_latency_gauge").set(latency);
            metrics::histogram!("request_latency").record(latency);
            // Occasionally simulate slower requests (simulate spikes)
            if rand::random::<f64>() < 0.1 {
                // 10% chance of a slow request (500-2000ms)
                let spike_latency = rand::random::<f64>() * 1500.0 + 500.0;
                gauge!("request_latency_gauge").set(spike_latency);
                metrics::histogram!("request_latency").record(spike_latency);
            }

            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    });

    HttpServer::new(|| {
        let dashboard_input = DashboardInput {
            buckets_for_metrics: vec![(
                Matcher::Prefix("request_latency".to_string()),
                &[50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0],
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
