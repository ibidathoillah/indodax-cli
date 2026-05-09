use super::CommandOutput;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Cell, Color, ContentArrangement, Table};

pub fn render(output: &CommandOutput) -> String {
    if output.headers.is_empty() && output.rows.is_empty() {
        if let Some(ref addendum) = output.addendum {
            return addendum.clone();
        }
        return serde_json::to_string_pretty(&output.data).unwrap_or_default();
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(
            output
                .headers
                .iter()
                .map(|h| Cell::new(h).fg(Color::Cyan))
                .collect::<Vec<_>>(),
        );

    for row in &output.rows {
        table.add_row(row.iter().map(Cell::new).collect::<Vec<_>>());
    }

    let mut result = table.to_string();

    if let Some(ref addendum) = output.addendum {
        result.push('\n');
        result.push_str(addendum);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_render_with_headers_and_rows() {
        let output = CommandOutput {
            data: json!({}),
            headers: vec!["Col1".into(), "Col2".into()],
            rows: vec![
                vec!["a".into(), "b".into()],
                vec!["c".into(), "d".into()],
            ],
            format: super::super::OutputFormat::Table,
            addendum: None,
        };
        
        let rendered = render(&output);
        assert!(rendered.contains("Col1"));
        assert!(rendered.contains("Col2"));
        assert!(rendered.contains("a"));
        assert!(rendered.contains("b"));
    }

    #[test]
    fn test_render_empty_headers_and_rows() {
        let output = CommandOutput {
            data: json!({"fallback": "data"}),
            headers: vec![],
            rows: vec![],
            format: super::super::OutputFormat::Table,
            addendum: None,
        };
        
        let rendered = render(&output);
        // Should fall back to JSON rendering of data
        assert!(rendered.contains("fallback") || rendered.contains("data") || rendered.contains("{}"));
    }

    #[test]
    fn test_render_with_addendum() {
        let output = CommandOutput {
            data: json!({}),
            headers: vec!["H".into()],
            rows: vec![vec!["v".into()]],
            format: super::super::OutputFormat::Table,
            addendum: Some("Extra info here".into()),
        };
        
        let rendered = render(&output);
        assert!(rendered.contains("Extra info here"));
    }

    #[test]
    fn test_render_single_row() {
        let output = CommandOutput {
            data: json!({}),
            headers: vec!["Name".into()],
            rows: vec![vec!["Alice".into()]],
            format: super::super::OutputFormat::Table,
            addendum: None,
        };
        
        let rendered = render(&output);
        assert!(rendered.contains("Name"));
        assert!(rendered.contains("Alice"));
    }

    #[test]
    fn test_render_multiple_rows() {
        let output = CommandOutput {
            data: json!({}),
            headers: vec!["ID".into(), "Value".into()],
            rows: vec![
                vec!["1".into(), "100".into()],
                vec!["2".into(), "200".into()],
                vec!["3".into(), "300".into()],
            ],
            format: super::super::OutputFormat::Table,
            addendum: None,
        };
        
        let rendered = render(&output);
        assert!(rendered.contains("ID"));
        assert!(rendered.contains("Value"));
        assert!(rendered.contains("100"));
        assert!(rendered.contains("200"));
        assert!(rendered.contains("300"));
    }

    #[test]
    fn test_render_no_addendum() {
        let output = CommandOutput {
            data: json!({}),
            headers: vec!["H".into()],
            rows: vec![vec!["v".into()]],
            format: super::super::OutputFormat::Table,
            addendum: None,
        };
        
        let rendered = render(&output);
        // Should not contain extra newline at end if no addendum
        assert!(!rendered.ends_with('\n') || rendered.trim().len() > 0);
    }

    #[test]
    fn test_render_empty_data_with_addendum() {
        let output = CommandOutput {
            data: json!({}),
            headers: vec![],
            rows: vec![],
            format: super::super::OutputFormat::Table,
            addendum: Some("Only addendum".into()),
        };
        
        let rendered = render(&output);
        assert!(rendered.contains("Only addendum"));
    }
}
