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
        table.add_row(row.iter().map(|c| Cell::new(c)).collect::<Vec<_>>());
    }

    let mut result = table.to_string();

    if let Some(ref addendum) = output.addendum {
        result.push('\n');
        result.push_str(addendum);
    }

    result
}
