use actix_web::{App, HttpResponse, HttpServer, Responder, web};
use log::info;
use metrics::{Unit, describe_counter, describe_gauge};
use metrics_exporter_prometheus::Matcher;
use metrics_rs_dashboard_actix::{DashboardInput, create_metrics_actx_scope};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::interval;

/// Enhanced rate tracker that handles rapid updates and provides smoothed rates
#[derive(Debug)]
struct EnhancedRateTracker {
    values: Vec<(f64, Instant)>,
    window_size: Duration,
    max_samples: usize,
}

impl EnhancedRateTracker {
    fn new(window_size: Duration, max_samples: usize) -> Self {
        Self {
            values: Vec::with_capacity(max_samples),
            window_size,
            max_samples,
        }
    }

    fn update(&mut self, value: f64) -> f64 {
        let now = Instant::now();

        // Add new value
        self.values.push((value, now));

        // Remove old values outside the window
        let cutoff = now - self.window_size;
        self.values.retain(|(_, timestamp)| *timestamp > cutoff);

        // Limit the number of samples to prevent unbounded growth
        if self.values.len() > self.max_samples {
            let excess = self.values.len() - self.max_samples;
            self.values.drain(0..excess);
        }

        // Calculate rate if we have at least 2 samples
        if self.values.len() < 2 {
            return 0.0;
        }

        // Use linear regression for a smoother rate calculation
        self.calculate_rate()
    }

    fn calculate_rate(&self) -> f64 {
        if self.values.len() < 2 {
            return 0.0;
        }

        // Simple rate calculation using first and last values
        let (first_value, first_time) = &self.values[0];
        let (last_value, last_time) = self.values.last().unwrap();

        let time_diff = last_time.duration_since(*first_time).as_secs_f64();

        if time_diff <= 0.0 {
            return 0.0;
        }

        let value_diff = last_value - first_value;

        // For counters, ensure non-negative rates
        (value_diff / time_diff).max(0.0)
    }

    fn calculate_smoothed_rate(&self) -> f64 {
        if self.values.len() < 3 {
            return self.calculate_rate();
        }

        // Calculate rates between consecutive samples
        let mut rates = Vec::new();

        for window in self.values.windows(2) {
            let (v1, t1) = &window[0];
            let (v2, t2) = &window[1];

            let time_diff = t2.duration_since(*t1).as_secs_f64();
            if time_diff > 0.0 {
                let rate = (v2 - v1) / time_diff;
                rates.push(rate.max(0.0));
            }
        }

        if rates.is_empty() {
            return 0.0;
        }

        // Return median rate to reduce impact of outliers
        rates.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mid = rates.len() / 2;

        if rates.len() % 2 == 0 {
            (rates[mid - 1] + rates[mid]) / 2.0
        } else {
            rates[mid]
        }
    }
}

// Global storage for our custom rate trackers
type RateTrackerMap = Arc<Mutex<HashMap<String, EnhancedRateTracker>>>;

lazy_static::lazy_static! {
    static ref CUSTOM_RATE_TRACKERS: RateTrackerMap = Arc::new(Mutex::new(HashMap::new()));
}

// Helper function to update custom rate tracker
fn update_custom_rate_tracker(key: &str, value: f64, use_smoothing: bool) -> f64 {
    let mut trackers = CUSTOM_RATE_TRACKERS.lock().unwrap();
    let tracker = trackers
        .entry(key.to_string())
        .or_insert_with(|| EnhancedRateTracker::new(Duration::from_secs(10), 50));

    let rate = if use_smoothing {
        tracker.update(value);
        tracker.calculate_smoothed_rate()
    } else {
        tracker.update(value)
    };

    rate
}

// Macro for custom counter with enhanced rate tracking
macro_rules! custom_counter_with_rate {
    ($name:expr, $value:expr, $smooth:expr) => {{
        // Record the counter
        metrics::counter!($name).absolute($value as u64);

        // Calculate and record the rate
        let rate_name = format!("{}_rate_per_sec", $name);
        let tracker_key = format!("{}_default", $name);
        let rate = update_custom_rate_tracker(&tracker_key, $value, $smooth);
        metrics::gauge!(rate_name).set(rate);
    }};
    ($name:expr, $value:expr, $smooth:expr, $label_key:expr, $label_value:expr) => {{
        // Record the counter with labels
        metrics::counter!($name, $label_key => $label_value).absolute($value as u64);

        // Calculate and record the rate with labels
        let rate_name = format!("{}_rate_per_sec", $name);
        let tracker_key = format!("{}_{}_{}", $name, $label_key, $label_value);
        let rate = update_custom_rate_tracker(&tracker_key, $value, $smooth);
        metrics::gauge!(rate_name, $label_key => $label_value).set(rate);
    }};
}

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello from Custom Rate Tracker Example!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    info!("Starting server with custom rate tracker at http://127.0.0.1:8083");
    info!("Metrics available at: http://127.0.0.1:8083/metrics");
    info!("Dashboard available at: http://127.0.0.1:8083/dashboard");

    // Example 1: High-frequency updates with standard rate calculation
    let high_freq_counter = Arc::new(AtomicU64::new(0));
    {
        let counter = high_freq_counter.clone();
        tokio::spawn(async move {
            describe_counter!(
                "high_frequency_events",
                Unit::Count,
                "Events that occur at high frequency"
            );
            describe_gauge!(
                "high_frequency_events_rate_per_sec",
                Unit::CountPerSecond,
                "Rate of high frequency events per second"
            );

            loop {
                // Simulate very frequent events (every 10ms)
                for _ in 0..5 {
                    let current = counter.fetch_add(1, Ordering::Relaxed) + 1;

                    custom_counter_with_rate!(
                        "high_frequency_events",
                        current as f64,
                        false, // Don't use smoothing
                        "source",
                        "sensor_a"
                    );

                    tokio::time::sleep(Duration::from_millis(10)).await;
                }

                // Brief pause to create some variation
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        });
    }

    // Example 2: Bursty updates with smoothed rate calculation
    let bursty_counter = Arc::new(AtomicU64::new(0));
    {
        let counter = bursty_counter.clone();
        tokio::spawn(async move {
            describe_counter!(
                "bursty_requests",
                Unit::Count,
                "Requests that come in bursts"
            );
            describe_gauge!(
                "bursty_requests_rate_per_sec",
                Unit::CountPerSecond,
                "Smoothed rate of bursty requests per second"
            );

            loop {
                // Simulate burst patterns
                let burst_size = rand::random::<u32>() % 100 + 10; // 10-110 requests

                for _ in 0..burst_size {
                    let current = counter.fetch_add(1, Ordering::Relaxed) + 1;

                    custom_counter_with_rate!(
                        "bursty_requests",
                        current as f64,
                        true, // Use smoothing
                        "type",
                        "api_call"
                    );

                    // Very rapid updates during burst
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }

                // Longer pause between bursts
                let pause = rand::random::<u64>() % 2000 + 500; // 0.5-2.5 second pause
                tokio::time::sleep(Duration::from_millis(pause)).await;
            }
        });
    }

    // Example 3: Comparison with different tracking methods
    let comparison_counter = Arc::new(AtomicU64::new(0));
    {
        let counter = comparison_counter.clone();
        tokio::spawn(async move {
            describe_counter!(
                "comparison_metric",
                Unit::Count,
                "Metric for comparing rate calculation methods"
            );
            describe_gauge!(
                "comparison_metric_standard_rate_per_sec",
                Unit::CountPerSecond,
                "Standard rate calculation"
            );
            describe_gauge!(
                "comparison_metric_smoothed_rate_per_sec",
                Unit::CountPerSecond,
                "Smoothed rate calculation"
            );

            let mut ticker = interval(Duration::from_millis(100));

            loop {
                ticker.tick().await;

                // Variable increment to create interesting patterns
                let increment = if rand::random::<f64>() < 0.2 {
                    rand::random::<u32>() % 50 + 1 // Large increment (20% chance)
                } else {
                    rand::random::<u32>() % 5 + 1 // Small increment (80% chance)
                };

                let current =
                    counter.fetch_add(increment as u64, Ordering::Relaxed) + increment as u64;

                // Standard rate tracking
                custom_counter_with_rate!(
                    "comparison_metric",
                    current as f64,
                    false,
                    "method",
                    "standard"
                );

                // Smoothed rate tracking
                custom_counter_with_rate!(
                    "comparison_metric",
                    current as f64,
                    true,
                    "method",
                    "smoothed"
                );
            }
        });
    }

    // Example 4: Multiple metrics with different characteristics
    tokio::spawn(async move {
        describe_counter!(
            "network_throughput_bytes",
            Unit::Bytes,
            "Network throughput in bytes"
        );
        describe_gauge!(
            "network_throughput_bytes_rate_per_sec",
            Unit::Count,
            "Network throughput rate in bytes per second"
        );

        let interfaces = ["eth0", "eth1", "wlan0"];
        let mut counters = [0u64; 3];
        let mut ticker = interval(Duration::from_millis(200));

        loop {
            ticker.tick().await;

            for (i, interface) in interfaces.iter().enumerate() {
                // Simulate different traffic patterns per interface
                let bytes = match i {
                    0 => rand::random::<u32>() % 10000 + 1000, // Steady high traffic
                    1 => {
                        if rand::random::<f64>() < 0.3 {
                            rand::random::<u32>() % 50000 + 5000 // Occasional large transfers
                        } else {
                            rand::random::<u32>() % 1000 + 100 // Mostly small traffic
                        }
                    }
                    2 => {
                        if rand::random::<f64>() < 0.1 {
                            rand::random::<u32>() % 5000 + 1000 // Sporadic usage
                        } else {
                            0
                        }
                    }
                    _ => 0,
                };

                if bytes > 0 {
                    counters[i] += bytes as u64;

                    custom_counter_with_rate!(
                        "network_throughput_bytes",
                        counters[i] as f64,
                        true, // Use smoothing for network metrics
                        "interface",
                        *interface
                    );
                }
            }
        }
    });

    // Example 5: Error rate tracking (infrequent events)
    tokio::spawn(async move {
        describe_counter!("application_errors", Unit::Count, "Application errors");
        describe_gauge!(
            "application_errors_rate_per_sec",
            Unit::CountPerSecond,
            "Application error rate per second"
        );

        let mut error_count = 0u64;
        let mut ticker = interval(Duration::from_millis(500));

        loop {
            ticker.tick().await;

            // Errors are infrequent
            if rand::random::<f64>() < 0.05 {
                // 5% chance
                error_count += 1;

                let error_type = match rand::random::<u32>() % 4 {
                    0 => "timeout",
                    1 => "validation",
                    2 => "database",
                    _ => "network",
                };

                custom_counter_with_rate!(
                    "application_errors",
                    error_count as f64,
                    true, // Use smoothing for sparse events
                    "type",
                    error_type
                );
            }
        }
    });

    HttpServer::new(|| {
        let dashboard_input = DashboardInput {
            buckets_for_metrics: vec![
                (
                    Matcher::Prefix("high_frequency".to_string()),
                    &[10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0],
                ),
                (
                    Matcher::Prefix("bursty".to_string()),
                    &[1.0, 10.0, 50.0, 100.0, 500.0, 1000.0],
                ),
                (
                    Matcher::Prefix("network_throughput".to_string()),
                    &[1024.0, 10240.0, 102400.0, 1048576.0, 10485760.0],
                ),
            ],
        };

        let metrics_actix_dashboard = create_metrics_actx_scope(&dashboard_input).unwrap();
        App::new()
            .route("/", web::get().to(hello))
            .service(metrics_actix_dashboard)
    })
    .bind(("127.0.0.1", 8083))?
    .run()
    .await
}
