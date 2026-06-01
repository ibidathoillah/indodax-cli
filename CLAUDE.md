# Instructions for Claude / IDE Extensions

This project is optimized for AI-assisted development and usage. When interacting with this repository, please adhere to the following guidelines:

## 🧩 Style & Conventions
- **Language**: Rust (edition 2021).
- **Async**: Uses `tokio` and `reqwest`.
- **Formatting**: Adhere to `cargo fmt`.
- **Error Handling**: Use `anyhow::Result` for application logic and `thiserror` for defined error types.

## 🤖 Tool Interaction
- When using the CLI via terminal, always prefer the `-o json` flag for structured output.
- For dangerous operations (e.g., placing orders), confirm the parameters and suggest using `--dry-run` or paper trading first.

## 📁 Repository Structure
- `src/main.rs`: Binary entry point and MCP server wrapper.
- `src/lib.rs`: Library core and CLI command definitions.
- `src/commands/`: Implementation of specific command groups.
- `src/mcp/`: MCP server implementation and tool definitions.
- `src/client.rs`: Core API client logic.
- `tests/`: Integration and E2E tests.

## 🧪 Testing
- Always run `cargo test` before submitting changes.
- Use `serial_test` for tests that might conflict (e.g., those touching the same config files).
- Add new test cases to `tests/` for any new features.

## 📜 Documentation
- Update `README.md` and `RELEASE_NOTES.md` for user-facing changes.
- Maintain `AGENTS.md` and `CONTEXT.md` for AI context.
- Keep `TODO.md` updated with progress.
