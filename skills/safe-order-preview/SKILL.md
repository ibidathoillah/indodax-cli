# Safe Order Preview Skill

## Goal
Verify trade parameters before execution to prevent errors.

## Workflow
1. **Validate Pair**: Confirm the pair exists with `indodax pairs`.
2. **Check Price**: Get the current market price using `indodax ticker <pair>`.
3. **Check Balance**: Run `indodax balance` to ensure sufficient funds.
4. **Simulate (Dry Run)**: If the command supports it, use `--dry-run`. Otherwise, manually calculate the expected cost/outcome.
5. **Human Confirmation**: Present the final details (Side, Pair, Amount, Price, Total) and ask for explicit confirmation.

## Dangerous Annotations
If `acknowledged: true` is required for MCP, explain exactly what it means to the user before setting it.
