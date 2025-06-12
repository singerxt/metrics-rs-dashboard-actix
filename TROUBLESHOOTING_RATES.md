# Troubleshooting Rate Tracking Issues

This guide helps you diagnose and fix common issues with rate tracking in the metrics-actix-dashboard library.

## Common Issue: Counter Value Increasing but Rate Always Zero

### Root Cause
The most common cause is that `counter_with_rate!` or `absolute_counter_with_rate!` calls are happening too frequently (less than 100ms apart). The `RateTracker` has a built-in safety mechanism that returns `0.0` for updates that occur within 100ms of each other to avoid division by zero and meaningless rate calculations.

### Diagnostic Steps

1. **Check Update Frequency**
   ```rust
   // BAD: Updates too frequently
   loop {
       counter_with_rate!("my_metric", 1.0);
       tokio::time::sleep(Duration::from_millis(10)).await; // Too fast!
   }
   ```

2. **Verify Threading Issues**
   If you're calling rate macros from different threads, they all share the same `RateTracker` instance via the global `RATE_TRACKERS` HashMap. Rapid calls from multiple threads can trigger the 100ms threshold.

3. **Check Your Usage Pattern**
   ```rust
   // GOOD: Proper spacing
   loop {
       counter_with_rate!("my_metric", 1.0);
       tokio::time::sleep(Duration::from_millis(150)).await; // >= 100ms
   }
   ```

## Solutions

### Solution 1: Batch Updates
Instead of updating on every single event, batch updates over time:

```rust
// Instead of this:
tokio::spawn(async move {
    loop {
        // Process single event
        process_event().await;
        counter_with_rate!("events_total", 1.0);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
});

// Do this:
tokio::spawn(async move {
    let mut event_count = 0u64;
    let mut ticker = tokio::time::interval(Duration::from_millis(200));
    
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                if event_count > 0 {
                    absolute_counter_with_rate!("events_total", event_count as f64);
                }
            }
            _ = process_event() => {
                event_count += 1;
            }
        }
    }
});
```

### Solution 2: Separate Rate Tracking Thread
Decouple business logic from rate tracking:

```rust
// High-frequency processing thread
let counter = Arc::new(AtomicU64::new(0));
let processing_counter = counter.clone();
tokio::spawn(async move {
    loop {
        // Process at high frequency
        process_data().await;
        processing_counter.fetch_add(1, Ordering::Relaxed);
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
});

// Separate rate tracking thread
let rate_counter = counter.clone();
tokio::spawn(async move {
    let mut ticker = tokio::time::interval(Duration::from_millis(250));
    loop {
        ticker.tick().await;
        let current_total = rate_counter.load(Ordering::Relaxed);
        absolute_counter_with_rate!("data_processed_total", current_total as f64);
    }
});
```

### Solution 3: Conditional Rate Updates
Only update rates when enough time has passed:

```rust
tokio::spawn(async move {
    let mut last_rate_update = Instant::now();
    let rate_update_interval = Duration::from_millis(150);
    
    loop {
        // Process events at any frequency
        process_event().await;
        
        let now = Instant::now();
        if now.duration_since(last_rate_update) >= rate_update_interval {
            counter_with_rate!("events_processed", 1.0);
            last_rate_update = now;
        } else {
            // Just update the counter without rate tracking
            metrics::counter!("events_processed").increment(1);
        }
        
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
});
```

### Solution 4: Use Proper Macro for Your Use Case

#### For Incremental Values (Individual Events)
```rust
// Use counter_with_rate! for individual increments
counter_with_rate!("messages_sent", 1.0);
counter_with_rate!("bytes_processed", chunk_size as f64);
```

#### For Running Totals (Recommended for Most Cases)
```rust
// Use absolute_counter_with_rate! for running totals
let total = running_total.fetch_add(increment, Ordering::Relaxed) + increment;
absolute_counter_with_rate!("total_requests", total as f64);
```

## Debugging Rate Issues

### Enable Debug Logging
```rust
// Add this to see rate calculations
use log::debug;

// In your rate tracking code:
let rate = update_rate_tracker("my_metric", value, tracker_key);
debug!("Rate for my_metric: {} per second", rate);
```

### Check Prometheus Metrics
Visit `/metrics` endpoint and look for your rate gauges:
```
# HELP my_metric_rate_per_sec Rate of my_metric per second
# TYPE my_metric_rate_per_sec gauge
my_metric_rate_per_sec{label="value"} 0
```

If the rate is consistently 0, you have a timing issue.

### Manual Rate Calculation for Comparison
```rust
// Implement your own rate calculation to compare
let mut last_value = 0u64;
let mut last_time = Instant::now();

loop {
    let current_value = get_current_value();
    let now = Instant::now();
    
    let time_diff = now.duration_since(last_time).as_secs_f64();
    if time_diff > 0.0 {
        let value_diff = current_value - last_value;
        let manual_rate = value_diff as f64 / time_diff;
        
        // Compare with library rate
        let library_rate = update_rate_tracker("test_metric", current_value as f64, "test_key".to_string());
        
        println!("Manual rate: {}, Library rate: {}", manual_rate, library_rate);
        
        last_value = current_value;
        last_time = now;
    }
    
    tokio::time::sleep(Duration::from_millis(500)).await;
}
```

## Best Practices

### 1. Choose Appropriate Update Intervals
- For high-frequency events: Update rates every 200-500ms
- For moderate-frequency events: Update rates every 100-200ms
- For low-frequency events: Update rates every 1-5 seconds

### 2. Use Appropriate Metrics Types
- Use `absolute_counter_with_rate!` for running totals (most common)
- Use `counter_with_rate!` for incremental values only

### 3. Consider Label Cardinality
Each unique combination of labels creates a separate rate tracker:
```rust
// This creates separate rate trackers for each endpoint
absolute_counter_with_rate!("requests_total", count as f64, "endpoint", "/api/users");
absolute_counter_with_rate!("requests_total", count as f64, "endpoint", "/api/orders");
```

### 4. Monitor Memory Usage
Rate trackers are stored in a global HashMap. High label cardinality can lead to memory growth.

### 5. Use Consistent Timing
```rust
// GOOD: Consistent timing
let mut ticker = tokio::time::interval(Duration::from_millis(200));
loop {
    ticker.tick().await;
    absolute_counter_with_rate!("my_metric", value as f64);
}

// BAD: Inconsistent timing
loop {
    absolute_counter_with_rate!("my_metric", value as f64);
    tokio::time::sleep(Duration::from_millis(rand::random::<u64>() % 1000)).await;
}
```

## Testing Rate Calculations

### Unit Test Example
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_rate_calculation_timing() {
        let mut values = vec![0.0, 10.0, 25.0, 45.0];
        let mut rates = vec![];
        
        for (i, &value) in values.iter().enumerate() {
            let rate = update_rate_tracker("test_metric", value, "test_key".to_string());
            rates.push(rate);
            
            if i < values.len() - 1 {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
        
        // First rate should be 0 (no previous value)
        assert_eq!(rates[0], 0.0);
        
        // Subsequent rates should be > 0
        for &rate in &rates[1..] {
            assert!(rate > 0.0, "Rate should be positive, got: {}", rate);
        }
    }
}
```

## Examples

See the following example files for working implementations:
- `examples/improved_rate_metrics.rs` - Proper timing and threading patterns
- `examples/custom_rate_tracker.rs` - Advanced rate tracking with smoothing
- `examples/rate_metrics.rs` - Basic usage patterns

## Common Pitfalls

1. **Calling rate macros in tight loops without delays**
2. **Using `counter_with_rate!` for running totals instead of `absolute_counter_with_rate!`**
3. **Not accounting for threading when multiple threads update the same metric**
4. **Expecting immediate rate calculations on the first update**

## When to Use Manual Rate Calculation

Consider implementing your own rate calculation if:
- You need rates calculated over different time windows
- You need more sophisticated smoothing algorithms
- You need to handle very high-frequency updates (>1000 per second)
- You need sub-100ms rate updates for real-time systems

```rust
// Example of manual rate calculation
struct CustomRateTracker {
    window: VecDeque<(f64, Instant)>,
    window_duration: Duration,
}

impl CustomRateTracker {
    fn update(&mut self, value: f64) -> f64 {
        let now = Instant::now();
        self.window.push_back((value, now));
        
        // Remove old entries
        let cutoff = now - self.window_duration;
        while let Some((_, timestamp)) = self.window.front() {
            if *timestamp < cutoff {
                self.window.pop_front();
            } else {
                break;
            }
        }
        
        // Calculate rate
        if self.window.len() < 2 {
            return 0.0;
        }
        
        let (first_val, first_time) = self.window[0];
        let (last_val, last_time) = *self.window.back().unwrap();
        
        let time_diff = last_time.duration_since(first_time).as_secs_f64();
        if time_diff > 0.0 {
            (last_val - first_val) / time_diff
        } else {
            0.0
        }
    }
}
```

This approach gives you full control over the rate calculation logic and timing constraints.