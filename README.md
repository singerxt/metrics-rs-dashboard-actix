# metrics-actix-dashboard

![Screenshot](https://github.com/singerxt/metrics-rs-dashboard-actix/blob/main/doc/screenshot.png?raw=true)
A Rust library for integrating metrics and a visualization dashboard into Actix web applications. This crate provides a simple way to set up Prometheus metrics and expose them through an API endpoint, as well as a web dashboard for real-time metrics visualization.

## Features

- Easy integration with Actix Web applications
- Real-time metrics visualization dashboard
- Prometheus metrics endpoint
- Support for custom histogram buckets
- Low overhead metrics collection
- Full compatibility with the `metrics` ecosystem
- No custom recorders, leveraging the existing metrics-prometheus-exporter

## Inspiration

This library is inspired by [metrics-dashboard-rs](https://github.com/giangndm/metrics-dashboard-rs) but with key differences:
- Does not implement custom recorders
- Uses `metrics-exporter-prometheus` instead of `prometheus` directly for less overhead
- Adds support for histograms with custom buckets

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
metrics-actix-dashboard = "0.1.0"
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

## Defining Metrics

This library re-exports the `metrics` crate, so you can use all its functionality:

```rust
use metrics_actix_dashboard::metrics::{counter, histogram, gauge};

// Define and use counters
counter!("my_counter").increment(1);

// Define and use histograms
histogram!("request_latency", "milliseconds").record(42.0);

// Define and use gauges
gauge!("active_connections").set(5.0);
```

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

## Documentation

For more examples and detailed documentation, check out the [example code](examples/simple.rs).

## License

This project is licensed under the MIT License.
