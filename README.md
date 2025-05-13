[![Crates.io](https://img.shields.io/crates/v/metrics-rs-dashboard-actix.svg)](https://crates.io/crates/metrics-rs-dashboard-actix)
[![Docs.rs](https://docs.rs/metrics-rs-dashboard-actix/badge.svg)](https://docs.rs/metrics-rs-dashboard-actix)
[![Crates.io Total Downloads](https://img.shields.io/crates/d/metrics-rs-dashboard-actix)]

# metrics-rs-dashboard-actix
A Rust library for integrating metrics and a visualization dashboard. This crate provides a simple way to set up Prometheus metrics and expose them through an API endpoint, as well as a web dashboard for real-time metrics visualization. Actix is used only for exposing the endpoints and dashboard - you can use this library even if your main application isn't built with Actix.
![Screenshot](https://github.com/singerxt/metrics-rs-dashboard-actix/blob/main/doc/screenshot.png?raw=true)

## Features

- Easy integration with any Rust application (Actix currently required for dashboard exposure only)
- Real-time metrics visualization dashboard with unit-aware charts
- Prometheus metrics endpoint
- Support for custom histogram buckets
- Unit support for all metric types (displayed in charts)
- Low overhead metrics collection
- Full compatibility with the `metrics` ecosystem
- No custom recorders, leveraging the existing metrics-prometheus-exporter

## Inspiration

This library is inspired by [metrics-dashboard-rs](https://github.com/giangndm/metrics-dashboard-rs) but with key differences:
- Uses `metrics-exporter-prometheus` instead of `prometheus` directly for less overhead
- Adds support for histograms with custom buckets

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
metrics-rs-dashboard-actix = "0.1.3"
```

or

```
cargo add metrics-rs-dashboard-actix
```

## Quick Start

```rust
use actix_web::{App, HttpServer};
use metrics::{counter, histogram};
use metrics_actix_dashboard::{create_metrics_actx_scope, DashboardInput};
use metrics_exporter_prometheus::Matcher;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Configure custom histogram buckets (optional)
    let dashboard_input = DashboardInput {
        buckets_for_metrics: vec![(
            Matcher::Prefix("request_latency".to_string()),
            &[10.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0],
        )],
    };

    // Create your Actix web app with the metrics scope
    // Note: Actix is currently required only for exposing the dashboard and metrics endpoints
    HttpServer::new(|| {
        let metrics_scope = create_metrics_actx_scope(&dashboard_input).unwrap();

        App::new()
            .service(metrics_scope)
            // ... your other routes
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
```

## Using the Dashboard

Once your application is running, you can:

1. Access the metrics dashboard at: `http://localhost:8080/metrics/dashboard`
2. View Prometheus metrics at: `http://localhost:8080/metrics/prometheus`

Note that while you can use the metrics collection functionality in any Rust application, Actix Web is currently required to expose the dashboard and metrics endpoints.

## Actix Web Integration

This library uses Actix Web solely for exposing the dashboard and metrics endpoints. You can use the metrics collection functionality in any Rust application, regardless of whether your main application uses Actix or not. However, at this moment, Actix Web is required to expose the dashboard and metrics API endpoints.

Future versions may provide additional integration options for other web frameworks.

## Grouping Counter and Gauge metrics with Units

You can use type label to group counter or gauges into single chart. You can also add units to your metrics using the `describe_*` macros:

```rust
// Define a gauge with milliseconds unit (will be displayed on charts)
describe_gauge!("request_latency", Unit::Milliseconds, "HTTP request latency");
gauge!("request_latency", "type" => "success").set(42.0);
gauge!("request_latency", "type" => "error").set(100.0);

// Define a counter with bytes unit
describe_counter!("network_traffic", Unit::Bytes, "Network traffic volume");
counter!("network_traffic", "direction" => "inbound").increment(2048);
counter!("network_traffic", "direction" => "outbound").increment(1024);

// Define a histogram with seconds unit
describe_histogram!("processing_time", Unit::Seconds, "Task processing duration");
histogram!("processing_time", "priority" => "high").record(0.25);
histogram!("processing_time", "priority" => "low").record(1.5);
```

The dashboard will automatically display these units (milliseconds, bytes, seconds, etc.) on the charts, making your metrics more readable and contextual.

## Custom Histogram Buckets

You can define custom histogram buckets for more precise measurements:

```rust
use metrics_actix_dashboard::{DashboardInput, create_metrics_actx_scope};
use metrics_exporter_prometheus::Matcher;

let dashboard_input = DashboardInput {
    buckets_for_metrics: vec![
        // Custom buckets for request latency
        (
            Matcher::Prefix("request_latency".to_string()),
            &[10.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0],
        ),
        // Custom buckets for another metric
        (
            Matcher::Exact("database_query_time".to_string()),
            &[0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0],
        ),
    ],
};

let metrics_scope = create_metrics_actx_scope(&dashboard_input).unwrap();
```

## Available Units

The following units are available for your metrics and will be displayed on charts:

- `Unit::Count` - Default unit for counters and gauges
- `Unit::Bytes` - Bytes (for memory, file sizes, etc.)
- `Unit::Seconds` - Seconds (for durations)
- `Unit::Milliseconds` - Milliseconds (for shorter durations)
- `Unit::Microseconds` - Microseconds (for very short durations)
- `Unit::Nanoseconds` - Nanoseconds (for extremely short durations)
- `Unit::Percent` - Percentage values (0-100)
- `Unit::TebiBytes` - TiB (2^40 bytes)
- `Unit::GibiBytes` - GiB (2^30 bytes)
- `Unit::MebiBytes` - MiB (2^20 bytes)
- `Unit::KibiBytes` - KiB (2^10 bytes)

## Documentation

For more examples and detailed documentation, check out the [example code](examples/simple.rs).

## License

This project is licensed under the MIT License.
