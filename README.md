# Aetos

A Rust proc macro library for generating Prometheus metrics rendering code from annotated structs.

The main goals of this library are to:

- Minimize boilerplate
- Colocate metrics/label definitions with their metadata
- Provide type-safe metrics and labels

# Why?

Because having an eagle eat my liver sounds better than instantiating a global metrics registry

## Quick start

```rust
use aetos::{metrics, Label};

#[derive(Label)]
struct RequestLabels<'a> {
    method: &'a str,
    status: u32,
}

#[metrics]
struct MyMetrics<'a> {
    // scalar metric, no labels
    #[counter(help = "Total requests")]
    requests: u64,

    // vec metric, single label shorthand
    #[counter(help = "Events by type", label = "event_type")]
    events: Vec<(String, u64)>,

    // vec metric, multiple labels requires a Label type
    #[counter(help = "HTTP requests by method and status")]
    http_requests: Vec<(RequestLabels<'a>, u64)>,
}

fn main() {
    let metrics = MyMetrics {
        requests: 1000,
        events: vec![
            ("stake".to_string(), 10),
            ("unstake".to_string(), 5),
        ],
        http_requests: vec![
            (RequestLabels { method: "GET", status: 200 }, 150),
            (RequestLabels { method: "POST", status: 404 }, 3),
        ],
    };

    println!("{}", metrics);
}
```

Output:
```
# HELP requests Total requests
# TYPE requests counter
requests 1000
# HELP events Events by type
# TYPE events counter
events{event_type="stake"} 10
events{event_type="unstake"} 5
# HELP http_requests HTTP requests by method and status
# TYPE http_requests counter
http_requests{method="GET",status="200"} 150
http_requests{method="POST",status="404"} 3
```

## Collection Types

Aetos supports collection types following the rules from above:

- If there's a single label, requires `K: Display`
- If there are multiple labels, requires `K: Label`

Specifically, the collection types must implement `IntoIterator<&(K, V)>` or `IntoIterator<(&K, &V)>`. This
means that `Vec`, `HashMap`, `BTreeMap` and `&[]` work.

## Histograms

Histograms track value distributions across predefined buckets. Define them with `define_histogram!` and specify bucket boundaries:

```rust
use aetos::{define_histogram, metrics, Label};

#[derive(Label, Hash, Eq, PartialEq, Clone, Debug)]
struct ResponseLabel {
    endpoint: &'static str,
}

// Labeled histogram
define_histogram!(Latency<ResponseLabel> = [0.1, 0.5, 1.0, 5.0]);

// Unlabeled histogram (use unit type)
define_histogram!(QueueTime<()> = [0.01, 0.1, 1.0]);

#[metrics]
struct Metrics {
    #[histogram(help = "Response time by endpoint")]
    response_time: Latency,

    #[histogram(help = "Queue wait time")]
    queue_time: QueueTime,
}

fn main() {
    let mut m = Metrics {
        response_time: Latency::default(),
        queue_time: QueueTime::default(),
    };

    m.response_time.observe(ResponseLabel { endpoint: "/api" }, 0.25);
    m.queue_time.observe((), 0.05);

    println!("{}", m);
}
```

Output:
```
# HELP response_time Response time by endpoint
# TYPE response_time histogram
response_time_bucket{endpoint="/api",le="0.1"} 0
response_time_bucket{endpoint="/api",le="0.5"} 1
response_time_bucket{endpoint="/api",le="1"} 1
response_time_bucket{endpoint="/api",le="5"} 1
response_time_bucket{endpoint="/api",le="+Inf"} 1
response_time_sum{endpoint="/api"} 0.25
response_time_count{endpoint="/api"} 1
# HELP queue_time Queue wait time
# TYPE queue_time histogram
queue_time_bucket{le="0.01"} 0
queue_time_bucket{le="0.1"} 1
queue_time_bucket{le="1"} 1
queue_time_bucket{le="+Inf"} 1
queue_time_sum{} 0.05
queue_time_count{} 1
```

Unlike counters and gauges which are just fields that get rendered, histograms maintain internal state (a `HashMap<Label, HistogramData>`) and compute cumulative bucket counts when you call `.observe()`. This means histograms have a runtime cost. The bucket boundaries are validated at compile time, so at least you'll know early if you mess up the array.

You can also use `linear_buckets` and `exponential_buckets`
```
linear_buckets::<10>(0.1, 0.1);
// [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0]
exponential_buckets::<8>(0.001, 2.0);
// [0.001, 0.002, 0.004, 0.008, 0.016, 0.032, 0.064, 0.128]
```

## Quick reference

### Struct-level

- `#[metrics(prefix = "...")]`: Add prefix to all metric names

### Field-level

- `#[counter(help = "...")]`: Mark field as a counter metric
- `#[gauge(help = "...")]`: Mark field as a gauge metric
- `#[histogram(help = "...")]`: Mark field as a histogram metric
- `name = "..."`: Override the metric name (optional)
- `label = "..."`: For single-label collections, specify the label name (if unset, use the field name)

The `help` parameter is **required** for all metric attributes.

## Crate features

`no-escaping`: By default, label values are scanned for `"` and `\` to ensure valid Prometheus syntax. If you can guarantee your data is clean, enable this feature to skip the scan for a minor performance boost.

## Shortcomings

The macro does not currently validate mis-usage of the `label` attribute for scalars.

There's a hack in place that blocks usage of `label` with `u64`/`f64`/... but it does not work for newtypes or aliases

```rust,compile_fail
type MyU64 = u64;

#[metrics]
struct Metrics {
    #[counter(help = "Count", label = "x")]  // Compile error!
    my_counter: u64

    #[counter(help = "Count", label = "x")]  // No error, but label is ignored
    my_other_counter: MyU64
}
```

I don't know how to detect all scalar types at macro expansion time. If you do, let me know.
