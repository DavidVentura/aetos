use aetos_core::Label;
use aetos_macro::Label;

#[test]
fn test_single_field_label() {
    #[derive(Label)]
    struct SingleLabel {
        event_type: String,
    }

    let label = SingleLabel {
        event_type: "stake".to_string(),
    };

    let result = format!("{}", MockFormatter(&label));
    assert_eq!(result, r#"event_type="stake""#);
}

#[test]
fn test_multi_field_label() {
    #[derive(Label)]
    struct MultiLabel {
        method: String,
        status: u32,
    }

    let label = MultiLabel {
        method: "GET".to_string(),
        status: 200,
    };

    let result = format!("{}", MockFormatter(&label));
    assert_eq!(result, r#"method="GET",status="200""#);
}

#[test]
fn test_label_escaping() {
    #[derive(Label)]
    struct EscapeLabel {
        value: String,
    }

    let label = EscapeLabel {
        value: r#"has"quotes"and\backslash"#.to_string(),
    };

    let result = format!("{}", MockFormatter(&label));
    assert_eq!(result, r#"value="has\"quotes\"and\\backslash""#);
}

#[test]
fn test_label_newline_escaping() {
    #[derive(Label)]
    struct NewlineLabel {
        text: String,
    }

    let label = NewlineLabel {
        text: "line1\nline2".to_string(),
    };

    let result = format!("{}", MockFormatter(&label));
    assert_eq!(result, r#"text="line1\nline2""#);
}

#[test]
fn test_three_fields_no_trailing_comma() {
    #[derive(Label)]
    struct ThreeFields {
        a: String,
        b: String,
        c: String,
    }

    let label = ThreeFields {
        a: "1".to_string(),
        b: "2".to_string(),
        c: "3".to_string(),
    };

    let result = format!("{}", MockFormatter(&label));
    assert_eq!(result, r#"a="1",b="2",c="3""#);
    assert!(!result.ends_with(','));
}

struct MockFormatter<'a, T: Label>(&'a T);

impl<'a, T: Label> std::fmt::Display for MockFormatter<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt_labels(f)
    }
}
