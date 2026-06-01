# Market Brief Skill

## Goal
Provide a concise summary of current market conditions for one or more pairs.

## Workflow
1. **List Pairs**: Run `indodax pairs` to confirm available assets if unsure.
2. **Fetch Tickers**: Run `indodax ticker <pair>` for each target.
3. **Analyze Trends**: Check `indodax history <pair> --timeframe 60` (1h) or `1440` (24h) for recent price action.
4. **Order Book Depth**: Run `indodax orderbook <pair> --count 5` to see immediate liquidity.
5. **Summarize**: Combine data into a "Price | 24h Change | Spread | Sentiment" format.

## Example
"BTC/IDR is at 950M, up 2% in 24h. Order book shows strong support at 940M."
