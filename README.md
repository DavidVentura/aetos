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

## Quick reference

### Struct-level

- `#[metrics(prefix = "...")]`: Add prefix to all metric names

### Field-level

- `#[counter(help = "...")]`: Mark field as a counter metric
- `#[gauge(help = "...")]`: Mark field as a gauge metric
- `name = "..."`: Override the metric name (optional)
- `label = "..."`: For single-label collections, specify the label name (if unset, use the field name)

The `help` parameter is **required** for all counter and gauge attributes.

## Crate features

`no-escaping`: By default, label values are scanned for `"` and `\` to ensure valid Prometheus syntax. If you can guarantee your data is clean, enable this feature to skip the scan for a minor performance boost.

## Shortcomings

The macro allows you to add `label = ...` for scalar types (eg: `u64`), but it will ignore the label. This simplifies implementation.

```rust
#[metrics]
struct Metrics {
    #[counter(help = "", label = "this is ignored")]
    my_counter: u64
}
```

If you have an idea of how to fix this, I'd be glad to hear it
