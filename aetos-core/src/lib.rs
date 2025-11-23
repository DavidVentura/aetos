use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};

pub trait Label {
    fn fmt_labels(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

pub trait PrometheusMetric: std::fmt::Display {}

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
}

// Private helper for scalar rendering
impl<'a, T: Display + ?Sized> MetricWrapper<'a, T> {
    fn render_scalar(&self, f: &mut Formatter, meta: &MetricMetadata) -> fmt::Result {
        writeln!(f, "# HELP {} {}", meta.name, meta.help)?;
        writeln!(f, "# TYPE {} {}", meta.name, meta.kind)?;
        writeln!(f, "{} {}", meta.name, self.0)
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
}
