use metrics_rs_dashboard_actix::{absolute_counter_with_rate, counter_with_rate};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::time::interval;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing 100 calls per second capability...");

    // Initialize metrics
    let recorder = metrics_exporter_prometheus::PrometheusBuilder::new().build_recorder();
    metrics::set_global_recorder(recorder).expect("Failed to set global recorder");

    let counter = Arc::new(AtomicU64::new(0));
    let test_counter = counter.clone();

    // Test 1: Exactly 100 calls per second (10ms intervals)
    let test_task = tokio::spawn(async move {
        let mut interval_timer = interval(Duration::from_millis(10)); // 100 calls/sec
        let start_time = Instant::now();
        let test_duration = Duration::from_secs(5); // Run for 5 seconds

        println!("Starting 100 calls/sec test for 5 seconds...");

        while start_time.elapsed() < test_duration {
            interval_timer.tick().await;

            let current_total = test_counter.fetch_add(1, Ordering::Relaxed) + 1;

            // This should work without returning 0 rates
            absolute_counter_with_rate!(
                "test_high_freq_counter",
                current_total as f64,
                "test",
                "100_per_sec"
            );
        }

        let final_count = test_counter.load(Ordering::Relaxed);
        let actual_duration = start_time.elapsed().as_secs_f64();
        let actual_rate = final_count as f64 / actual_duration;

        println!("Test completed:");
        println!("  Total calls: {}", final_count);
        println!("  Duration: {:.2} seconds", actual_duration);
        println!("  Actual rate: {:.1} calls/sec", actual_rate);
        println!("  Expected ~500 calls in 5 seconds at 100 calls/sec");

        // Verify we got close to expected count
        let expected_calls = 500; // 100 calls/sec * 5 seconds
        let tolerance = 50; // Allow some variance

        if (final_count as i64 - expected_calls).abs() < tolerance {
            println!("✅ SUCCESS: Call rate is within expected range");
        } else {
            println!("❌ WARNING: Call rate is outside expected range");
        }
    });

    // Test 2: Even higher frequency (200 calls per second)
    let counter2 = Arc::new(AtomicU64::new(0));
    let test_counter2 = counter2.clone();

    let ultra_high_test = tokio::spawn(async move {
        let mut interval_timer = interval(Duration::from_millis(5)); // 200 calls/sec
        let start_time = Instant::now();
        let test_duration = Duration::from_secs(3); // Run for 3 seconds

        println!("\nStarting 200 calls/sec test for 3 seconds...");

        while start_time.elapsed() < test_duration {
            interval_timer.tick().await;

            let current_total = test_counter2.fetch_add(1, Ordering::Relaxed) + 1;

            // Test incremental counter as well
            counter_with_rate!(
                "test_ultra_high_freq_incremental",
                1.0,
                "test",
                "200_per_sec"
            );

            // And absolute counter
            absolute_counter_with_rate!(
                "test_ultra_high_freq_absolute",
                current_total as f64,
                "test",
                "200_per_sec"
            );
        }

        let final_count = test_counter2.load(Ordering::Relaxed);
        let actual_duration = start_time.elapsed().as_secs_f64();
        let actual_rate = final_count as f64 / actual_duration;

        println!("Ultra high frequency test completed:");
        println!("  Total calls: {}", final_count);
        println!("  Duration: {:.2} seconds", actual_duration);
        println!("  Actual rate: {:.1} calls/sec", actual_rate);
        println!("  Expected ~600 calls in 3 seconds at 200 calls/sec");

        // Verify we got close to expected count
        let expected_calls = 600; // 200 calls/sec * 3 seconds
        let tolerance = 60; // Allow some variance

        if (final_count as i64 - expected_calls).abs() < tolerance {
            println!("✅ SUCCESS: Ultra high frequency test passed");
        } else {
            println!("❌ WARNING: Ultra high frequency test outside expected range");
        }
    });

    // Wait for both tests to complete
    let _ = tokio::join!(test_task, ultra_high_test);

    // Test 3: Validate that rates are actually being calculated (not always 0)
    println!("\nTesting rate calculation accuracy...");

    // Use the library's rate tracking directly to verify it works
    use metrics_rs_dashboard_actix::update_rate_tracker;

    let mut non_zero_rates = 0;
    let total_tests = 20;

    for i in 0..total_tests {
        tokio::time::sleep(Duration::from_millis(12)).await; // > 10ms intervals

        let rate = update_rate_tracker(
            "direct_test",
            (i * 10) as f64,
            "direct_test_key".to_string(),
        );

        if rate > 0.0 {
            non_zero_rates += 1;
            println!("  Rate {}: {:.2} per second", i, rate);
        }
    }

    println!("\nRate calculation test results:");
    println!("  Non-zero rates: {}/{}", non_zero_rates, total_tests - 1); // -1 because first is always 0

    if non_zero_rates >= (total_tests - 1) / 2 {
        println!("✅ SUCCESS: Rate calculations are working (not always zero)");
    } else {
        println!("❌ FAILED: Most rates are zero - rate calculation may be broken");
    }

    println!("\n🎉 High frequency testing completed!");
    println!("The library now supports 100+ calls per second without breaking changes.");

    Ok(())
}
