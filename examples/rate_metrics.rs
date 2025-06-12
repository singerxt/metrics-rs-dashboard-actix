use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use log::info;
use metrics::{Unit, describe_counter};
use metrics_exporter_prometheus::Matcher;
use metrics_rs_dashboard_actix::{
    DashboardInput, absolute_counter_with_rate, counter_with_rate, create_metrics_actx_scope,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello from Actix-Web with Rate Metrics!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Starting Actix-Web server with rate metrics at /metrics");

    // Shared counter for simulating accumulated values
    let total_requests = Arc::new(AtomicU64::new(0));
    let total_bytes = Arc::new(AtomicU64::new(0));
    let total_errors = Arc::new(AtomicU64::new(0));

    // Simulate HTTP request processing with rate tracking
    {
        let requests_counter = total_requests.clone();
        tokio::spawn(async move {
            describe_counter!(
                "http_requests_total",
                Unit::Count,
                "Total number of HTTP requests processed"
            );

            loop {
                // Simulate varying request rates
                let requests_this_interval = if rand::random::<f64>() < 0.3 {
                    // 30% chance of burst (5-20 requests)
                    rand::random_range(5..20)
                } else {
                    // Normal load (1-5 requests)
                    rand::random_range(1..5)
                };

                for _ in 0..requests_this_interval {
                    let current_total = requests_counter.fetch_add(1, Ordering::Relaxed) + 1;

                    // Record both the counter and its rate
                    absolute_counter_with_rate!(
                        "http_requests_total",
                        current_total as f64,
                        "endpoint",
                        "/api/users"
                    );
                }

                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        });
    }

    // Simulate data processing with byte rate tracking
    {
        let bytes_counter = total_bytes.clone();
        tokio::spawn(async move {
            describe_counter!(
                "bytes_processed_total",
                Unit::Bytes,
                "Total bytes processed by the application"
            );

            loop {
                // Simulate varying data processing rates
                let bytes_this_interval = rand::random_range(1024..8192); // 1KB to 8KB
                let current_total = bytes_counter.fetch_add(bytes_this_interval, Ordering::Relaxed)
                    + bytes_this_interval;

                absolute_counter_with_rate!(
                    "bytes_processed_total",
                    current_total as f64,
                    "processor",
                    "data_pipeline"
                );

                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        });
    }

    // Simulate error occurrence with rate tracking
    {
        let errors_counter = total_errors.clone();
        tokio::spawn(async move {
            describe_counter!(
                "errors_total",
                Unit::Count,
                "Total number of errors encountered"
            );

            loop {
                // Errors occur less frequently
                if rand::random::<f64>() < 0.1 {
                    // 10% chance per interval
                    let current_total = errors_counter.fetch_add(1, Ordering::Relaxed) + 1;

                    let error_type = if rand::random::<f64>() < 0.7 {
                        "timeout"
                    } else {
                        "validation"
                    };

                    absolute_counter_with_rate!(
                        "errors_total",
                        current_total as f64,
                        "type",
                        error_type
                    );
                }

                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            }
        });
    }

    // Simulate network throughput with incremental rate tracking
    tokio::spawn(async move {
        describe_counter!("network_bytes_sent", Unit::Bytes, "Network bytes sent");

        loop {
            // Simulate sending data in chunks
            let chunk_size = rand::random_range(512..2048); // 512B to 2KB chunks

            counter_with_rate!("network_bytes_sent", chunk_size as f64, "interface", "eth0");

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    });

    // Simulate database operations with rate tracking
    tokio::spawn(async move {
        describe_counter!(
            "db_queries_total",
            Unit::Count,
            "Total database queries executed"
        );

        let mut query_count = 0u64;

        loop {
            // Simulate bursts of database activity
            let queries_this_batch = if rand::random::<f64>() < 0.2 {
                // 20% chance of heavy batch (10-50 queries)
                rand::random_range(10..50)
            } else {
                // Normal activity (1-3 queries)
                rand::random_range(1..3)
            };

            for _ in 0..queries_this_batch {
                query_count += 1;

                let query_type = match rand::random_range(0..3) {
                    0 => "SELECT",
                    1 => "INSERT",
                    _ => "UPDATE",
                };

                absolute_counter_with_rate!(
                    "db_queries_total",
                    query_count as f64,
                    "type",
                    query_type
                );
            }

            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
    });

    // Simulate message queue processing
    tokio::spawn(async move {
        describe_counter!(
            "messages_processed",
            Unit::Count,
            "Messages processed from queue"
        );

        let mut message_count = 0u64;

        loop {
            // Process messages at varying rates
            let messages_this_round = rand::random_range(1..8);

            for _ in 0..messages_this_round {
                message_count += 1;

                let queue_name = if rand::random::<bool>() {
                    "high_priority"
                } else {
                    "normal"
                };

                absolute_counter_with_rate!(
                    "messages_processed",
                    message_count as f64,
                    "queue",
                    queue_name
                );
            }

            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        }
    });

    HttpServer::new(|| {
        let dashboard_input = DashboardInput {
            buckets_for_metrics: vec![
                (
                    Matcher::Prefix("http_requests".to_string()),
                    &[1.0, 5.0, 10.0, 25.0, 50.0, 100.0],
                ),
                (
                    Matcher::Prefix("bytes_processed".to_string()),
                    &[1024.0, 4096.0, 16384.0, 65536.0, 262144.0, 1048576.0],
                ),
            ],
        };

        let metrics_actix_dashboard = create_metrics_actx_scope(&dashboard_input).unwrap();
        App::new()
            .route("/", web::get().to(hello))
            .service(metrics_actix_dashboard)
    })
    .bind(("127.0.0.1", 8081))?
    .run()
    .await
}
