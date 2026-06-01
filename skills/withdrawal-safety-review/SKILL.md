# Withdrawal Safety Review Skill

## Goal
Ensure withdrawal requests are accurate and secure.

## Workflow
1. **Check Fee**: Run `indodax funding withdraw-fee <asset>` to know the cost.
2. **Verify Destination**: Double-check the address and network (`indodax funding deposit-address` for self-transfers).
3. **Check Balance**: Ensure enough funds exist including the fee.
4. **Execute**: Run `indodax withdraw --asset <asset> --volume <amount> --address <address> --network <network>`.
5. **Monitor Callback**: If using a callback server, run `indodax funding withdraw-callback` to confirm the transaction.

## Safety
Never share withdrawal addresses or memos in public logs.
