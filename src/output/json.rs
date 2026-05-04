use super::CommandOutput;

pub fn render(output: &CommandOutput) -> String {
    serde_json::to_string_pretty(&output.data).unwrap_or_else(|_| "{}".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_render_simple_object() {
        let output = CommandOutput {
            data: json!({"key": "value"}),
            headers: vec![],
            rows: vec![],
            format: super::super::OutputFormat::Json,
            addendum: None,
        };
        
        let rendered = render(&output);
        assert!(rendered.contains("key"));
        assert!(rendered.contains("value"));
    }

    #[test]
    fn test_render_nested_object() {
        let output = CommandOutput {
            data: json!({"nested": {"a": 1, "b": 2}}),
            headers: vec![],
            rows: vec![],
            format: super::super::OutputFormat::Json,
            addendum: None,
        };
        
        let rendered = render(&output);
        assert!(rendered.contains("nested"));
        assert!(rendered.contains("a"));
        assert!(rendered.contains("b"));
    }

    #[test]
    fn test_render_array() {
        let output = CommandOutput {
            data: json!([1, 2, 3]),
            headers: vec![],
            rows: vec![],
            format: super::super::OutputFormat::Json,
            addendum: None,
        };
        
        let rendered = render(&output);
        assert!(rendered.contains("1"));
        assert!(rendered.contains("2"));
        assert!(rendered.contains("3"));
    }

    #[test]
    fn test_render_empty_object() {
        let output = CommandOutput {
            data: json!({}),
            headers: vec![],
            rows: vec![],
            format: super::super::OutputFormat::Json,
            addendum: None,
        };
        
        let rendered = render(&output);
        assert_eq!(rendered.trim(), "{}");
    }

    #[test]
    fn test_render_null() {
        let output = CommandOutput {
            data: json!(null),
            headers: vec![],
            rows: vec![],
            format: super::super::OutputFormat::Json,
            addendum: None,
        };
        
        let rendered = render(&output);
        assert!(rendered.contains("null"));
    }

    #[test]
    fn test_render_with_special_chars() {
        let output = CommandOutput {
            data: json!({"message": "hello\nworld\t!"}),
            headers: vec![],
            rows: vec![],
            format: super::super::OutputFormat::Json,
            addendum: None,
        };
        
        let rendered = render(&output);
        assert!(rendered.contains("hello"));
        assert!(rendered.contains("world"));
    }
}
