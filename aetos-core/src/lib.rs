use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};
use std::hash::Hash;

pub trait Label {
    fn fmt_labels(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

impl Label for () {
    fn fmt_labels(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

pub const fn linear_buckets<const N: usize>(start: f64, width: f64) -> [f64; N] {
    let mut buckets = [0.0; N];
    let mut i = 0;

    while i < N {
        buckets[i] = start + width * (i as f64);
        i += 1;
    }
    buckets
}

pub const fn exponential_buckets<const N: usize>(start: f64, factor: f64) -> [f64; N] {
    let mut buckets = [0.0; N];
    let mut current = start;
    let mut i = 0;
    while i < N {
        buckets[i] = current;
        current *= factor;
        i += 1;
    }
    buckets
}

pub trait PrometheusMetric: std::fmt::Display {}

#[derive(Clone, Debug)]
pub struct HistogramData<const N: usize> {
    pub counts: [u64; N],
    pub count: u64,
    pub sum: f64,
}

impl<const N: usize> Default for HistogramData<N> {
    fn default() -> Self {
        Self {
            counts: [0; N],
            count: 0,
            sum: 0.0,
        }
    }
}

/// Validates that histogram buckets are in strictly ascending order.
///
/// This is a const function that panics at compile time if buckets are not ordered correctly.
/// It is automatically called by the `define_histogram!` macro.
///
/// # Examples
///
/// Valid buckets compile successfully:
/// ```
/// use aetos_core::validate_histogram_buckets;
///
/// const _: () = {
///     const BUCKETS: [f64; 3] = [0.1, 0.5, 1.0];
///     validate_histogram_buckets(&BUCKETS);
/// };
/// ```
///
/// Invalid buckets cause a compile-time error:
/// ```compile_fail
/// use aetos_core::validate_histogram_buckets;
///
/// const _: () = {
///     const BUCKETS: [f64; 3] = [1.0, 0.5, 2.0];  // Out of order!
///     validate_histogram_buckets(&BUCKETS);
/// };
/// ```
pub const fn validate_histogram_buckets(buckets: &[f64]) {
    let mut i = 1;
    while i < buckets.len() {
        if buckets[i - 1] >= buckets[i] {
            panic!("Histogram buckets must be in strictly ascending order");
        }
        i += 1;
    }
}

#[derive(Clone, Debug)]
pub struct Histogram<L, const N: usize> {
    buckets: [f64; N],
    pub data: HashMap<L, HistogramData<N>>,
}

impl<L: Hash + Eq, const N: usize> Histogram<L, N> {
    pub fn new(buckets: [f64; N]) -> Self {
        Self {
            buckets,
            data: HashMap::new(),
        }
    }

    pub fn observe(&mut self, label: L, value: f64) {
        let entry = self.data.entry(label).or_default();

        entry.sum += value;
        entry.count += 1;

        for i in 0..N {
            if value <= self.buckets[i] {
                entry.counts[i] += 1;
                break;
            }
        }
    }
}

#[cfg(not(feature = "no-escaping"))]
pub fn escape_label_value(s: &str) -> Cow<'_, str> {
    // Fast path: check if escaping is needed
    if !s.chars().any(|ch| matches!(ch, '"' | '\\' | '\n')) {
        return Cow::Borrowed(s);
    }

    // Slow path: escape special characters
    let mut result = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            _ => result.push(ch),
        }
    }
    Cow::Owned(result)
}

#[cfg(feature = "no-escaping")]
#[inline(always)]
pub fn escape_label_value(s: &str) -> Cow<'_, str> {
    Cow::Borrowed(s)
}

/// This abstracts away the difference between:
/// - Vec/Slice iterator items: &(K, V)
/// - HashMap iterator items:   (&K, &V)
pub trait BorrowPair {
    type Key: ?Sized;
    type Value: Display + ?Sized;

    fn borrow_pair(&self) -> (&Self::Key, &Self::Value);
}

// Implementation for references to tuples (Vec, Slices, Arrays)
// Iterating &[(K,V)] yields &(K,V)
impl<K, V: Display> BorrowPair for &(K, V) {
    type Key = K;
    type Value = V;

    fn borrow_pair(&self) -> (&K, &V) {
        (&self.0, &self.1)
    }
}

// Implementation for tuples of references (HashMaps and BTreeMaps)
// Iterating &HashMap<K,V> yields (&K, &V)
impl<'a, K, V: Display> BorrowPair for (&'a K, &'a V) {
    type Key = K;
    type Value = V;

    fn borrow_pair(&self) -> (&K, &V) {
        (self.0, self.1)
    }
}

pub trait HistogramMetric {
    fn render_histogram(&self, f: &mut Formatter, meta: &MetricMetadata) -> fmt::Result;
}

impl<L, const N: usize> HistogramMetric for Histogram<L, N>
where
    L: Label + Hash + Eq,
{
    fn render_histogram(&self, f: &mut Formatter, meta: &MetricMetadata) -> fmt::Result {
        write!(
            f,
            "# HELP {} {}\n# TYPE {} histogram\n",
            meta.name, meta.help, meta.name
        )?;

        for (label, data) in &self.data {
            let mut cumulative_count = 0;
            for i in 0..N {
                cumulative_count += data.counts[i];
                let upper_bound = self.buckets[i];

                write!(f, "{}_bucket{{", meta.name)?;
                label.fmt_labels(f)?;
                if std::mem::size_of::<L>() > 0 {
                    write!(f, ",")?;
                }
                writeln!(f, "le=\"{}\"}} {}", upper_bound, cumulative_count)?;
            }

            write!(f, "{}_bucket{{", meta.name)?;
            label.fmt_labels(f)?;
            if std::mem::size_of::<L>() > 0 {
                write!(f, ",")?;
            }
            writeln!(f, "le=\"+Inf\"}} {}", data.count)?;

            write!(f, "{}_sum{{", meta.name)?;
            label.fmt_labels(f)?;
            writeln!(f, "}} {}", data.sum)?;

            write!(f, "{}_count{{", meta.name)?;
            label.fmt_labels(f)?;
            writeln!(f, "}} {}", data.count)?;
        }
        Ok(())
    }
}

pub struct MetricMetadata<'a> {
    pub name: &'a str,
    pub help: &'a str,
    pub kind: &'a str,
}

pub struct MetricWrapper<'a, T: ?Sized>(pub &'a T);

pub trait RenderScalarFallback {
    fn render_with_label_attr(
        &self,
        f: &mut Formatter,
        meta: &MetricMetadata,
        _label: &str,
    ) -> fmt::Result;

    fn render_with_struct_key(&self, f: &mut Formatter, meta: &MetricMetadata) -> fmt::Result;

    fn render_histogram(&self, f: &mut Formatter, meta: &MetricMetadata) -> fmt::Result;
}

// Blanket impl for anything Display (u64, f64, AtomicU64, etc.)
impl<'a, T: Display + ?Sized> RenderScalarFallback for MetricWrapper<'a, T> {
    fn render_with_label_attr(
        &self,
        f: &mut Formatter,
        meta: &MetricMetadata,
        _label: &str,
    ) -> fmt::Result {
        self.render_scalar(f, meta)
    }

    fn render_with_struct_key(&self, f: &mut Formatter, meta: &MetricMetadata) -> fmt::Result {
        self.render_scalar(f, meta)
    }

    fn render_histogram(&self, _f: &mut Formatter, _meta: &MetricMetadata) -> fmt::Result {
        Ok(())
    }
}

// Private helper for scalar rendering
impl<'a, T: Display + ?Sized> MetricWrapper<'a, T> {
    fn render_scalar(&self, f: &mut Formatter, meta: &MetricMetadata) -> fmt::Result {
        writeln!(f, "# HELP {} {}", meta.name, meta.help)?;
        writeln!(f, "# TYPE {} {}", meta.name, meta.kind)?;
        writeln!(f, "{} {}", meta.name, self.0)
    }
}

// Inherent method for histogram types.
// Because this is inherent, Rust picks it BEFORE looking at RenderScalarFallback.
impl<'a, T: ?Sized> MetricWrapper<'a, T>
where
    T: HistogramMetric,
{
    pub fn render_histogram(&self, f: &mut Formatter, meta: &MetricMetadata) -> fmt::Result {
        self.0.render_histogram(f, meta)
    }
}

// These methods exist *only* if T is iterable with BorrowPair items.
// Because they are inherent, Rust picks them BEFORE looking at RenderScalarFallback.
impl<'a, T: ?Sized> MetricWrapper<'a, T>
where
    &'a T: IntoIterator,
    <&'a T as IntoIterator>::Item: BorrowPair,
{
    // Only exists when Key implements Display
    pub fn render_with_label_attr(
        &self,
        f: &mut Formatter,
        meta: &MetricMetadata,
        label_name: &str,
    ) -> fmt::Result
    where
        <<&'a T as IntoIterator>::Item as BorrowPair>::Key: Display,
    {
        writeln!(f, "# HELP {} {}", meta.name, meta.help)?;
        writeln!(f, "# TYPE {} {}", meta.name, meta.kind)?;

        for item in self.0 {
            let (k, v) = item.borrow_pair();
            writeln!(
                f,
                "{}{{{}=\"{}\"}} {}",
                meta.name,
                label_name,
                escape_label_value(&k.to_string()),
                v
            )?;
        }
        Ok(())
    }

    // Only exists when Key implements Label
    pub fn render_with_struct_key(&self, f: &mut Formatter, meta: &MetricMetadata) -> fmt::Result
    where
        <<&'a T as IntoIterator>::Item as BorrowPair>::Key: Label,
    {
        writeln!(f, "# HELP {} {}", meta.name, meta.help)?;
        writeln!(f, "# TYPE {} {}", meta.name, meta.kind)?;

        for item in self.0 {
            let (k, v) = item.borrow_pair();
            write!(f, "{}{{", meta.name)?;
            k.fmt_labels(f)?;
            writeln!(f, "}} {}", v)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn within_epsilon(a: f64, b: f64) -> bool {
        (a - b).abs() < f64::EPSILON
    }

    #[test]
    #[cfg(not(feature = "no-escaping"))]
    fn test_escape_label_value() {
        assert_eq!(escape_label_value("simple"), "simple");
        assert_eq!(escape_label_value("with\"quote"), "with\\\"quote");
        assert_eq!(escape_label_value("with\\backslash"), "with\\\\backslash");
        assert_eq!(escape_label_value("with\nnewline"), "with\\nnewline");
        assert_eq!(
            escape_label_value("all\"three\\\nchars"),
            "all\\\"three\\\\\\nchars"
        );
    }

    #[test]
    #[cfg(feature = "no-escaping")]
    fn test_escape_label_value_no_escaping() {
        assert_eq!(escape_label_value("simple"), "simple");
        assert_eq!(escape_label_value("with\"quote"), "with\"quote");
        assert_eq!(escape_label_value("with\\backslash"), "with\\backslash");
        assert_eq!(escape_label_value("with\nnewline"), "with\nnewline");
        assert_eq!(
            escape_label_value("all\"three\\\nchars"),
            "all\"three\\\nchars"
        );
    }

    #[test]
    fn test_linear_buckets_basic() {
        const BUCKETS: [f64; 5] = linear_buckets(0.1, 0.1);
        assert_eq!(BUCKETS.len(), 5);
        assert!(within_epsilon(BUCKETS[0], 0.1));
        assert!(within_epsilon(BUCKETS[1], 0.2));
        assert!(within_epsilon(BUCKETS[2], 0.3));
        assert!(within_epsilon(BUCKETS[3], 0.4));
        assert!(within_epsilon(BUCKETS[4], 0.5));
    }

    #[test]
    fn test_linear_buckets_single() {
        const BUCKETS: [f64; 1] = linear_buckets(5.0, 1.0);
        assert_eq!(BUCKETS.len(), 1);
        assert_eq!(BUCKETS[0], 5.0);
    }

    #[test]
    fn test_linear_buckets_zero_start() {
        const BUCKETS: [f64; 3] = linear_buckets(0.0, 0.5);
        assert_eq!(BUCKETS, [0.0, 0.5, 1.0]);
    }

    #[test]
    fn test_linear_buckets_with_validation() {
        const BUCKETS: [f64; 5] = linear_buckets(1.0, 1.0);
        const _: () = validate_histogram_buckets(&BUCKETS);
    }

    #[test]
    fn test_exponential_buckets_basic() {
        const BUCKETS: [f64; 5] = exponential_buckets(1.0, 2.0);
        assert_eq!(BUCKETS.len(), 5);
        assert_eq!(BUCKETS[0], 1.0);
        assert_eq!(BUCKETS[1], 2.0);
        assert_eq!(BUCKETS[2], 4.0);
        assert_eq!(BUCKETS[3], 8.0);
        assert_eq!(BUCKETS[4], 16.0);
    }

    #[test]
    fn test_exponential_buckets_single() {
        const BUCKETS: [f64; 1] = exponential_buckets(100.0, 10.0);
        assert_eq!(BUCKETS.len(), 1);
        assert_eq!(BUCKETS[0], 100.0);
    }

    #[test]
    fn test_exponential_buckets_small_factor() {
        const BUCKETS: [f64; 4] = exponential_buckets(1.0, 1.5);
        assert_eq!(BUCKETS.len(), 4);
        assert_eq!(BUCKETS[0], 1.0);
        assert_eq!(BUCKETS[1], 1.5);
        assert_eq!(BUCKETS[2], 2.25);
        assert_eq!(BUCKETS[3], 3.375);
    }

    #[test]
    fn test_exponential_buckets_large_factor() {
        const BUCKETS: [f64; 3] = exponential_buckets(1.0, 10.0);
        assert_eq!(BUCKETS, [1.0, 10.0, 100.0]);
    }

    #[test]
    fn test_exponential_buckets_with_validation() {
        const BUCKETS: [f64; 8] = exponential_buckets(100.0, 2.0);
        const _: () = validate_histogram_buckets(&BUCKETS);
    }

    #[test]
    fn test_exponential_buckets_growth_rate() {
        const BUCKETS: [f64; 5] = exponential_buckets(2.0, 3.0);
        for i in 1..5 {
            let ratio = BUCKETS[i] / BUCKETS[i - 1];
            assert!(
                within_epsilon(ratio, 3.0),
                "Growth factor should be 3.0, got {}",
                ratio
            );
        }
    }

    #[test]
    fn test_linear_buckets_integration() {
        let mut hist = Histogram::new(linear_buckets::<5>(0.1, 0.1));

        hist.observe((), 0.15);
        hist.observe((), 0.25);
        hist.observe((), 0.45);

        let data = hist.data.get(&()).unwrap();
        assert_eq!(data.count, 3);
        assert!(within_epsilon(data.sum, 0.85));
        assert_eq!(data.counts[0], 0);
        assert_eq!(data.counts[1], 1);
        assert_eq!(data.counts[2], 1);
        assert_eq!(data.counts[3], 0);
        assert_eq!(data.counts[4], 1);
    }

    #[test]
    fn test_exponential_buckets_integration() {
        let mut hist = Histogram::new(exponential_buckets::<5>(1.0, 2.0));

        hist.observe((), 0.5);
        hist.observe((), 1.5);
        hist.observe((), 3.0);
        hist.observe((), 7.0);

        let data = hist.data.get(&()).unwrap();
        assert_eq!(data.count, 4);
        assert_eq!(data.sum, 12.0);
        assert_eq!(data.counts[0], 1);
        assert_eq!(data.counts[1], 1);
        assert_eq!(data.counts[2], 1);
        assert_eq!(data.counts[3], 1);
        assert_eq!(data.counts[4], 0);
    }
}
