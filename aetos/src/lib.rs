//! # Aetos
//!
//! A Rust proc macro library for generating Prometheus metrics rendering code from annotated structs.
//!
//! ## Quick Start
//!
//! ```
//! use aetos::{metrics, Label};
//! use std::collections::HashMap;
//!
//! #[derive(Label)]
//! struct RequestLabels<'a> {
//!     method: &'a str,
//!     status: u32,
//! }
//!
//! #[metrics(prefix = "myapp")]  // Optional prefix for all metrics
//! struct MyMetrics<'a> {
//!     // Scalar metric, no labels
//!     #[counter(name = "requests_total", help = "Total requests")]  // name is optional
//!     requests: u64,
//!
//!     // HashMap metric, single label shorthand
//!     #[counter(help = "Events by type", label = "event_type")]
//!     events: HashMap<String, u64>,
//!
//!     // Vec metric, multiple labels requires a Label type
//!     #[counter(help = "HTTP requests by method and status")]
//!     http_requests: Vec<(RequestLabels<'a>, u64)>,
//! }
//!
//! let metrics = MyMetrics {
//!     requests: 1000,
//!     events: HashMap::from([
//!         ("add".to_string(), 10),
//!         ("remove".to_string(), 5),
//!     ]),
//!     http_requests: vec![
//!         (RequestLabels { method: "GET", status: 200 }, 150),
//!         (RequestLabels { method: "POST", status: 404 }, 3),
//!     ],
//! };
//!
//! println!("{}", metrics);
//! ```
//!
//! This outputs:
//! ```text
//! # HELP myapp_requests_total Total requests
//! # TYPE myapp_requests_total counter
//! myapp_requests_total 1000
//! # HELP myapp_events Events by type
//! # TYPE myapp_events counter
//! myapp_events{event_type="add"} 10
//! myapp_events{event_type="remove"} 5
//! # HELP myapp_http_requests HTTP requests by method and status
//! # TYPE myapp_http_requests counter
//! myapp_http_requests{method="GET",status="200"} 150
//! myapp_http_requests{method="POST",status="404"} 3
//! ```
//!
//! ## Collection Types
//!
//! Labeled metrics accept anything that implements `IntoIterator<&(K, V)>` or `IntoIterator<(&K, &V)>` (Vec, HashMap, BTreeMap, slices, etc.).
//!
//! - Single label: `K` implements `Display`
//! - Multiple labels: `K` implements `Label`
//!
//! ## Override Metric Names
//!
//! Use the `name` attribute to export a different metric name than the field name (see Quick Start example).
//!
//! ## Histograms
//!
//! Histograms track value distributions across predefined buckets:
//!
//! ```
//! use aetos::{define_histogram, metrics, Label};
//!
//! #[derive(Label, Hash, Eq, PartialEq, Clone, Debug)]
//! struct ResponseLabel {
//!     endpoint: &'static str,
//! }
//!
//! // Labeled histogram
//! define_histogram!(Latency<ResponseLabel> = [0.1, 0.5, 1.0, 5.0]);
//!
//! // Unlabeled histogram
//! define_histogram!(QueueTime<()> = [0.01, 0.1, 1.0]);
//!
//! #[metrics]
//! struct Metrics {
//!     #[histogram(help = "Response time by endpoint")]
//!     response_time: Latency,
//!
//!     #[histogram(help = "Queue wait time")]
//!     queue_time: QueueTime,
//! }
//!
//! let mut m = Metrics {
//!     response_time: Latency::default(),
//!     queue_time: QueueTime::default(),
//! };
//!
//! m.response_time.observe(ResponseLabel { endpoint: "/api" }, 0.25);
//! m.queue_time.observe((), 0.05);
//!
//! println!("{}", m);
//! ```
//! If you don't want to manually specify buckets, you can use these functions to
//! generate them
//!
//! ```
//! use aetos::{define_histogram, linear_buckets};
//!
//! define_histogram!(RequestLatency<()> = linear_buckets::<10>(0.1, 0.1));
//! // Generates buckets: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0]
//! ```
//!
//! ```
//! use aetos::{define_histogram, exponential_buckets};
//!
//! define_histogram!(ResponseSize<()> = exponential_buckets::<8>(0.001, 2.0));
//! // Generates buckets: [0.001, 0.002, 0.004, 0.008, 0.016, 0.032, 0.064, 0.128]
//! ```

#[doc(hidden)]
pub use aetos_macro::{Label, metrics};

#[doc(hidden)]
pub use aetos_core as core;

pub use aetos_core::{exponential_buckets, linear_buckets};

/// Defines a histogram type with compile-time validated bucket boundaries.
///
/// This macro creates a newtype wrapper around `Histogram<L, N>` with specific bucket
/// boundaries. The bucket values are validated at compile time to ensure they are in
/// strictly ascending order.
///
/// # Syntax
///
/// ```text
/// define_histogram!(HistogramName<LabelType> = [bucket1, bucket2, ...]);
/// ```
///
/// # Examples
///
/// Define a histogram with custom labels:
/// ```
/// use aetos::{define_histogram, Label};
///
/// #[derive(Label, Hash, Eq, PartialEq, Clone, Debug)]
/// struct HttpLabel {
///     method: &'static str,
///     status: u16,
/// }
///
/// define_histogram!(HttpLatency<HttpLabel> = [0.05, 0.1, 0.5, 1.0, 5.0]);
/// ```
///
/// Define an unlabeled histogram:
/// ```
/// use aetos::define_histogram;
///
/// define_histogram!(ResponseTime<()> = [0.1, 0.5, 1.0]);
/// ```
///
/// Invalid bucket ordering fails at compile time:
/// ```compile_fail
/// use aetos::define_histogram;
///
/// // This will fail because buckets are not in ascending order
/// define_histogram!(Bad<()> = [1.0, 0.5, 2.0]);
/// ```
///
/// Histogram labels come from the type parameter, not the `label` attribute:
/// ```compile_fail
/// use aetos::{define_histogram, metrics};
///
/// define_histogram!(ResponseTime<()> = [0.1, 0.5, 1.0]);
///
/// #[metrics]
/// struct Metrics {
///     // This will fail - histograms don't support 'label' attribute
///     #[histogram(help = "Response time", label = "endpoint")]
///     response_time: ResponseTime,
/// }
/// ```
#[macro_export]
macro_rules! define_histogram {
    ($name:ident < $label:ty > = $buckets:expr) => {
        const _: () = {
            $crate::core::validate_histogram_buckets(&$buckets);
        };

        #[derive(Clone, Debug)]
        struct $name($crate::core::Histogram<$label, { $buckets.len() }>);

        impl $name {
            pub fn new() -> Self {
                Self($crate::core::Histogram::new($buckets))
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::ops::Deref for $name {
            type Target = $crate::core::Histogram<$label, { $buckets.len() }>;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl $crate::core::HistogramMetric for $name {
            fn render_histogram(
                &self,
                f: &mut std::fmt::Formatter,
                meta: &$crate::core::MetricMetadata,
            ) -> std::fmt::Result {
                self.0.render_histogram(f, meta)
            }
        }
    };
}
