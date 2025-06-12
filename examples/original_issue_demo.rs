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
    HttpResponse::Ok().body("Original Issue Demo - Counter increasing but rate was zero")
}

async fn metrics_status() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "issue": "Counter value increasing but rate always zero",
        "cause": "counter_with_rate called from different threads at high frequency",
        "solution": "Sliding window rate tracker handles 100+ calls/sec",
        "status": "FIXED",
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

    info!("🚀 Original Issue Demo: Counter increasing but rate was zero");
    info!("Problem: counter_with_rate might be called from different threads at high frequency");
    info!("Solution: New sliding window rate tracker handles 100+ calls per second");
    info!("");
    info!("Server: http://127.0.0.1:8085");
    info!("Metrics: http://127.0.0.1:8085/metrics");
    info!("Dashboard: http://127.0.0.1:8085/dashboard");
    info!("Status: http://127.0.0.1:8085/status");

    let shared_counter = Arc::new(AtomicU64::new(0));

    // Scenario 1: The original problematic pattern
    // Multiple threads calling counter_with_rate at high frequency
    info!("📊 Starting multi-threaded high-frequency counter demo...");

    for thread_id in 0..5 {
        let counter = shared_counter.clone();

        tokio::spawn(async move {
            describe_counter!(
                "multi_thread_events",
                Unit::Count,
                "Events from multiple threads (original issue scenario)"
            );
            describe_gauge!(
                "multi_thread_events_rate_per_sec",
                Unit::CountPerSecond,
                "Rate of multi-thread events (should NOT be zero)"
            );

            // Each thread updates every 8ms = 125 calls/sec per thread
            // Total: 5 threads × 125 calls/sec = 625 calls/sec
            let mut thread_interval = interval(Duration::from_millis(8));

            loop {
                thread_interval.tick().await;

                let current_total = counter.fetch_add(1, Ordering::Relaxed) + 1;

                // This was the problematic call that returned 0.0 rates
                // Now it should work correctly even at 625 calls/sec total
                match thread_id {
                    0 => absolute_counter_with_rate!(
                        "multi_thread_events",
                        current_total as f64,
                        "thread_id",
                        "thread_0"
                    ),
                    1 => absolute_counter_with_rate!(
                        "multi_thread_events",
                        current_total as f64,
                        "thread_id",
                        "thread_1"
                    ),
                    2 => absolute_counter_with_rate!(
                        "multi_thread_events",
                        current_total as f64,
                        "thread_id",
                        "thread_2"
                    ),
                    3 => absolute_counter_with_rate!(
                        "multi_thread_events",
                        current_total as f64,
                        "thread_id",
                        "thread_3"
                    ),
                    4 => absolute_counter_with_rate!(
                        "multi_thread_events",
                        current_total as f64,
                        "thread_id",
                        "thread_4"
                    ),
                    _ => absolute_counter_with_rate!(
                        "multi_thread_events",
                        current_total as f64,
                        "thread_id",
                        "thread_other"
                    ),
                }
            }
        });
    }

    // Scenario 2: Single thread but very high frequency
    info!("⚡ Starting ultra-high frequency single thread demo...");

    let ultra_fast_counter = Arc::new(AtomicU64::new(0));
    {
        let counter = ultra_fast_counter.clone();
        tokio::spawn(async move {
            describe_counter!(
                "ultra_fast_events",
                Unit::Count,
                "Ultra fast events (150 calls/sec from single thread)"
            );
            describe_gauge!(
                "ultra_fast_events_rate_per_sec",
                Unit::CountPerSecond,
                "Rate of ultra fast events (should be ~150/sec)"
            );

            // Update every 6.67ms = 150 calls/sec
            let mut fast_interval = interval(Duration::from_millis(7));

            loop {
                fast_interval.tick().await;

                let current_total = counter.fetch_add(1, Ordering::Relaxed) + 1;

                // This should show rates around 150/sec, not 0.0
                absolute_counter_with_rate!(
                    "ultra_fast_events",
                    current_total as f64,
                    "source",
                    "single_thread"
                );
            }
        });
    }

    // Scenario 3: Burst pattern that was especially problematic
    info!("💥 Starting burst pattern demo...");

    let burst_counter = Arc::new(AtomicU64::new(0));
    {
        let counter = burst_counter.clone();
        tokio::spawn(async move {
            describe_counter!(
                "burst_pattern_events",
                Unit::Count,
                "Burst pattern events (rapid bursts followed by pauses)"
            );
            describe_gauge!(
                "burst_pattern_events_rate_per_sec",
                Unit::CountPerSecond,
                "Rate of burst events (should show meaningful rates during bursts)"
            );

            loop {
                // Rapid burst: 50 events in 50ms (1000 events/sec during burst)
                for _ in 0..50 {
                    let current_total = counter.fetch_add(1, Ordering::Relaxed) + 1;

                    absolute_counter_with_rate!(
                        "burst_pattern_events",
                        current_total as f64,
                        "pattern",
                        "burst"
                    );

                    tokio::time::sleep(Duration::from_millis(1)).await;
                }

                // Pause between bursts
                tokio::time::sleep(Duration::from_millis(2000)).await;
            }
        });
    }

    // Scenario 4: Incremental counter pattern
    info!("🔢 Starting incremental counter demo...");

    tokio::spawn(async move {
        describe_counter!(
            "incremental_fast_events",
            Unit::Count,
            "Incremental events at high frequency"
        );
        describe_gauge!(
            "incremental_fast_events_rate_per_sec",
            Unit::CountPerSecond,
            "Rate of incremental events"
        );

        // Use counter_with_rate! for incremental values at high frequency
        let mut incremental_interval = interval(Duration::from_millis(12)); // ~83 calls/sec

        loop {
            incremental_interval.tick().await;

            // Random increment size to make it more realistic
            let increment = if rand::random::<f64>() < 0.1 {
                rand::random::<u32>() % 10 + 5 // Occasional large increment
            } else {
                1 // Usually increment by 1
            };

            // This should show meaningful rates, not always 0.0
            counter_with_rate!(
                "incremental_fast_events",
                increment as f64,
                "type",
                "incremental"
            );
        }
    });

    // Statistics logger to show the fix is working
    {
        let multi_counter = shared_counter.clone();
        let ultra_counter = ultra_fast_counter.clone();
        let burst_counter_ref = burst_counter.clone();

        tokio::spawn(async move {
            let mut stats_interval = interval(Duration::from_secs(10));
            let start_time = Instant::now();

            loop {
                stats_interval.tick().await;

                let elapsed = start_time.elapsed().as_secs();
                let multi_total = multi_counter.load(Ordering::Relaxed);
                let ultra_total = ultra_counter.load(Ordering::Relaxed);
                let burst_total = burst_counter_ref.load(Ordering::Relaxed);

                info!("📈 LIVE STATS ({}s elapsed):", elapsed);
                info!(
                    "  Multi-thread: {} events ({:.1}/sec) - Expected ~625/sec",
                    multi_total,
                    multi_total as f64 / elapsed as f64
                );
                info!(
                    "  Ultra-fast:   {} events ({:.1}/sec) - Expected ~150/sec",
                    ultra_total,
                    ultra_total as f64 / elapsed as f64
                );
                info!(
                    "  Burst:        {} events ({:.1}/sec) - Expected ~25/sec average",
                    burst_total,
                    burst_total as f64 / elapsed as f64
                );
                info!("  🎯 Check /metrics endpoint - rates should NOT be zero!");
                info!("  💡 Before fix: All rates would be 0.0");
                info!("  ✅ After fix: Rates show actual values");
                info!("");
            }
        });
    }

    HttpServer::new(|| {
        let dashboard_input = DashboardInput {
            buckets_for_metrics: vec![
                (
                    Matcher::Prefix("multi_thread".to_string()),
                    &[100.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0],
                ),
                (
                    Matcher::Prefix("ultra_fast".to_string()),
                    &[50.0, 100.0, 200.0, 500.0, 1000.0],
                ),
                (
                    Matcher::Prefix("burst_pattern".to_string()),
                    &[10.0, 50.0, 100.0, 500.0, 1000.0, 2000.0],
                ),
                (
                    Matcher::Prefix("incremental_fast".to_string()),
                    &[25.0, 50.0, 100.0, 200.0, 500.0],
                ),
            ],
        };

        let metrics_actix_dashboard = create_metrics_actx_scope(&dashboard_input).unwrap();
        App::new()
            .route("/", web::get().to(hello))
            .route("/status", web::get().to(metrics_status))
            .service(metrics_actix_dashboard)
    })
    .bind(("127.0.0.1", 8085))?
    .run()
    .await
}
