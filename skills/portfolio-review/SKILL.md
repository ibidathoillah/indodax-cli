# Portfolio Review Skill

## Goal
Evaluate the current value, allocation, and performance of the user's holdings.

## Workflow
1. **Get Balances**: Run `indodax balance -o json` to get a list of assets.
2. **Current Value**: For each non-zero asset, fetch the current price using `indodax ticker <asset>_idr`.
3. **Calculate Allocation**: Determine the percentage of each asset relative to total portfolio value in IDR.
4. **Historical Context**: Run `indodax account transactions` to see recent deposits/withdrawals.
5. **Report**: Output a table with "Asset | Amount | Value (IDR) | % Allocation".

## Safety
Do not disclose full API keys or secrets in the summary.
