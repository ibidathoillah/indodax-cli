# Paper Trading Session Skill

## Goal
Manage a risk-free trading simulation session.

## Workflow
1. **Initialize**: Run `indodax paper init` to set up starting capital.
2. **Set Alerts**: Run `indodax alert add` to monitor entry points.
3. **Execute Trades**: Use `indodax paper buy` or `indodax paper sell`.
4. **Monitor Fills**: Run `indodax paper check-fills` periodically to simulate market matching.
5. **Review Status**: Use `indodax paper status` and `indodax paper history` to track performance.

## Best Practice
Use paper trading for at least 24 hours before moving a strategy to the live exchange.
