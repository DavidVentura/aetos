use aetos::{define_histogram, exponential_buckets, linear_buckets};

// Test using linear_buckets directly in the macro
define_histogram!(LinearTest<()> = linear_buckets::<5>(0.1, 0.1));

// Test using exponential_buckets directly in the macro
define_histogram!(ExponentialTest<()> = exponential_buckets::<5>(1.0, 2.0));

// Test that array literals still work
define_histogram!(ArrayLiteralTest<()> = [0.1, 0.5, 1.0]);

#[test]
fn test_linear_buckets_in_macro() {
    let mut hist = LinearTest::new();
    hist.observe((), 0.15);
    hist.observe((), 0.35);

    let data = hist.data.get(&()).unwrap();
    assert_eq!(data.count, 2);
    assert_eq!(data.counts[0], 0);
    assert_eq!(data.counts[1], 1);
    assert_eq!(data.counts[2], 0);
    assert_eq!(data.counts[3], 1);
    assert_eq!(data.counts[4], 0);
}

#[test]
fn test_exponential_buckets_in_macro() {
    let mut hist = ExponentialTest::new();
    hist.observe((), 0.5);
    hist.observe((), 1.5);
    hist.observe((), 3.0);

    let data = hist.data.get(&()).unwrap();
    assert_eq!(data.count, 3);
    assert_eq!(data.counts[0], 1);
    assert_eq!(data.counts[1], 1);
    assert_eq!(data.counts[2], 1);
}

#[test]
fn test_array_literal_still_works() {
    let mut hist = ArrayLiteralTest::new();
    hist.observe((), 0.25);

    let data = hist.data.get(&()).unwrap();
    assert_eq!(data.count, 1);
}
