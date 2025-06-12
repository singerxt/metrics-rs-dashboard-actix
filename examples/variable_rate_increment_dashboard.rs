use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use log::info;
use metrics::{Unit, describe_counter, describe_gauge};
use metrics_rs_dashboard_actix::{DashboardInput, counter_with_rate, create_metrics_actx_scope};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("🚀 Variable Rate Increment Dashboard (2000-6000/sec)")
}

async fn dashboard_status() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "status": "running",
        "frequency": "2000-6000 increments per second",
        "description": "Variable rate increment demonstration with thread::spawn",
        "increment_type": "counter_with_rate! only (increment by 1)",
        "endpoints": {
            "dashboard": "/dashboard",
            "metrics": "/metrics",
            "status": "/status",
            "stats": "/stats"
        },
        "features": [
            "Thread::spawn increment patterns",
            "Variable rates 2000-6000/sec",
            "Only counter_with_rate! (no absolute)",
            "Increment by 1 only",
            "No bucket definitions"
        ]
    }))
}

async fn live_stats(
    fast_counter: web::Data<Arc<AtomicU64>>,
    variable_counter: web::Data<Arc<AtomicU64>>,
    burst_counter: web::Data<Arc<AtomicU64>>,
    start_time: web::Data<Arc<Instant>>,
) -> impl Responder {
    let now = Instant::now();
    let elapsed = now.duration_since(***start_time).as_secs_f64();

    let fast_total = fast_counter.load(Ordering::Relaxed);
    let variable_total = variable_counter.load(Ordering::Relaxed);
    let burst_total = burst_counter.load(Ordering::Relaxed);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    HttpResponse::Ok().json(json!({
        "timestamp": timestamp,
        "uptime_seconds": elapsed,
        "counters": {
            "fast_increment": {
                "total": fast_total,
                "rate_per_second": fast_total as f64 / elapsed,
                "target_range": "4000-6000/sec"
            },
            "variable_increment": {
                "total": variable_total,
                "rate_per_second": variable_total as f64 / elapsed,
                "target_range": "2000-5000/sec variable"
            },
            "burst_increment": {
                "total": burst_total,
                "rate_per_second": burst_total as f64 / elapsed,
                "target_range": "2000-6000/sec burst patterns"
            }
        },
        "performance": {
            "total_events": fast_total + variable_total + burst_total,
            "average_rate": (fast_total + variable_total + burst_total) as f64 / elapsed,
            "increment_method": "counter_with_rate! only, increment by 1"
        }
    }))
}

fn main() -> std::io::Result<()> {
    env_logger::init();

    println!("🚀 Starting Variable Rate Increment Dashboard");
    println!("==============================================");
    println!("Rate Range: 2000-6000 increments per second");
    println!("Method: counter_with_rate! increment by 1");
    println!("Threads: thread::spawn with upper scope vars");
    println!("Server: http://127.0.0.1:8087");
    println!("Dashboard: http://127.0.0.1:8087/dashboard");
    println!("==============================================");

    // Shared counters defined in upper scope
    let fast_increment_counter = Arc::new(AtomicU64::new(0));
    let variable_increment_counter = Arc::new(AtomicU64::new(0));
    let burst_increment_counter = Arc::new(AtomicU64::new(0));
    let start_time = Arc::new(Instant::now());

    // Pattern 1: Fast steady rate (4000-6000/sec) using thread::spawn
    {
        let counter = fast_increment_counter.clone(); // Upper scope definition

        thread::spawn(move || {
            info!("🏃 Starting fast increment thread (4000-6000/sec)");

            describe_counter!(
                "fast_increment_events",
                Unit::Count,
                "Fast increment events at 4000-6000 per second"
            );
            describe_gauge!(
                "fast_increment_events_rate",
                Unit::CountPerSecond,
                "Rate of fast increment events"
            );

            let mut current_rate = 4000; // Start at 4000/sec
            let mut rate_direction = true; // true = increasing, false = decreasing
            let mut last_rate_change = Instant::now();

            loop {
                // Calculate sleep duration based on current rate
                let sleep_micros = 1_000_000 / current_rate;
                thread::sleep(Duration::from_micros(sleep_micros));

                // Increment counter by 1 using counter_with_rate!
                counter.fetch_add(1, Ordering::Relaxed);
                counter_with_rate!("fast_increment_events", 1.0, "pattern", "steady_fast");

                // Change rate every 5 seconds between 4000-6000/sec
                if last_rate_change.elapsed() >= Duration::from_secs(5) {
                    if rate_direction {
                        current_rate += 500;
                        if current_rate >= 6000 {
                            rate_direction = false;
                        }
                    } else {
                        current_rate -= 500;
                        if current_rate <= 4000 {
                            rate_direction = true;
                        }
                    }
                    info!("🔄 Fast thread rate changed to {}/sec", current_rate);
                    last_rate_change = Instant::now();
                }
            }
        });
    }

    // Pattern 2: Variable rate (2000-5000/sec) using thread::spawn
    {
        let counter = variable_increment_counter.clone(); // Upper scope definition

        thread::spawn(move || {
            info!("📈 Starting variable increment thread (2000-5000/sec)");

            describe_counter!(
                "variable_increment_events",
                Unit::Count,
                "Variable increment events at 2000-5000 per second"
            );
            describe_gauge!(
                "variable_increment_events_rate",
                Unit::CountPerSecond,
                "Rate of variable increment events"
            );

            let rate_patterns = [
                (2000, "slow", 8),    // 2000/sec for 8 seconds
                (3000, "medium", 6),  // 3000/sec for 6 seconds
                (4500, "fast", 4),    // 4500/sec for 4 seconds
                (5000, "fastest", 3), // 5000/sec for 3 seconds
                (2500, "cooling", 5), // 2500/sec for 5 seconds
            ];

            loop {
                for (rate_per_sec, pattern_name, duration_secs) in &rate_patterns {
                    info!(
                        "🔧 Variable pattern: {} ({}/sec for {}s)",
                        pattern_name, rate_per_sec, duration_secs
                    );

                    let sleep_micros = 1_000_000 / rate_per_sec;
                    let pattern_start = Instant::now();

                    while pattern_start.elapsed() < Duration::from_secs(*duration_secs) {
                        thread::sleep(Duration::from_micros(sleep_micros));

                        // Increment by 1 using counter_with_rate!
                        counter.fetch_add(1, Ordering::Relaxed);
                        counter_with_rate!(
                            "variable_increment_events",
                            1.0,
                            "pattern",
                            *pattern_name
                        );
                    }
                }
            }
        });
    }

    // Pattern 3: Burst patterns (2000-6000/sec) using thread::spawn
    {
        let counter = burst_increment_counter.clone(); // Upper scope definition

        thread::spawn(move || {
            info!("💥 Starting burst increment thread (2000-6000/sec bursts)");

            describe_counter!(
                "burst_increment_events",
                Unit::Count,
                "Burst increment events with 2000-6000/sec patterns"
            );
            describe_gauge!(
                "burst_increment_events_rate",
                Unit::CountPerSecond,
                "Rate of burst increment events"
            );

            loop {
                // Normal period: 2000/sec for 10 seconds
                info!("🟢 Burst: Normal period (2000/sec for 10s)");
                let normal_start = Instant::now();
                while normal_start.elapsed() < Duration::from_secs(10) {
                    thread::sleep(Duration::from_micros(500)); // 2000/sec
                    counter.fetch_add(1, Ordering::Relaxed);
                    counter_with_rate!("burst_increment_events", 1.0, "phase", "normal");
                }

                // Ramp up: 2000 -> 6000/sec over 5 seconds
                info!("🟡 Burst: Ramp up (2000->6000/sec over 5s)");
                let ramp_start = Instant::now();
                while ramp_start.elapsed() < Duration::from_secs(5) {
                    let elapsed_ratio = ramp_start.elapsed().as_secs_f64() / 5.0;
                    let current_rate = 2000.0 + (4000.0 * elapsed_ratio); // 2000 + (0 to 4000)
                    let sleep_micros = (1_000_000.0 / current_rate) as u64;

                    thread::sleep(Duration::from_micros(sleep_micros));
                    counter.fetch_add(1, Ordering::Relaxed);
                    counter_with_rate!("burst_increment_events", 1.0, "phase", "ramp_up");
                }

                // Peak burst: 6000/sec for 3 seconds
                info!("🔴 Burst: Peak burst (6000/sec for 3s)");
                let peak_start = Instant::now();
                while peak_start.elapsed() < Duration::from_secs(3) {
                    thread::sleep(Duration::from_micros(167)); // ~6000/sec
                    counter.fetch_add(1, Ordering::Relaxed);
                    counter_with_rate!("burst_increment_events", 1.0, "phase", "peak");
                }

                // Ramp down: 6000 -> 2000/sec over 4 seconds
                info!("🟡 Burst: Ramp down (6000->2000/sec over 4s)");
                let ramp_down_start = Instant::now();
                while ramp_down_start.elapsed() < Duration::from_secs(4) {
                    let elapsed_ratio = ramp_down_start.elapsed().as_secs_f64() / 4.0;
                    let current_rate = 6000.0 - (4000.0 * elapsed_ratio); // 6000 - (0 to 4000)
                    let sleep_micros = (1_000_000.0 / current_rate) as u64;

                    thread::sleep(Duration::from_micros(sleep_micros));
                    counter.fetch_add(1, Ordering::Relaxed);
                    counter_with_rate!("burst_increment_events", 1.0, "phase", "ramp_down");
                }

                // Recovery: 2000/sec for 8 seconds
                info!("🔵 Burst: Recovery (2000/sec for 8s)");
                let recovery_start = Instant::now();
                while recovery_start.elapsed() < Duration::from_secs(8) {
                    thread::sleep(Duration::from_micros(500)); // 2000/sec
                    counter.fetch_add(1, Ordering::Relaxed);
                    counter_with_rate!("burst_increment_events", 1.0, "phase", "recovery");
                }
            }
        });
    }

    // Statistics reporter thread
    {
        let fast_counter = fast_increment_counter.clone();
        let variable_counter = variable_increment_counter.clone();
        let burst_counter = burst_increment_counter.clone();
        let start = start_time.clone();

        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(15));

                let elapsed = start.elapsed().as_secs_f64();
                let fast_total = fast_counter.load(Ordering::Relaxed);
                let variable_total = variable_counter.load(Ordering::Relaxed);
                let burst_total = burst_counter.load(Ordering::Relaxed);

                let total_events = fast_total + variable_total + burst_total;

                println!(
                    "\n📊 VARIABLE RATE DASHBOARD STATS ({:.1}s elapsed)",
                    elapsed
                );
                println!("═════════════════════════════════════════════════════");
                println!(
                    "🏃 Fast Pattern:     {:>10} events ({:>7.1}/sec)",
                    fast_total,
                    fast_total as f64 / elapsed
                );
                println!(
                    "📈 Variable Pattern: {:>10} events ({:>7.1}/sec)",
                    variable_total,
                    variable_total as f64 / elapsed
                );
                println!(
                    "💥 Burst Pattern:    {:>10} events ({:>7.1}/sec)",
                    burst_total,
                    burst_total as f64 / elapsed
                );
                println!("─────────────────────────────────────────────────────");
                println!(
                    "📊 TOTAL:            {:>10} events ({:>7.1}/sec)",
                    total_events,
                    total_events as f64 / elapsed
                );
                println!("═════════════════════════════════════════════════════\n");
            }
        });
    }

    // Start the web server
    actix_web::rt::System::new().block_on(async {
        HttpServer::new(move || {
            // Create dashboard input without bucket definitions
            let dashboard_input = DashboardInput::default();

            let metrics_actix_dashboard = create_metrics_actx_scope(&dashboard_input).unwrap();

            App::new()
                .app_data(web::Data::new(fast_increment_counter.clone()))
                .app_data(web::Data::new(variable_increment_counter.clone()))
                .app_data(web::Data::new(burst_increment_counter.clone()))
                .app_data(web::Data::new(start_time.clone()))
                .route("/", web::get().to(hello))
                .route("/status", web::get().to(dashboard_status))
                .route("/stats", web::get().to(live_stats))
                .service(metrics_actix_dashboard)
        })
        .bind(("127.0.0.1", 8087))?
        .run()
        .await
    })
}
