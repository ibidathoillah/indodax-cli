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
        }
    }

    pub fn json(data: serde_json::Value) -> Self {
        Self {
            data,
            headers: vec![],
            rows: vec![],
            format: OutputFormat::Table,
            addendum: None,
        }
    }

    pub fn new_empty() -> Self {
        Self {
            data: serde_json::json!({}),
            headers: vec![],
            rows: vec![],
            format: OutputFormat::Table,
            addendum: None,
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

    pub fn render(&self) -> String {
        match self.format {
            OutputFormat::Table => table::render(self),
            OutputFormat::Json => json::render(self),
        }
    }
}

pub mod table;
pub mod json;
