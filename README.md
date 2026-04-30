# TaskPay

> On-chain task management with milestone-based USDC escrow on Stellar

---

## Project Description

**TaskPay** is a decentralised task management and payroll protocol built on the Stellar blockchain using Soroban smart contracts. It replaces the trust gap between remote managers and contractors with cryptographic escrow — USDC is locked on-chain when a task is created, and released automatically once the manager approves the worker's submitted proof of completion.

### Who it's for

| Role | Description |
|------|-------------|
| **Project Managers** | Remote-first SME leads, startup CTOs, agency owners who hire cross-border freelancers |
| **Freelancers / Workers** | Designers, developers, writers in SEA (PH, ID, VN) paid in USDC with instant settlement |
| **DAOs / Collectives** | Decentralised teams that need transparent, auditable task-bounty payouts |

### How it works — at a glance

TaskPay introduces a four-role contract interaction:

1. **Manager** posts a task with a USDC bounty → funds locked in escrow
2. **Worker** accepts and completes the task → submits an on-chain proof hash
3. **Manager** reviews and approves → USDC auto-transferred to worker in <5 seconds
4. **Dispute?** Manager rejects → USDC automatically clawed back, no court needed

### Why Stellar

- **Speed** — 3–5 second finality vs 2–5 day bank wires
- **Cost** — sub-cent transaction fees vs $15–30 SWIFT charges
- **USDC-native** — Circle's USDC runs natively on Stellar, no wrapping or bridging
- **Composable** — integrates with any SEA Stellar anchor (GCash, GrabPay, M-Pesa) for local cash-out
- **Clawback** — built-in Stellar feature enables trustless dispute resolution without arbitrators

### Key differentiators vs existing tools

| Feature | TaskPay | Upwork / Fiverr | PayPal | Bank Wire |
|---------|---------|-----------------|--------|-----------|
| Settlement time | <5 seconds | 3–7 days | 1–3 days | 2–5 days |
| Fees | <$0.01 | 10–20% | 3–5% | $15–30 |
| Dispute resolution | Automatic (clawback) | Manual review | Manual review | Legal process |
| Bank account required | No | Yes | Yes | Yes |
| On-chain audit trail | Yes | No | No | No |

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

## Deployed Contract Details

The TaskPay contract is deployed on the **Stellar Testnet** for demo and integration testing purposes.

| Property | Value |
|----------|-------|
| **Network** | Stellar Testnet |
| **Contract ID** | `CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX` |
| **Deployer Address** | `GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX` |
| **USDC Token (Testnet)** | `GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5` |
| **Soroban SDK Version** | `21.0.0` |
| **Wasm Hash** | `xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx` |
| **Deployed On** | `YYYY-MM-DD` |
| **Ledger at Deploy** | `XXXXXXXX` |
| **Explorer** | [View on Stellar Expert](https://stellar.expert/explorer/testnet/contract/CXXXXXXX) |

> **Note:** Replace all placeholder values above with real values after running `soroban contract deploy`. The Wasm hash is returned by the Soroban CLI and can also be verified on the Stellar explorer.

### Verifying the deployment

After deployment, confirm the contract is live:

```bash
soroban contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- \
  get_task_count
```

Expected output: `0` (no tasks created yet)

### Testnet USDC setup

To test with USDC on testnet, fund a keypair and set up a trustline:

```bash
# Fund a new keypair via Friendbot
soroban keys generate worker --network testnet
soroban keys fund worker --network testnet

# Check XLM balance
soroban contract invoke \
  --id <USDC_TOKEN_ADDRESS> \
  --network testnet \
  -- \
  balance \
  --id <WORKER_ADDRESS>
```

### Network configuration reference

```toml
# ~/.config/soroban/config.toml
[networks.testnet]
rpc-url            = "https://soroban-testnet.stellar.org"
network-passphrase = "Test SDF Network ; September 2015"
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

## Future Scope

TaskPay  is an MVP escrow contract. The roadmap below outlines how it evolves into full payroll infrastructure for the decentralised gig economy.

### Phase 1 — Foundation (Current MVP)
- [x] Soroban escrow contract with USDC bounties
- [x] Create / Accept / Submit / Approve / Reject / Cancel lifecycle
- [x] On-chain event emission for off-chain indexing
- [x] Clawback-based dispute resolution
- [x] Testnet deployment

### Phase 2 — Multi-milestone & Recurring Payroll
- [ ] **Milestone contracts** — split a single project into multiple escrow tranches, each with its own deadline and bounty
- [ ] **Recurring payroll streams** — weekly/biweekly USDC streams using Soroban time-based unlock (streaming payments)
- [ ] **Multi-approver** — require M-of-N manager signatures before release (DAO-friendly)
- [ ] **Partial releases** — approve and release a percentage of escrow (e.g. 50% upfront, 50% on completion)

### Phase 3 — AI Oracle Integration
- [ ] **Automated verification** — an off-chain AI oracle watches GitHub/GitLab for merged PRs and triggers `approve_and_release` without manual manager action
- [ ] **Work quality scoring** — LLM-based rubric evaluates submitted deliverables (design files, written content, code) and recommends approve/reject with a confidence score
- [ ] **Fraud detection** — flag suspicious completion hashes (duplicated submissions, plagiarised work) before manager review

### Phase 4 — Anchor & Wallet Integration
- [ ] **GCash anchor** — Filipino workers can cash out USDC to GCash wallets directly from TaskPay earnings
- [ ] **GrabPay / OVO / Dana** — SEA-wide anchor integrations for Indonesia and Vietnam
- [ ] **M-Pesa bridge** — extend to East Africa for pan-emerging-market coverage
- [ ] **Lobstr / Freighter deep-link** — one-tap wallet UX for mobile workers; no CLI needed

### Phase 5 — On-chain Reputation & Identity
- [ ] **Worker reputation NFT** — non-transferable Soroban token minted per completed task; score visible to future managers
- [ ] **Manager trust score** — track approval rate, dispute rate, and avg payout speed on-chain
- [ ] **Decentralised KYC** — integrate with Stellar's SEP-12 for compliant identity without centralised databases
- [ ] **Credential badges** — verifiable on-chain proof of skills (e.g. "10 React tasks completed, 100% approval rate")

### Phase 6 — DeFi Composability
- [ ] **Yield on idle escrow** — escrowed USDC earns yield via a Soroban-compatible lending protocol while awaiting approval
- [ ] **Built-in DEX swap** — workers can instantly swap earned USDC to XLM or any Stellar asset via the built-in DEX on payout
- [ ] **Liquidity pool staking** — TaskPay treasury stakes platform fees into Stellar DEX liquidity pools
- [ ] **Token-gated task boards** — post tasks only claimable by holders of specific Stellar tokens (e.g. a DAO's governance token)

### Phase 7 — Governance
- [ ] **TaskPay DAO** — platform fee governance via on-chain voting using a custom Stellar asset
- [ ] **Dispute arbitration panel** — community-elected arbitrators resolve disputes with stake-weighted voting
- [ ] **Open grant board** — NGOs and impact orgs post grant-funded tasks claimable by verified community members

---
## Deployed Contract Link
[1]https://stellar.expert/explorer/testnet/tx/c9e9ef4e341324fff40b2d1267fed2d3f898eebac73bcde9adedae971d5c0b47
[2] https://lab.stellar.org/r/testnet/contract/CAJ6GMQ6OV4OISK4ZWRJSQIL735CA4ANPVG2H2HIRXNATQCEFTMWEYUN

## License

MIT © 2024 TaskPay Contributors
