# Contributing to Indodax CLI

We welcome contributions from the community! Whether you're fixing a bug, adding a feature, or improving documentation, your help is appreciated.

## 🚀 Getting Started
1. **Fork the repo** and create your branch from `main`.
2. **Install dependencies**: Ensure you have Rust and Cargo installed.
3. **Run tests**: `cargo test` to verify the current state.

## 🛠️ Development Guidelines
- **Rust Edition**: 2021.
- **Formatting**: Run `cargo fmt` before committing.
- **Linting**: Run `cargo clippy` to check for common issues.
- **Error Handling**: Follow the patterns in `src/errors.rs`. Use `anyhow` for top-level results and `thiserror` for library errors.

## 🤖 AI-Native Contributions
We highly value contributions that improve the experience for AI agents:
- **Skills**: Add new workflow patterns to `skills/*/SKILL.md`.
- **Catalogs**: Update `agents/tool-catalog.json` or `agents/error-catalog.json`.
- **Docs**: Improve `AGENTS.md` or `CONTEXT.md`.

## 🧪 Testing Requirements
- Any new feature should include integration tests in `tests/` and unit tests in the relevant module.
- For trading/funding logic, ensure there is a paper trading equivalent.

## 📜 Pull Request Process
1. Describe your changes clearly in the PR description.
2. Link any related issues.
3. Ensure all CI checks pass (GitHub Actions).

## ⚖️ License
By contributing, you agree that your contributions will be licensed under the MIT License.
