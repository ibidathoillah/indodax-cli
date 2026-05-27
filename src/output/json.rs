use super::CommandOutput;

pub fn render(output: &CommandOutput) -> String {
    let mut envelope = serde_json::json!({
        "success": true,
        "data": output.data,
    });
    if let Some(ref addendum) = output.addendum {
        envelope["addendum"] = serde_json::Value::String(addendum.clone());
    }
    if !output.warnings.is_empty() {
        envelope["warnings"] = serde_json::Value::Array(
            output
                .warnings
                .iter()
                .map(|w| serde_json::Value::String(w.clone()))
                .collect(),
        );
    }
    serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| "{}".into())
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
            warnings: vec![],
            suppress_final_output: false,
        };

        let rendered = render(&output);
        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["data"]["key"], "value");
    }

    #[test]
    fn test_render_nested_object() {
        let output = CommandOutput {
            data: json!({"nested": {"a": 1, "b": 2}}),
            headers: vec![],
            rows: vec![],
            format: super::super::OutputFormat::Json,
            addendum: None,
            warnings: vec![],
            suppress_final_output: false,
        };

        let rendered = render(&output);
        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["data"]["nested"]["a"], 1);
        assert_eq!(parsed["data"]["nested"]["b"], 2);
    }

    #[test]
    fn test_render_array() {
        let output = CommandOutput {
            data: json!([1, 2, 3]),
            headers: vec![],
            rows: vec![],
            format: super::super::OutputFormat::Json,
            addendum: None,
            warnings: vec![],
            suppress_final_output: false,
        };

        let rendered = render(&output);
        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["data"][0], 1);
        assert_eq!(parsed["data"][1], 2);
        assert_eq!(parsed["data"][2], 3);
    }

    #[test]
    fn test_render_empty_object() {
        let output = CommandOutput {
            data: json!({}),
            headers: vec![],
            rows: vec![],
            format: super::super::OutputFormat::Json,
            addendum: None,
            warnings: vec![],
            suppress_final_output: false,
        };

        let rendered = render(&output);
        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["data"], json!({}));
    }

    #[test]
    fn test_render_null() {
        let output = CommandOutput {
            data: json!(null),
            headers: vec![],
            rows: vec![],
            format: super::super::OutputFormat::Json,
            addendum: None,
            warnings: vec![],
            suppress_final_output: false,
        };

        let rendered = render(&output);
        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert!(parsed["data"].is_null());
    }

    #[test]
    fn test_render_with_special_chars() {
        let output = CommandOutput {
            data: json!({"msg": "hello\nworld\t!"}),
            headers: vec![],
            rows: vec![],
            format: super::super::OutputFormat::Json,
            addendum: None,
            warnings: vec![],
            suppress_final_output: false,
        };

        let rendered = render(&output);
        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["data"]["msg"], "hello\nworld\t!");
    }

    #[test]
    fn test_render_with_addendum() {
        let output = CommandOutput {
            data: json!({"key": "value"}),
            headers: vec![],
            rows: vec![],
            format: super::super::OutputFormat::Json,
            addendum: Some("Extra message".into()),
            warnings: vec![],
            suppress_final_output: false,
        };

        let rendered = render(&output);
        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["data"]["key"], "value");
        assert_eq!(parsed["addendum"], "Extra message");
    }
}
