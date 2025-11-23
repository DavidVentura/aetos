use aetos::metrics;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

// Wrapper type that implements Display for AtomicU64
struct AtomicMetric(AtomicU64);

impl fmt::Display for AtomicMetric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.load(Ordering::Relaxed))
    }
}

#[test]
fn test_atomic_u64_wrapper() {
    #[metrics]
    struct TestMetrics {
        #[counter(help = "Total requests")]
        requests: AtomicMetric,
    }

    let m = TestMetrics {
        requests: AtomicMetric(AtomicU64::new(42)),
    };

    let output = m.to_string();

    assert!(output.contains("# HELP requests Total requests\n"));
    assert!(output.contains("# TYPE requests counter\n"));
    assert!(output.contains("requests 42\n"));
}

#[test]
fn test_custom_display_type() {
    use std::fmt;

    struct CustomMetric(u64);

    impl fmt::Display for CustomMetric {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    #[metrics]
    struct TestMetrics {
        #[gauge(help = "Custom value")]
        custom: CustomMetric,
    }

    let m = TestMetrics {
        custom: CustomMetric(123),
    };

    let output = m.to_string();

    assert!(output.contains("# HELP custom Custom value\n"));
    assert!(output.contains("# TYPE custom gauge\n"));
    assert!(output.contains("custom 123\n"));
}
