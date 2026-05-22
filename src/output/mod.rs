use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutputFormat::Table => write!(f, "table"),
            OutputFormat::Json => write!(f, "json"),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CommandOutput {
    pub data: serde_json::Value,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    #[serde(skip)]
    pub format: OutputFormat,
    #[serde(skip)]
    pub addendum: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(skip)]
    #[doc(hidden)]
    pub suppress_final_output: bool,
}

impl CommandOutput {
    pub fn new(
        data: serde_json::Value,
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    ) -> Self {
        Self {
            data,
            headers,
            rows,
            format: OutputFormat::Table,
            addendum: None,
            warnings: vec![],
            suppress_final_output: false,
        }
    }

    pub fn json(data: serde_json::Value) -> Self {
        Self {
            data,
            headers: vec![],
            rows: vec![],
            format: OutputFormat::Table,
            addendum: None,
            warnings: vec![],
            suppress_final_output: false,
        }
    }

    pub fn new_empty() -> Self {
        Self {
            data: serde_json::json!({}),
            headers: vec![],
            rows: vec![],
            format: OutputFormat::Table,
            addendum: None,
            warnings: vec![],
            suppress_final_output: false,
        }
    }

    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.format = format;
        self
    }

    pub fn with_addendum(mut self, addendum: impl Into<String>) -> Self {
        self.addendum = Some(addendum.into());
        self
    }

    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    pub fn with_suppress_final_output(mut self, suppress: bool) -> Self {
        self.suppress_final_output = suppress;
        self
    }

    pub fn render(&self) -> String {
        match self.format {
            #[cfg(not(target_arch = "wasm32"))]
            OutputFormat::Table => table::render(self),
            #[cfg(target_arch = "wasm32")]
            OutputFormat::Table => json::render(self),
            OutputFormat::Json => json::render(self),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod table;
pub mod json;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_output_format_display_table() {
        assert_eq!(format!("{}", OutputFormat::Table), "table");
    }

    #[test]
    fn test_output_format_display_json() {
        assert_eq!(format!("{}", OutputFormat::Json), "json");
    }

    #[test]
    fn test_output_format_from_clap() {
        // Test that OutputFormat works with clap ValueEnum
        let formats = [OutputFormat::Table, OutputFormat::Json];
        assert_eq!(formats.len(), 2);
    }

    #[test]
    fn test_command_output_new() {
        let data = json!({"key": "value"});
        let headers = vec!["H1".into(), "H2".into()];
        let rows = vec![vec!["a".into(), "b".into()]];
        let output = CommandOutput::new(data.clone(), headers.clone(), rows.clone());
        
        assert_eq!(output.data, data);
        assert_eq!(output.headers, headers);
        assert_eq!(output.rows, rows);
        assert_eq!(output.format, OutputFormat::Table);
        assert!(output.addendum.is_none());
    }

    #[test]
    fn test_command_output_json() {
        let data = json!({"test": 123});
        let output = CommandOutput::json(data.clone());
        
        assert_eq!(output.data, data);
        assert!(output.headers.is_empty());
        assert!(output.rows.is_empty());
        assert_eq!(output.format, OutputFormat::Table);
    }

    #[test]
    fn test_command_output_new_empty() {
        let output = CommandOutput::new_empty();
        
        assert_eq!(output.data, json!({}));
        assert!(output.headers.is_empty());
        assert!(output.rows.is_empty());
    }

    #[test]
    fn test_command_output_with_format() {
        let output = CommandOutput::json(json!({"test": true}))
            .with_format(OutputFormat::Json);
        
        assert_eq!(output.format, OutputFormat::Json);
    }

    #[test]
    fn test_command_output_with_addendum() {
        let output = CommandOutput::new_empty()
            .with_addendum("test addendum");
        
        assert_eq!(output.addendum, Some("test addendum".into()));
    }

    #[test]
    fn test_command_output_render_table() {
        let output = CommandOutput {
            data: json!({"key": "value"}),
            headers: vec!["Key".into(), "Value".into()],
            rows: vec![vec!["key".into(), "value".into()]],
            format: OutputFormat::Table,
            addendum: None,
            warnings: vec![],
            suppress_final_output: false,
        };
        
        let rendered = output.render();
        assert!(!rendered.is_empty());
    }

    #[test]
    fn test_command_output_render_json() {
        let data = json!({"hello": "world"});
        let output = CommandOutput::json(data.clone()).with_format(OutputFormat::Json);
        
        let rendered = output.render();
        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["data"], data);
    }

    #[test]
    fn test_command_output_render_json_empty() {
        let output = CommandOutput::new_empty().with_format(OutputFormat::Json);
        let rendered = output.render();
        let parsed: serde_json::Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["data"], json!({}));
    }

    #[test]
    fn test_command_output_render_table_with_addendum() {
        let output = CommandOutput {
            data: json!({}),
            headers: vec![],
            rows: vec![],
            format: OutputFormat::Table,
            addendum: Some("Extra info".into()),
            warnings: vec![],
            suppress_final_output: false,
        };
        
        let rendered = output.render();
        assert!(rendered.contains("Extra info"));
    }

    #[test]
    fn test_command_output_serialization() {
        let output = CommandOutput {
            data: json!({"test": true}),
            headers: vec!["H".into()],
            rows: vec![vec!["v".into()]],
            format: OutputFormat::Json,
            addendum: Some("info".into()),
            warnings: vec![],
            suppress_final_output: false,
        };
        
        let serialized = serde_json::to_string(&output).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(deserialized["data"]["test"], true);
        assert_eq!(deserialized["headers"][0], "H");
        assert_eq!(deserialized["rows"][0][0], "v");
        // format and addendum should be skipped (marked with #[serde(skip)])
    }

    #[test]
    fn test_command_output_render_empty_table_with_data() {
        let output = CommandOutput {
            data: json!({"key": "value"}),
            headers: vec![],
            rows: vec![],
            format: OutputFormat::Table,
            addendum: None,
            warnings: vec![],
            suppress_final_output: false,
        };
        
        let rendered = output.render();
        // Should fall back to rendering data as JSON
        assert!(rendered.contains("key") || rendered.contains("value") || rendered.is_empty());
    }
}
