use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use log::info;
use metrics::{Unit, describe_counter, describe_gauge};
use metrics_exporter_prometheus::Matcher;
use metrics_rs_dashboard_actix::{
    DashboardInput, absolute_counter_with_rate, counter_with_rate, create_metrics_actx_scope,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::time::interval;

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("High-Frequency Rate Tracking Demo - 100+ calls/sec")
}

async fn status() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "running",
        "frequency": "100+ calls per second",
        "endpoints": {
            "metrics": "/metrics",
            "dashboard": "/dashboard",
            "status": "/status"
        }
    }))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Starting High-Frequency Rate Tracking Demo");
    info!("Server running at: http://127.0.0.1:8084");
    info!("Metrics endpoint: http://127.0.0.1:8084/metrics");
    info!("Dashboard: http://127.0.0.1:8084/dashboard");
    info!("Status: http://127.0.0.1:8084/status");

    // Shared counters for demonstration
    let ultra_high_freq_counter = Arc::new(AtomicU64::new(0));
    let burst_counter = Arc::new(AtomicU64::new(0));
    let steady_counter = Arc::new(AtomicU64::new(0));
    let variable_counter = Arc::new(AtomicU64::new(0));

    // Example 1: Ultra high-frequency updates (200+ calls per second)
    // This demonstrates the library can handle very rapid updates
    {
        let counter = ultra_high_freq_counter.clone();
        tokio::spawn(async move {
            describe_counter!(
                "ultra_high_frequency_events",
                Unit::Count,
                "Events processed at 200+ calls per second"
            );
            describe_gauge!(
                "ultra_high_frequency_events_rate_per_sec",
                Unit::CountPerSecond,
                "Rate of ultra high frequency events"
            );

            let mut interval_timer = interval(Duration::from_millis(5)); // 200 calls/sec

            loop {
                interval_timer.tick().await;

                let current_total = counter.fetch_add(1, Ordering::Relaxed) + 1;

                // This call happens every 5ms (200 times per second)
                absolute_counter_with_rate!(
                    "ultra_high_frequency_events",
                    current_total as f64,
                    "processor",
                    "ultra_fast"
                );
            }
        });
    }

    // Example 2: Exactly 100 calls per second with burst patterns
    {
        let counter = burst_counter.clone();
        tokio::spawn(async move {
            describe_counter!(
                "burst_pattern_events",
                Unit::Count,
                "Events with burst patterns at 100 calls/sec"
            );
            describe_gauge!(
                "burst_pattern_events_rate_per_sec",
                Unit::CountPerSecond,
                "Rate of burst pattern events"
            );

            let mut interval_timer = interval(Duration::from_millis(10)); // 100 calls/sec
            let mut burst_mode = false;
            let mut burst_count = 0;

            loop {
                interval_timer.tick().await;

                // Alternate between burst and normal modes
                if burst_count % 500 == 0 {
                    burst_mode = !burst_mode;
                    info!(
                        "Switching to {} mode",
                        if burst_mode { "BURST" } else { "NORMAL" }
                    );
                }

                let increment = if burst_mode {
                    rand::random::<u32>() % 10 + 5 // 5-14 events per call
                } else {
                    1 // 1 event per call
                };

                let current_total =
                    counter.fetch_add(increment as u64, Ordering::Relaxed) + increment as u64;

                absolute_counter_with_rate!(
                    "burst_pattern_events",
                    current_total as f64,
                    "mode",
                    if burst_mode { "burst" } else { "normal" }
                );

                burst_count += 1;
            }
        });
    }

    // Example 3: Steady 100 calls per second with different endpoints
    {
        let counter = steady_counter.clone();
        tokio::spawn(async move {
            describe_counter!(
                "api_requests_steady",
                Unit::Count,
                "Steady API requests at 100 calls/sec"
            );
            describe_gauge!(
                "api_requests_steady_rate_per_sec",
                Unit::CountPerSecond,
                "Rate of steady API requests"
            );

            let endpoints = [
                "/api/users",
                "/api/orders",
                "/api/products",
                "/api/analytics",
            ];
            let mut counters = [0u64; 4];
            let mut interval_timer = interval(Duration::from_millis(10)); // 100 calls/sec

            loop {
                interval_timer.tick().await;

                // Randomly select an endpoint (simulating real traffic distribution)
                let endpoint_idx = rand::random::<u32>() as usize % endpoints.len();
                counters[endpoint_idx] += 1;

                let total = counters.iter().sum::<u64>();
                counter.store(total, Ordering::Relaxed);

                absolute_counter_with_rate!(
                    "api_requests_steady",
                    total as f64,
                    "endpoint",
                    endpoints[endpoint_idx]
                );
            }
        });
    }

    // Example 4: Variable frequency (50-150 calls per second)
    {
        let counter = variable_counter.clone();
        tokio::spawn(async move {
            describe_counter!(
                "variable_frequency_events",
                Unit::Count,
                "Events with variable frequency"
            );
            describe_gauge!(
                "variable_frequency_events_rate_per_sec",
                Unit::CountPerSecond,
                "Rate of variable frequency events"
            );

            loop {
                // Variable timing: 6.67ms to 20ms (50-150 calls per second)
                let delay_ms = rand::random::<u64>() % 14 + 7; // 7-20ms
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;

                let current_total = counter.fetch_add(1, Ordering::Relaxed) + 1;

                let load_level = match delay_ms {
                    7..=10 => "high",    // ~100-142 calls/sec
                    11..=15 => "medium", // ~66-90 calls/sec
                    _ => "low",          // ~50-62 calls/sec
                };

                absolute_counter_with_rate!(
                    "variable_frequency_events",
                    current_total as f64,
                    "load",
                    load_level
                );
            }
        });
    }

    // Example 5: Incremental counter with high frequency
    // This shows using counter_with_rate! for actual increments
    tokio::spawn(async move {
        describe_counter!(
            "incremental_high_freq",
            Unit::Count,
            "Incremental counter at high frequency"
        );
        describe_gauge!(
            "incremental_high_freq_rate_per_sec",
            Unit::CountPerSecond,
            "Rate of incremental high frequency counter"
        );

        let mut interval_timer = interval(Duration::from_millis(10)); // 100 calls/sec

        loop {
            interval_timer.tick().await;

            // Use counter_with_rate! for incremental values
            let increment = rand::random::<u32>() % 5 + 1; // 1-5 increment
            counter_with_rate!(
                "incremental_high_freq",
                increment as f64,
                "type",
                "incremental"
            );
        }
    });

    // Example 6: Network throughput simulation at high frequency
    tokio::spawn(async move {
        describe_counter!(
            "network_bytes_high_freq",
            Unit::Bytes,
            "Network bytes at high frequency"
        );
        describe_gauge!(
            "network_bytes_high_freq_rate_per_sec",
            Unit::Count,
            "Network byte rate at high frequency"
        );

        let mut total_bytes = 0u64;
        let mut interval_timer = interval(Duration::from_millis(8)); // ~125 calls/sec

        loop {
            interval_timer.tick().await;

            // Simulate network packet sizes
            let packet_size = match rand::random::<u32>() % 10 {
                0..=5 => rand::random::<u32>() % 1500 + 64, // Small packets (64-1563 bytes)
                6..=8 => rand::random::<u32>() % 8000 + 1500, // Medium packets (1.5-9.5KB)
                _ => rand::random::<u32>() % 60000 + 9000,  // Large packets (9-69KB)
            };

            total_bytes += packet_size as u64;

            absolute_counter_with_rate!(
                "network_bytes_high_freq",
                total_bytes as f64,
                "interface",
                "eth0"
            );
        }
    });

    // Example 7: Error tracking at moderate frequency
    tokio::spawn(async move {
        describe_counter!(
            "errors_moderate_freq",
            Unit::Count,
            "Errors at moderate frequency"
        );
        describe_gauge!(
            "errors_moderate_freq_rate_per_sec",
            Unit::CountPerSecond,
            "Error rate at moderate frequency"
        );

        let mut error_count = 0u64;
        let mut interval_timer = interval(Duration::from_millis(50)); // 20 calls/sec

        loop {
            interval_timer.tick().await;

            // Errors occur less frequently
            if rand::random::<f64>() < 0.1 {
                // 10% chance = ~2 errors per second
                error_count += 1;

                let error_types = ["timeout", "validation", "database", "network", "auth"];
                let error_type = error_types[rand::random::<u32>() as usize % error_types.len()];

                absolute_counter_with_rate!(
                    "errors_moderate_freq",
                    error_count as f64,
                    "type",
                    error_type
                );
            }
        }
    });

    // Statistics reporter - logs current rates every 10 seconds
    tokio::spawn(async move {
        let mut stats_timer = interval(Duration::from_secs(10));
        let start_time = Instant::now();

        loop {
            stats_timer.tick().await;

            let elapsed = start_time.elapsed().as_secs();
            let ultra_total = ultra_high_freq_counter.load(Ordering::Relaxed);
            let burst_total = burst_counter.load(Ordering::Relaxed);
            let steady_total = steady_counter.load(Ordering::Relaxed);
            let variable_total = variable_counter.load(Ordering::Relaxed);

            info!("=== PERFORMANCE STATS ({}s elapsed) ===", elapsed);
            info!(
                "Ultra High Freq: {} events ({:.1} events/sec)",
                ultra_total,
                ultra_total as f64 / elapsed as f64
            );
            info!(
                "Burst Pattern:   {} events ({:.1} events/sec)",
                burst_total,
                burst_total as f64 / elapsed as f64
            );
            info!(
                "Steady Rate:     {} events ({:.1} events/sec)",
                steady_total,
                steady_total as f64 / elapsed as f64
            );
            info!(
                "Variable Rate:   {} events ({:.1} events/sec)",
                variable_total,
                variable_total as f64 / elapsed as f64
            );
            info!("==========================================");
        }
    });

    HttpServer::new(|| {
        let dashboard_input = DashboardInput {
            buckets_for_metrics: vec![
                (
                    Matcher::Prefix("ultra_high_frequency".to_string()),
                    &[100.0, 500.0, 1000.0, 5000.0, 10000.0, 50000.0],
                ),
                (
                    Matcher::Prefix("burst_pattern".to_string()),
                    &[50.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0],
                ),
                (
                    Matcher::Prefix("api_requests_steady".to_string()),
                    &[10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0],
                ),
                (
                    Matcher::Prefix("variable_frequency".to_string()),
                    &[25.0, 75.0, 150.0, 300.0, 600.0, 1200.0],
                ),
                (
                    Matcher::Prefix("network_bytes".to_string()),
                    &[1024.0, 8192.0, 65536.0, 524288.0, 4194304.0, 33554432.0],
                ),
            ],
        };

        let metrics_actix_dashboard = create_metrics_actx_scope(&dashboard_input).unwrap();
        App::new()
            .route("/", web::get().to(hello))
            .route("/status", web::get().to(status))
            .service(metrics_actix_dashboard)
    })
    .bind(("127.0.0.1", 8084))?
    .run()
    .await
}
