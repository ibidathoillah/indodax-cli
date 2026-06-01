# MCP Setup Skill

## Goal
Configure and run the Model Context Protocol (MCP) server for AI integration.

## Workflow
1. **Choose Profile**: Decide on the access level (`readonly`, `paper`, `full`).
2. **Launch Server**: Run `indodax mcp --groups <groups>`.
3. **HTTP Bridge (Optional)**: If remote access is needed, run `indodax mcp --http --port 8000`.
4. **Security Check**: Ensure `allow_dangerous` is only set if human confirmation is guaranteed at the agent layer.
5. **Verify**: Use an MCP inspector or client to list available tools.

## Recommended Groups
- `market,account`: Safe for most use cases.
- `trade,funding`: High risk, requires careful guarding.
