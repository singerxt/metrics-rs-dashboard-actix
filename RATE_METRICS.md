# Rate Metrics Implementation

This document provides a comprehensive overview of the rate metrics functionality added to the metrics-actix-dashboard library.

## Overview

Rate metrics provide automatic per-second rate calculation from counter values, enabling real-time monitoring of throughput, request rates, data processing rates, and other time-based metrics. This feature bridges the gap between raw counter values and meaningful rate visualizations.

## Key Features

- **Automatic Rate Calculation**: Converts counter increments into per-second rates
- **Thread-Safe Tracking**: Uses internal rate trackers with proper synchronization
- **Visual Distinction**: Rate charts use teal-colored area charts in the dashboard
- **Unit-Aware Display**: Automatically formats rate units (requests/sec, bytes/sec, etc.)
- **Label Support**: Full support for metric labels in both counters and rates

## Architecture

### Components

1. **RateTracker**: Core struct that tracks counter values and timestamps to calculate rates
2. **Rate Macros**: Convenient macros for recording counters with automatic rate tracking
3. **RateChart Component**: Frontend component for visualizing rate metrics
4. **Global Storage**: Thread-safe storage for rate trackers using static variables

### Data Flow

```
Counter Value → RateTracker → Rate Calculation → Rate Gauge → Dashboard
```

1. User records a counter value using rate macros
2. RateTracker calculates the per-second rate based on value change and time elapsed
3. Both the original counter and calculated rate are recorded as metrics
4. Dashboard displays rate metrics with specialized visualization

## API Reference

### Macros

#### `counter_with_rate!`

Records an incremental counter value and its rate.

**Syntax:**
```rust
counter_with_rate!(metric_name, value);
counter_with_rate!(metric_name, value, label_key, label_value);
```

**Examples:**
```rust
// Simple increment with rate
counter_with_rate!("requests_total", 1.0);

// Increment with labels and rate
counter_with_rate!("requests_total", 1.0, "endpoint", "/api/users");
```

#### `absolute_counter_with_rate!`

Records an absolute counter value and its rate. Recommended for running totals.

**Syntax:**
```rust
absolute_counter_with_rate!(metric_name, absolute_value);
absolute_counter_with_rate!(metric_name, absolute_value, label_key, label_value);
```

**Examples:**
```rust
// Absolute counter with rate
absolute_counter_with_rate!("bytes_processed_total", 1024.0);

// Absolute counter with labels and rate
absolute_counter_with_rate!("db_queries_total", 42.0, "type", "SELECT");
```

### Utility Function

#### `update_rate_tracker`

Internal function used by macros to calculate rates. Not intended for direct use.

```rust
pub fn update_rate_tracker(_counter_name: &str, value: f64, tracker_key: String) -> f64
```

## Metric Generation

When using rate macros, two metrics are automatically created:

1. **Original Counter**: `{metric_name}` (Prometheus COUNTER type)
2. **Rate Gauge**: `{metric_name}_rate_per_sec` (Prometheus GAUGE type)

### Example

```rust
absolute_counter_with_rate!("http_requests_total", 100.0, "method", "GET");
```

Creates:
- `http_requests_total{method="GET"}` (counter = 100)
- `http_requests_total_rate_per_sec{method="GET"}` (gauge = calculated rate)

## Dashboard Integration

### Chart Recognition

The dashboard automatically recognizes rate metrics by checking for gauges with the `_rate_per_sec` suffix and renders them using the `RateChart` component.

### Visual Features

- **Chart Type**: Area chart with gradient fill
- **Color Scheme**: Teal (#17a2b8) for distinction from other metric types
- **Smooth Curves**: Smooth line interpolation for better visualization
- **Unit Formatting**: Intelligent rate unit display (requests/sec, bytes/sec, etc.)
- **Precision Handling**: Appropriate decimal precision based on rate magnitude

## Implementation Examples

### HTTP Request Rate Monitoring

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use metrics::{describe_counter, Unit};
use metrics_actix_dashboard::absolute_counter_with_rate;

let request_counter = Arc::new(AtomicU64::new(0));

tokio::spawn(async move {
    describe_counter!("http_requests_total", Unit::Count, "Total HTTP requests");
    
    loop {
        // Simulate request processing
        let current_total = request_counter.fetch_add(1, Ordering::Relaxed) + 1;
        
        absolute_counter_with_rate!(
            "http_requests_total",
            current_total as f64,
            "endpoint",
            "/api/users"
        );
        
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
});
```

### Data Processing Throughput

```rust
use metrics_actix_dashboard::counter_with_rate;

loop {
    let bytes_processed = process_data_chunk().await;
    
    // Track incremental bytes processed with rate
    counter_with_rate!(
        "data_processing_bytes",
        bytes_processed as f64,
        "pipeline",
        "etl"
    );
    
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}
```

### Database Query Rate

```rust
let mut query_count = 0u64;

loop {
    let queries_in_batch = execute_query_batch().await;
    query_count += queries_in_batch;
    
    absolute_counter_with_rate!(
        "db_queries_total",
        query_count as f64,
        "type",
        "SELECT"
    );
}
```

## Technical Details

### Rate Calculation Algorithm

```rust
pub fn update(&mut self, new_value: f64) -> f64 {
    let now = Instant::now();
    let time_diff = now.duration_since(self.last_timestamp);
    
    // Minimum time threshold to avoid division by zero
    if time_diff < Duration::from_millis(100) {
        return 0.0;
    }
    
    let value_diff = new_value - self.last_value;
    let rate = value_diff / time_diff.as_secs_f64();
    
    self.last_value = new_value;
    self.last_timestamp = now;
    
    // Ensure non-negative rates for counters
    rate.max(0.0)
}
```

### Thread Safety

- Uses `Arc<Mutex<HashMap>>` for thread-safe rate tracker storage
- Atomic operations for shared counters in examples
- Lock-based synchronization for rate calculations

### Memory Management

- Rate trackers are stored globally and persist for the application lifetime
- Each unique metric name + label combination gets its own tracker
- Minimal memory overhead per tracked metric

## Best Practices

### When to Use Rate Metrics

- **HTTP Request Monitoring**: Track requests per second across different endpoints
- **Data Processing**: Monitor throughput in bytes/records per second
- **Database Operations**: Track query rates by type or table
- **Message Queue Processing**: Monitor message processing rates
- **Network Traffic**: Track bandwidth utilization rates

### Choosing Between Macros

- Use `counter_with_rate!` for incremental values (e.g., processing individual items)
- Use `absolute_counter_with_rate!` for running totals (recommended for most cases)

### Performance Considerations

- Rate calculation has minimal overhead (~100ms minimum interval prevents excessive calculations)
- Each rate tracker uses approximately 32 bytes of memory
- Lock contention is minimal due to separate trackers per metric+label combination

### Label Strategy

- Use labels to segment rates by meaningful dimensions (endpoint, method, type, etc.)
- Each unique label combination creates a separate rate tracker
- Consider label cardinality to avoid excessive memory usage

## Troubleshooting

### Common Issues

1. **Zero Rates**: Ensure sufficient time passes between updates (>100ms)
2. **Negative Rates**: Should not occur due to `max(0.0)` clamping
3. **Missing Rate Charts**: Verify metric names end with `_rate_per_sec`
4. **Label Mismatch**: Ensure labels are consistent between counter and rate metrics

### Debugging

Enable debug logging to see metric registration:
```rust
env_logger::init();
```

Check Prometheus endpoint directly:
```bash
curl http://localhost:8080/metrics/prometheus | grep rate_per_sec
```

## Future Enhancements

Potential improvements for future versions:

- Support for custom rate windows (e.g., per-minute, per-hour rates)
- Rate smoothing algorithms for more stable visualizations
- Integration with other web frameworks beyond Actix
- Rate alerting thresholds
- Historical rate trend analysis

## Contributing

When contributing to rate metrics functionality:

1. Maintain thread safety in all implementations
2. Add comprehensive tests for new rate calculation features
3. Update dashboard components for new visualization needs
4. Document any API changes in this file
5. Ensure backward compatibility with existing rate metrics

---

For implementation details, see the source code in `src/lib.rs` and examples in the `examples/` directory.