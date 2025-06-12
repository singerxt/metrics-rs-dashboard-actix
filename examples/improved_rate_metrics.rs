use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use log::info;
use metrics::{Unit, describe_counter, describe_gauge};
use metrics_exporter_prometheus::Matcher;
use metrics_rs_dashboard_actix::{
    DashboardInput, absolute_counter_with_rate, counter_with_rate, create_metrics_actx_scope,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::time::{Instant, interval};

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello from Improved Rate Metrics Example!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Starting Actix-Web server with improved rate metrics at /metrics");

    // Shared counters for simulating accumulated values
    let total_requests = Arc::new(AtomicU64::new(0));
    let total_bytes = Arc::new(AtomicU64::new(0));
    let total_errors = Arc::new(AtomicU64::new(0));

    // Example 1: High-frequency counter with batched rate updates
    // This simulates a scenario where events happen very frequently but we want
    // to track rates without overwhelming the rate tracker
    {
        let requests_counter = total_requests.clone();
        tokio::spawn(async move {
            describe_counter!(
                "http_requests_total",
                Unit::Count,
                "Total number of HTTP requests processed"
            );
            describe_gauge!(
                "http_requests_total_rate_per_sec",
                Unit::CountPerSecond,
                "Rate of HTTP requests per second"
            );

            // Create a ticker that updates every 200ms (well above the 100ms threshold)
            let mut ticker = interval(Duration::from_millis(200));

            loop {
                ticker.tick().await;

                // Simulate processing many requests between rate updates
                let burst_size = rand::random::<u32>() % 50 + 1; // 1-50 requests
                for _ in 0..burst_size {
                    requests_counter.fetch_add(1, Ordering::Relaxed);

                    // Simulate some processing time
                    tokio::time::sleep(Duration::from_micros(100)).await;
                }

                // Now update the rate metric with the current total
                let current_total = requests_counter.load(Ordering::Relaxed);
                absolute_counter_with_rate!(
                    "http_requests_total",
                    current_total as f64,
                    "endpoint",
                    "/api/users"
                );
            }
        });
    }

    // Example 2: Separate thread for rate calculation to avoid contention
    // This shows how to decouple the business logic from rate tracking
    {
        let bytes_counter_processing = total_bytes.clone();
        let bytes_counter_tracking = total_bytes.clone();

        // High-frequency data processing thread
        tokio::spawn(async move {
            loop {
                // Simulate processing data chunks at high frequency
                let chunk_size = rand::random::<u64>() % 8192 + 1024; // 1KB-8KB
                bytes_counter_processing.fetch_add(chunk_size, Ordering::Relaxed);

                // Process at high frequency
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        });

        // Separate thread for rate tracking
        tokio::spawn(async move {
            describe_counter!(
                "bytes_processed_total",
                Unit::Bytes,
                "Total bytes processed by the application"
            );
            describe_gauge!(
                "bytes_processed_total_rate_per_sec",
                Unit::Count,
                "Rate of bytes processed per second"
            );

            let mut ticker = interval(Duration::from_millis(250));

            loop {
                ticker.tick().await;

                let current_total = bytes_counter_tracking.load(Ordering::Relaxed);
                absolute_counter_with_rate!(
                    "bytes_processed_total",
                    current_total as f64,
                    "processor",
                    "data_pipeline"
                );
            }
        });
    }

    // Example 3: Error tracking with conditional rate updates
    // This shows how to handle infrequent events that might not meet the timing threshold
    {
        let errors_counter = total_errors.clone();
        tokio::spawn(async move {
            describe_counter!(
                "errors_total",
                Unit::Count,
                "Total number of errors encountered"
            );
            describe_gauge!(
                "errors_total_rate_per_sec",
                Unit::CountPerSecond,
                "Rate of errors per second"
            );

            let mut last_update = Instant::now();
            let min_update_interval = Duration::from_millis(150);

            loop {
                // Errors occur less frequently
                if rand::random::<f64>() < 0.05 {
                    // 5% chance per iteration
                    errors_counter.fetch_add(1, Ordering::Relaxed);

                    // Only update rate if enough time has passed
                    let now = Instant::now();
                    if now.duration_since(last_update) >= min_update_interval {
                        let current_total = errors_counter.load(Ordering::Relaxed);

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

                        last_update = now;
                    }
                }

                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });
    }

    // Example 4: Using incremental counter_with_rate! properly
    // This shows the correct usage for incremental metrics
    tokio::spawn(async move {
        describe_counter!("network_packets_sent", Unit::Count, "Network packets sent");
        describe_gauge!(
            "network_packets_sent_rate_per_sec",
            Unit::CountPerSecond,
            "Rate of network packets sent per second"
        );

        let mut ticker = interval(Duration::from_millis(300));

        loop {
            ticker.tick().await;

            // Simulate sending a batch of packets
            let packets_in_batch = rand::random::<u32>() % 10 + 1; // 1-10 packets

            // Use counter_with_rate! for the batch size (incremental value)
            counter_with_rate!(
                "network_packets_sent",
                packets_in_batch as f64,
                "interface",
                "eth0"
            );
        }
    });

    // Example 5: Multiple metrics with different labels to show isolation
    // This demonstrates that different label combinations get separate rate trackers
    tokio::spawn(async move {
        describe_counter!(
            "api_requests_total",
            Unit::Count,
            "Total API requests by endpoint"
        );
        describe_gauge!(
            "api_requests_total_rate_per_sec",
            Unit::CountPerSecond,
            "Rate of API requests per second by endpoint"
        );

        let endpoints = vec!["/api/users", "/api/orders", "/api/products"];
        let mut counters = vec![0u64; endpoints.len()];
        let mut ticker = interval(Duration::from_millis(180));

        loop {
            ticker.tick().await;

            // Simulate requests to different endpoints
            for (i, endpoint) in endpoints.iter().enumerate() {
                if rand::random::<f64>() < 0.6 {
                    // 60% chance of activity
                    let requests = rand::random::<u32>() % 5 + 1; // 1-5 requests
                    counters[i] += requests as u64;

                    // Each endpoint gets its own rate tracker due to different labels
                    absolute_counter_with_rate!(
                        "api_requests_total",
                        counters[i] as f64,
                        "endpoint",
                        *endpoint
                    );
                }
            }
        }
    });

    // Example 6: Manual rate calculation for comparison
    // This shows how you might implement your own rate calculation for very specific needs
    tokio::spawn(async move {
        describe_counter!(
            "custom_metric_total",
            Unit::Count,
            "Custom metric with manual rate calculation"
        );
        describe_gauge!(
            "custom_metric_manual_rate_per_sec",
            Unit::CountPerSecond,
            "Manually calculated rate for custom metric"
        );

        let mut last_value = 0u64;
        let mut last_time = Instant::now();
        let mut current_value = 0u64;
        let mut ticker = interval(Duration::from_millis(500));

        loop {
            ticker.tick().await;

            // Simulate metric updates
            current_value += rand::random::<u64>() % 20 + 1;

            // Update the counter
            metrics::counter!("custom_metric_total").absolute(current_value);

            // Manual rate calculation
            let now = Instant::now();
            let time_diff = now.duration_since(last_time).as_secs_f64();

            if time_diff > 0.0 {
                let value_diff = current_value - last_value;
                let rate = value_diff as f64 / time_diff;

                metrics::gauge!("custom_metric_manual_rate_per_sec").set(rate);

                last_value = current_value;
                last_time = now;
            }
        }
    });

    HttpServer::new(|| {
        let dashboard_input = DashboardInput {
            buckets_for_metrics: vec![
                (
                    Matcher::Prefix("http_requests".to_string()),
                    &[1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 200.0, 500.0],
                ),
                (
                    Matcher::Prefix("bytes_processed".to_string()),
                    &[1024.0, 4096.0, 16384.0, 65536.0, 262144.0, 1048576.0],
                ),
                (
                    Matcher::Prefix("api_requests".to_string()),
                    &[0.1, 0.5, 1.0, 2.5, 5.0, 10.0, 25.0, 50.0],
                ),
            ],
        };

        let metrics_actix_dashboard = create_metrics_actx_scope(&dashboard_input).unwrap();
        App::new()
            .route("/", web::get().to(hello))
            .service(metrics_actix_dashboard)
    })
    .bind(("127.0.0.1", 8082))?
    .run()
    .await
}
