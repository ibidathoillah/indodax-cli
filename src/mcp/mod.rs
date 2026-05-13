pub mod safety;
pub mod service;
pub mod tools;

use crate::client::IndodaxClient;
use crate::config::IndodaxConfig;
use crate::errors::IndodaxError;
use rmcp::ServiceExt;
use service::ServiceGroup;
use tools::IndodaxMcp;

/// Run the MCP stdio server.
///
/// Reads JSON-RPC messages from stdin and writes responses to stdout.
/// Service groups filter which tools are available.
pub async fn run(
    groups_str: &str,
    allow_dangerous: bool,
    client: IndodaxClient,
    config: IndodaxConfig,
) -> Result<(), IndodaxError> {
    let enabled_groups = ServiceGroup::parse(groups_str)
        .map_err(|e| IndodaxError::Other(format!("Invalid service groups: {}", e)))?;

    let safety = safety::SafetyConfig::new(allow_dangerous);

    let mcp_server = IndodaxMcp::new(client, config, safety, enabled_groups);

    let service = mcp_server
        .serve(rmcp::transport::io::stdio())
        .await
        .map_err(|e| IndodaxError::Other(format!("MCP server error: {}", e)))?;

    tracing::info!(
        "MCP server started with groups: {}, allow_dangerous: {}",
        groups_str,
        allow_dangerous
    );

    // Block forever while the MCP server runs
    service.waiting().await
        .map_err(|e| IndodaxError::Other(format!("MCP server error: {}", e)))?;

    Ok(())
}
