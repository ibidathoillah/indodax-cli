# Price Alert Monitor Skill

## Goal
Set up and manage price alerts for proactive trading.

## Workflow
1. **Identify Level**: Use `market-brief` to find key support/resistance levels.
2. **Add Alert**: Run `indodax alert add -p <pair> --above <price>` or `--below <price>`.
3. **List Active**: Run `indodax alert list` to confirm all alerts are set correctly.
4. **Watch Mode**: Run `indodax alert watch -p <pair>` to keep a live view of price vs alerts.
5. **Handle Trigger**: When an alert triggers, use `safe-order-preview` to decide on an action.

## Cleanup
Run `indodax alert cancel -i <id>` for alerts that are no longer relevant.
