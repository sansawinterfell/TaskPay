# TaskPay

> On-chain task management with milestone-based USDC escrow on Stellar

---

## Problem

A freelance project manager in Manila overseeing a remote team across SEA has no trustless way to release pay. Contractors distrust manual bank wire delays (2–5 days, $15–30 in fees), and managers risk paying for incomplete work — costing teams 15–20% of project budgets in disputes and refunds.

## Solution

TaskPay uses a Soroban smart contract to lock USDC into escrow per task. Workers submit on-chain proof of completion; managers approve with one transaction. USDC releases to the worker's Stellar wallet in under 5 seconds — no bank, no intermediary, no dispute delay. If work is rejected, the clawback mechanism returns funds to the manager automatically.

---

## Transaction Lifecycle

```
Manager creates task  →  USDC locked in contract escrow
        ↓
Worker accepts task   →  Worker address recorded on-chain
        ↓
Worker submits hash   →  Proof of completion stored on-chain
        ↓
Manager approves      →  USDC released to worker (<5 seconds)
   OR
Manager rejects       →  USDC clawed back to manager (Disputed)
```

---

## Stellar Features Used

| Feature               | Usage                                               |
|-----------------------|-----------------------------------------------------|
| USDC transfers        | Bounty payments from manager → escrow → worker      |
| Soroban smart contracts | Escrow logic, status state machine, access control |
| Trustlines            | Worker must trust USDC token before receiving funds |
| Clawback              | Dispute resolution — returns USDC to manager        |
| XLM                   | Transaction fees                                    |

---

## Vision & Purpose

TaskPay is infrastructure for the async, cross-border gig economy. Any team paying remote workers — in the Philippines, Indonesia, Vietnam, or beyond — can use TaskPay without a bank account or payment processor. The contract is composable: plugs into any Stellar anchor for local cash-out (GCash, GrabPay, M-Pesa). Future versions integrate an AI oracle that auto-verifies GitHub PR merges, enabling fully autonomous payroll.

---

## Prerequisites

| Tool              | Required Version |
|-------------------|-----------------|
| Rust              | 1.74+           |
| `wasm32` target   | via `rustup`    |
| Soroban CLI       | 21.x            |
| Stellar Testnet   | Friendbot funded |

Install the Wasm target:
```bash
rustup target add wasm32-unknown-unknown
```

Install Soroban CLI:
```bash
cargo install --locked soroban-cli --features opt
```

---

## Build

```bash
soroban contract build
```

Output: `target/wasm32-unknown-unknown/release/task_pay.wasm`

Optimise binary size (optional):
```bash
soroban contract optimize --wasm target/wasm32-unknown-unknown/release/task_pay.wasm
```

---

## Test

```bash
cargo test
```

Runs 5 tests:
1. **Happy path** — full create → accept → submit → approve lifecycle
2. **Edge case** — cannot approve before worker submits
3. **State verification** — contract storage reflects correct state post-completion
4. **Dispute path** — rejected submission claws USDC back to manager
5. **Cancel path** — open task cancel refunds manager in full

---

## Deploy to Testnet

Fund your account:
```bash
soroban keys generate manager --network testnet
soroban keys fund manager --network testnet
```

Deploy:
```bash
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/task_pay.wasm \
  --source manager \
  --network testnet
```

This outputs a `CONTRACT_ID`. Save it — you'll need it for all invocations.

---

## Sample CLI Invocations

Replace `<CONTRACT_ID>`, `<MANAGER_ADDRESS>`, `<USDC_TOKEN_ADDRESS>`, and `<WORKER_ADDRESS>` with real values.

**Create a task (lock 50 USDC into escrow):**
```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source manager \
  --network testnet \
  -- \
  create_task \
  --manager <MANAGER_ADDRESS> \
  --usdc_token <USDC_TOKEN_ADDRESS> \
  --bounty 500000000 \
  --description "Build login screen component" \
  --deadline_ledger 99999999
```

**Worker accepts task:**
```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source worker \
  --network testnet \
  -- \
  accept_task \
  --task_id 1 \
  --worker <WORKER_ADDRESS>
```

**Worker submits completion hash:**
```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source worker \
  --network testnet \
  -- \
  submit_completion \
  --task_id 1 \
  --completion_hash "abc123def456"
```

**Manager approves and releases USDC:**
```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source manager \
  --network testnet \
  -- \
  approve_and_release \
  --task_id 1
```

**Manager rejects submission (clawback):**
```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --source manager \
  --network testnet \
  -- \
  reject_submission \
  --task_id 1
```

**Read task state:**
```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- \
  get_task \
  --task_id 1
```

---

## Project Structure

```
taskpay/
├── Cargo.toml        # package manifest & Soroban dependencies
├── README.md         # this file
└── src/
    ├── lib.rs        # Soroban smart contract (escrow logic)
    └── test.rs       # 5 unit tests using soroban_sdk::testutils
```

---

## Deployed Contract Link
[1]https://stellar.expert/explorer/testnet/tx/c9e9ef4e341324fff40b2d1267fed2d3f898eebac73bcde9adedae971d5c0b47
[2]https://lab.stellar.org/smart-contracts/contract-explorer?$=network$id=testnet&label=Testnet&horizonUrl=https:////horizon-testnet.stellar.org&rpcUrl=https:////soroban-testnet.stellar.org&passphrase=Test%20SDF%20Network%20/;%20September%202015;&smartContracts$explorer$contractId=CAJ6GMQ6OV4OISK4ZWRJSQIL735CA4ANPVG2H2HIRXNATQCEFTMWEYUN;;


## License

MIT © 2024 TaskPay Contributors
