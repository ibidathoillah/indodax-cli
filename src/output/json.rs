use super::CommandOutput;

pub fn render(output: &CommandOutput) -> String {
    serde_json::to_string_pretty(&output.data).unwrap_or_else(|_| "{}".into())
}
