# 🎵 BeatSplit

**Instant, transparent, on-chain royalty splits for independent artists, built on Stellar's Soroban smart contracts.**

> Pay once. Everyone gets paid, immediately, exactly as agreed.

![Status](https://img.shields.io/badge/status-MVP%20in%20development-orange)
![Platform](https://img.shields.io/badge/platform-Stellar%20%2F%20Soroban-blue)
![Language](https://img.shields.io/badge/contracts-Rust-b7410e)
![License](https://img.shields.io/badge/license-MIT-green)

---

## Table of Contents

1. [The Problem](#the-problem)
2. [The Solution](#the-solution)
3. [Who It's For](#who-its-for)
4. [How It Works](#how-it-works)
5. [Scope and Honest Limitations](#scope-and-honest-limitations)
6. [Features](#features)
7. [Architecture](#architecture)
8. [Smart Contract Design](#smart-contract-design)
9. [Security Considerations](#security-considerations)
10. [Project Structure](#project-structure)
11. [Getting Started](#getting-started)
12. [Usage Examples](#usage-examples)
13. [Frontend](#frontend)
14. [Testing](#testing)
15. [Deployment](#deployment)
16. [Cash-Out: Fiat On/Off-Ramps](#cash-out-fiat-onoff-ramps)
17. [Roadmap](#roadmap)
18. [Legal and Compliance Notes](#legal-and-compliance-notes)
19. [Contributing](#contributing)
20. [FAQ](#faq)
21. [License](#license)

---

## The Problem

In emerging music scenes (Lagos, Nairobi, Accra, Jos, Johannesburg, Kingston, São Paulo, Manila and beyond), artists face two compounding money problems:

1. **Slow payouts.** Streaming and distribution revenue passes through several intermediaries (platform → aggregator → label or collecting society → artist) and often arrives 60 to 180 days after the listens happened.
2. **Manual splitting.** A single track usually involves a lead artist, featured artists, a producer, a songwriter, a beat-maker and sometimes a label or manager. Splits are agreed over WhatsApp or verbally, then settled by hand through bank transfers or mobile money. This causes disputes, missed payments, awkward conversations and little to no audit trail.

The people with the least financial cushion wait the longest, and trust between collaborators erodes when money is unclear.

## The Solution

**BeatSplit** is a Soroban smart contract suite plus a web app that does three things:

- **Records the agreement.** Collaborators and their percentage shares are registered on-chain, so the deal is public and tamper-resistant.
- **Splits automatically.** When revenue is paid into a split, the contract divides it by the agreed percentages and pays every collaborator in the same transaction.
- **Stays transparent.** Every deposit, payout and amendment is an on-chain event anyone with the split ID can verify.

Stellar is a good fit because it has fast finality (about 5 seconds), very low fees (fractions of a cent), native stablecoins such as USDC, and a mature network of **anchors** that convert on-chain dollars into local currency and mobile money.

## Who It's For

| User | What they get |
|---|---|
| **Artists / bands** | Set up a split in minutes, receive earnings instantly, show collaborators proof of payment |
| **Producers / songwriters** | Guaranteed share on every payment, no chasing anyone |
| **Labels / managers** | Programmatic, auditable payouts instead of manual accounting |
| **Promoters / venues** | Pay one address; the contract handles the artist's team |
| **Fans / buyers** | Pay directly for tickets, merch or tips, knowing the money reaches the whole team |
| **Developers** | Open contracts they can integrate with distributors, ticketing or merch platforms |

## How It Works

```
                    ┌───────────────────────────┐
  Ticket sale ─────▶│                           │──▶ Artist      50%
  Merch sale  ─────▶│   BeatSplit Soroban       │──▶ Producer    30%
  Fan tip     ─────▶│   Split Contract          │──▶ Songwriter  10%
  Distributor ─────▶│  (USDC / any Stellar      │──▶ Label       10%
  payout      ─────▶│   asset)                  │
                    └───────────────────────────┘
                       one deposit, N payouts,
                       one transaction, ~5 seconds
```

1. **Create a split.** The creator defines recipients and their shares in basis points (1 bp = 0.01%, total must equal 10,000).
2. **Everyone confirms (optional but recommended).** Each recipient signs to accept their share, which proves consent.
3. **Receive revenue.** Anyone can deposit the split's asset (e.g. USDC) into the contract.
4. **Auto-distribute.** The contract calculates each share with exact integer math and transfers it. If a transfer to a recipient fails, their share is held as a claimable balance so nobody else's payout is blocked.
5. **Cash out.** Recipients withdraw USDC to a local bank, mobile money wallet or cash agent via a Stellar anchor.

## Scope and Honest Limitations

Read this section before building a business plan around the project.

**Streaming platforms do not pay into smart contracts.** Spotify, Apple Music, YouTube and similar services pay distributors, labels or collecting societies in fiat, on their own schedule. BeatSplit cannot speed up that upstream delay by itself.

What BeatSplit makes instant is revenue that **starts on-chain or can be routed on-chain**:

| Revenue source | Instant today? | Notes |
|---|---|---|
| Direct fan payments, tips, digital sales | ✅ Yes | Native use case |
| Ticket sales (via integrated ticketing) | ✅ Yes | Payer sends USDC to the split |
| Merch sales | ✅ Yes | Via checkout integration or manual deposit |
| Live-show promoter payments | ✅ Yes | Promoter pays the split address |
| Distributor payouts | ⚠️ Partial | Requires a distributor that pays into the contract, or an operator who deposits fiat-converted USDC |
| Streaming royalties direct from platforms | ❌ No | Controlled upstream; see the advance pool idea in the [Roadmap](#roadmap) |

**MVP strategy:** prove the split contract with direct revenue first, then tackle streaming through distributor partnerships and an advance-liquidity pool.

## Features

### MVP (v0.1)
- ✅ Create splits with 2 to 20 recipients, shares in basis points
- ✅ Deposit any Stellar Asset Contract (SAC) token, with USDC as the primary target
- ✅ Automatic distribution on deposit, with a claimable-balance fallback for failed transfers
- ✅ Exact math: rounding remainder ("dust") is assigned deterministically
- ✅ Recipient acceptance (consent) before a split goes active
- ✅ Amendments that require unanimous approval of current recipients
- ✅ Freeze/lock a split to make it permanently immutable
- ✅ Full event log (create, accept, deposit, payout, claim, amend, lock)
- ✅ Read-only queries: split details, per-recipient lifetime earnings, claimable balances

### Planned
- 🔜 Multi-asset splits and per-split allowed-token lists
- 🔜 Tiered / recoupment splits (e.g. label recoups an advance first, then the split changes)
- 🔜 Streaming-style continuous payouts
- 🔜 Royalty advance pool
- 🔜 NFT or share-token representation of a split position (tradable royalty rights)
- 🔜 Fiat anchor integration inside the app (SEP-24 / SEP-31)
- 🔜 Mobile-first PWA with offline-tolerant UX

## Architecture

```
┌──────────────────────┐        ┌────────────────────────────┐
│  Web App (Next.js)   │◀──────▶│  Stellar RPC / Horizon     │
│  - Create/manage     │        │  (testnet → mainnet)       │
│  - Dashboard         │        └─────────────┬──────────────┘
│  - Wallet connect    │                      │
└─────────┬────────────┘                      ▼
          │                      ┌────────────────────────────┐
          │  signs with          │  BeatSplit Soroban         │
          ▼  Freighter / xBull   │  Contract (Rust → WASM)    │
┌──────────────────────┐         │  - Split registry          │
│  User Wallet         │         │  - Distribution engine     │
└──────────────────────┘         │  - Claim ledger            │
                                 └─────────────┬──────────────┘
                                               │ token transfers
                                               ▼
                                 ┌────────────────────────────┐
                                 │  USDC (Stellar Asset       │
                                 │  Contract)                 │
                                 └─────────────┬──────────────┘
                                               │ cash-out
                                               ▼
                                 ┌────────────────────────────┐
                                 │  Anchors (SEP-24/31):      │
                                 │  bank, mobile money, cash  │
                                 └────────────────────────────┘

Optional off-chain: indexer (events → Postgres) for fast dashboards and notifications.
```

## Smart Contract Design

### Data model

```rust
#[contracttype]
pub struct Recipient {
    pub addr: Address,
    pub bps: u32,          // basis points; all recipients sum to 10_000
}

#[contracttype]
pub enum SplitStatus {
    Pending,   // created, awaiting recipient acceptance
    Active,    // all recipients accepted; deposits distribute
    Locked,    // active and permanently immutable
}

#[contracttype]
pub struct Split {
    pub id: u64,
    pub creator: Address,
    pub token: Address,            // SAC token address (e.g. USDC)
    pub recipients: Vec<Recipient>,
    pub status: SplitStatus,
    pub metadata_hash: BytesN<32>, // hash of off-chain JSON (title, ISRC, notes)
    pub total_received: i128,
    pub version: u32,              // increments on every amendment
}
```

### Storage keys

| Key | Value | Storage type |
|---|---|---|
| `Split(id)` | `Split` | Persistent |
| `Accepted(id, addr)` | `bool` | Persistent |
| `Claimable(id, addr)` | `i128` | Persistent |
| `Earned(id, addr)` | `i128` (lifetime) | Persistent |
| `Proposal(id)` | pending amendment + approvals | Persistent |
| `NextId` | `u64` | Instance |

> TTL management: persistent entries must have their TTL extended on write/read paths, and the app should run a keeper that periodically bumps TTLs for active splits.

### Public interface

| Function | Auth required | Description |
|---|---|---|
| `create_split(creator, token, recipients, metadata_hash) → id` | creator | Validates and registers a split in `Pending` state |
| `accept(id, recipient)` | recipient | Records consent; when all accept, status becomes `Active` |
| `deposit(id, from, amount)` | from | Pulls tokens from the payer and distributes immediately |
| `distribute_balance(id)` | none | Distributes any tokens sent directly to the contract for this split (optional pattern) |
| `claim(id, recipient)` | recipient | Withdraws a held (failed-push) balance |
| `propose_amendment(id, proposer, new_recipients)` | proposer (must be a recipient) | Opens an amendment |
| `approve_amendment(id, recipient)` | recipient | Records approval; applied automatically when everyone approves |
| `cancel_amendment(id, recipient)` | recipient | Withdraws a proposal |
| `lock(id, caller)` | all recipients | Makes the split permanently immutable |
| `get_split(id)` | none | Read split |
| `get_claimable(id, addr)` | none | Read held balance |
| `get_earned(id, addr)` | none | Read lifetime earnings |

### Distribution algorithm

All math uses integer arithmetic on the token's smallest unit (USDC on Stellar has 7 decimals).

```
for each recipient i (except the designated remainder recipient):
    share_i = amount * bps_i / 10_000        // floor division
remainder_recipient_share = amount - sum(share_i)   // absorbs rounding dust
```

The remainder recipient is the first recipient in the list (typically the lead artist), which is documented in the metadata and visible on-chain. Use `checked_mul` / `checked_add` everywhere and reject amounts that would overflow.

### Failure-isolated payouts

A single bad recipient (for example, a closed or untrusted account) must never block everyone else.

```
for each share:
    match token_client.try_transfer(contract, recipient, share) {
        Ok(_)  => emit Payout(id, recipient, share)
        Err(_) => Claimable(id, recipient) += share; emit Held(id, recipient, share)
    }
```

The held balance can be withdrawn later with `claim`, to the same address or after the recipient fixes their account.

### Events

`SplitCreated`, `Accepted`, `Activated`, `Deposited`, `Payout`, `Held`, `Claimed`, `AmendmentProposed`, `AmendmentApproved`, `AmendmentApplied`, `AmendmentCancelled`, `Locked`.

These events feed the indexer and give every collaborator a verifiable payment history.

### Amendment rules

- Any recipient may propose a new recipient list.
- **Every** current recipient must approve; there are no admin overrides.
- Approvals reset if the proposal changes.
- Applying an amendment increments `version`; past payouts are unaffected.
- A `Locked` split cannot be amended, which is useful once a deal is final.

## Security Considerations

Smart contracts that move money need conservative design. Planned safeguards:

- **Authorization:** every state-changing function calls `require_auth()` on the right address. No privileged admin can redirect funds.
- **Validation:** shares sum to exactly 10,000 bps; no zero-share recipients; no duplicate addresses; positive deposit amounts; bounded recipient count (≤ 20) to cap gas/resource use.
- **Reentrancy:** update accounting state before external token calls (checks-effects-interactions), even though Soroban's execution model limits classic reentrancy.
- **Integer safety:** checked arithmetic; `i128` amounts; explicit rejection of negative values.
- **Token trust:** a split is bound to a single token address at creation. Use only well-known SACs (e.g. Circle's USDC) in production.
- **Griefing resistance:** pull-based claim fallback prevents one recipient from blocking payouts.
- **Upgradeability:** the MVP contract is non-upgradeable. If an upgrade path is added later, it must be controlled by a multisig with a time lock and publicly announced.
- **Audits:** a professional third-party audit is required before handling real funds on mainnet. Until then, run on testnet only or with strict deposit caps.
- **Responsible disclosure:** see `SECURITY.md` for how to report vulnerabilities.

> ⚠️ **This software is unaudited. Do not use it with significant real funds until an audit is complete.**

## Project Structure

```
beatsplit/
├── contracts/
│   └── beatsplit/
│       ├── src/
│       │   ├── lib.rs          # contract entry points
│       │   ├── types.rs        # Split, Recipient, enums
│       │   ├── storage.rs      # storage helpers + TTL extension
│       │   ├── distribute.rs   # share math + failure-isolated payout
│       │   ├── amend.rs        # amendment workflow
│       │   ├── events.rs       # event definitions
│       │   ├── errors.rs       # contracterror enum
│       │   └── test.rs         # unit + integration tests
│       └── Cargo.toml
├── app/                        # Next.js web app
│   ├── src/
│   │   ├── components/
│   │   ├── lib/stellar.ts      # RPC + contract client helpers
│   │   └── pages/
│   └── package.json
├── indexer/                    # optional event indexer (Node/TS)
├── scripts/
│   ├── deploy-testnet.sh
│   └── seed-demo.sh
├── docs/
│   ├── ARCHITECTURE.md
│   ├── THREAT_MODEL.md
│   └── ANCHOR_INTEGRATION.md
├── Cargo.toml                  # workspace
├── SECURITY.md
├── CONTRIBUTING.md
├── LICENSE
└── README.md
```

## Getting Started

### Prerequisites

- **Rust** (stable) with the WASM target: `rustup target add wasm32v1-none` (or `wasm32-unknown-unknown` on older toolchains, check the current Stellar docs)
- **Stellar CLI**: <https://developers.stellar.org/docs/tools/cli>
- **Node.js** 20+ and npm/pnpm (for the web app)
- **Freighter** wallet browser extension (for the web app): <https://www.freighter.app>

### Clone and build

```bash
git clone https://github.com/<your-username>/beatsplit.git
cd beatsplit

# Build the contract
stellar contract build
```

The compiled WASM will be at `target/wasm32v1-none/release/beatsplit.wasm` (path varies with toolchain).

### Set up a testnet identity

```bash
stellar keys generate alice --network testnet --fund
stellar keys address alice
```

### Run tests

```bash
cargo test
```

## Usage Examples

> Command syntax follows the Stellar CLI; flags can change between versions, so check `stellar contract invoke --help`.

### Deploy to testnet

```bash
stellar contract deploy \
  --wasm target/wasm32v1-none/release/beatsplit.wasm \
  --source alice \
  --network testnet \
  --alias beatsplit
```

### Create a split (50% artist, 30% producer, 20% label)

```bash
stellar contract invoke --id beatsplit --source alice --network testnet -- \
  create_split \
  --creator <ARTIST_ADDRESS> \
  --token <USDC_SAC_ADDRESS> \
  --recipients '[
    {"addr":"<ARTIST_ADDRESS>","bps":5000},
    {"addr":"<PRODUCER_ADDRESS>","bps":3000},
    {"addr":"<LABEL_ADDRESS>","bps":2000}
  ]' \
  --metadata_hash <32_BYTE_HEX>
```

### Accept your share

```bash
stellar contract invoke --id beatsplit --source producer --network testnet -- \
  accept --id 1 --recipient <PRODUCER_ADDRESS>
```

### Pay into a split

```bash
stellar contract invoke --id beatsplit --source fan --network testnet -- \
  deposit --id 1 --from <FAN_ADDRESS> --amount 100000000   # 10.0000000 USDC (7 decimals)
```

Result: artist receives 5.0, producer 3.0, label 2.0, all in one transaction.

### Inspect a split

```bash
stellar contract invoke --id beatsplit --network testnet -- get_split --id 1
stellar contract invoke --id beatsplit --network testnet -- get_earned --id 1 --addr <PRODUCER_ADDRESS>
```

### Minimal contract sketch (Rust)

```rust
#![no_std]
use soroban_sdk::{contract, contractimpl, token, Address, Env, Vec};

#[contractimpl]
impl BeatSplit {
    pub fn deposit(env: Env, id: u64, from: Address, amount: i128) -> Result<(), Error> {
        from.require_auth();
        if amount <= 0 { return Err(Error::InvalidAmount); }

        let mut split = load_split(&env, id)?;
        if split.status == SplitStatus::Pending { return Err(Error::NotActive); }

        let token = token::Client::new(&env, &split.token);
        token.transfer(&from, &env.current_contract_address(), &amount);

        // compute shares (floor), remainder to first recipient
        let shares = compute_shares(&env, &split.recipients, amount)?;

        // effects first
        split.total_received = split.total_received.checked_add(amount).ok_or(Error::Overflow)?;
        save_split(&env, &split);

        // interactions: failure-isolated payouts
        for (r, share) in split.recipients.iter().zip(shares.iter()) {
            payout_or_hold(&env, &token, id, &r.addr, share);
        }
        Ok(())
    }
}
```

*(Illustrative only; the real implementation lives in `contracts/beatsplit/src`.)*

## Frontend

The web app is a thin, wallet-signed client over the contract. Key screens:

1. **Create Split:** add collaborators by Stellar address, set percentages with a live total indicator, attach metadata (track title, ISRC, notes), submit.
2. **Invite & Accept:** share a link; collaborators connect a wallet and accept their share.
3. **Dashboard:** lifetime earnings, recent payouts, held balances, per-split activity feed.
4. **Pay a Split:** public payment page (QR code + link) for fans, promoters and merch checkouts.
5. **Amendments:** propose, review, approve.
6. **Cash Out:** deep link to an anchor for converting USDC to local currency.

Design principles: mobile-first, low-bandwidth, plain-language ("your share", not "basis points"), and support for people without prior crypto experience.

Run it:

```bash
cd app
cp .env.example .env.local   # set NEXT_PUBLIC_CONTRACT_ID, NEXT_PUBLIC_RPC_URL, NEXT_PUBLIC_NETWORK
npm install
npm run dev
```

## Testing

| Layer | Approach |
|---|---|
| Unit | Share math: exact sums, dust handling, extreme values, 1-stroop deposits |
| Contract | `soroban-sdk` testutils with mocked auth: create, accept, deposit, claim, amend, lock |
| Property-based | Random recipient sets and amounts; invariant: `sum(payouts + held) == deposit` |
| Failure paths | Recipient transfer failure → held balance; unauthorized calls; duplicate/zero/oversum recipients |
| Integration | Testnet end-to-end with USDC test asset |
| Frontend | Component tests + Playwright flows against testnet |

Core invariants that must always hold:

1. `sum(shares) == amount` for every deposit.
2. No funds are ever left unaccounted for in the contract (distributed or claimable).
3. A locked split's recipient list never changes.
4. No address other than the recipient can claim that recipient's held balance.

## Deployment

1. **Testnet:** deploy, run the seed script, share with pilot artists.
2. **Pre-mainnet checklist:**
   - [ ] External security audit completed and findings resolved
   - [ ] Deposit caps configured for the initial rollout
   - [ ] TTL keeper running
   - [ ] Monitoring and alerting on contract events
   - [ ] Incident response plan documented
   - [ ] Verified USDC issuer address configured
3. **Mainnet:** deploy, verify the WASM hash publicly, publish the contract ID, announce.

## Cash-Out: Fiat On/Off-Ramps

On-chain speed only helps if people can spend the money. BeatSplit is designed to pair with Stellar anchors that support:

- **SEP-24** (interactive deposit/withdrawal) for bank transfer, mobile money and cash pickup
- **SEP-31** (cross-border payments) for business-to-business flows
- **SEP-10** (authentication) and **SEP-12** (KYC) as required by each anchor

Regional anchor availability changes frequently, so verify current options and supported countries in the Stellar Anchor Directory before promising users a cash-out route. Product work should include a clear "How do I turn this into naira / shillings / cedis / rand?" guide for each launch market.

## Roadmap

**Phase 1: Foundation (MVP)**
- Core split contract, consent, amendments, locking
- Testnet web app, pilot with a handful of artists

**Phase 2: Real revenue**
- Ticketing and merch checkout integrations (pay-to-split links, QR codes)
- Anchor cash-out integration in-app
- Indexer, notifications (email/WhatsApp/SMS)
- External audit, capped mainnet launch

**Phase 3: Streaming bridge**
- Distributor partnerships that settle into split contracts
- CSV/API import of distributor statements for reconciliation
- **Royalty advance pool:** liquidity providers fund artists against expected streaming income; incoming royalties automatically repay the pool through a recoupment tier in the split

**Phase 4: Ecosystem**
- Tradable royalty-share tokens
- Multi-asset splits and tiered/waterfall payouts
- Public SDK and API for third-party platforms
- Governance for shared infrastructure parameters

## Legal and Compliance Notes

*This is general information, not legal advice. Consult a lawyer in your jurisdiction.*

- **A smart contract split is a payment mechanism, not a copyright contract.** Artists should still have a written agreement covering ownership, credits and licensing; the on-chain split enforces the payment portion.
- **Metadata hash:** anchoring an agreement document's hash on-chain gives a timestamped record that it existed in that form.
- **Tax:** earnings may be taxable income depending on where each collaborator lives. The app should make payment history easy to export.
- **KYC/AML:** on/off-ramp partners handle regulated identity checks. If you operate any custodial or fiat-conversion component yourself, you may need licensing.
- **Securities risk:** tradable royalty-share tokens or investor-funded advance pools may be regulated as securities in many jurisdictions. Get legal review before building those features.
- **Collecting societies:** an artist's rights may be assigned to a PRO or CMO; confirm that on-chain splits don't conflict with existing assignments.

## Contributing

Contributions are welcome, especially from musicians, producers, and developers in the markets this project serves.

1. Fork the repo and create a feature branch: `git checkout -b feat/my-feature`
2. Write tests for new behavior
3. Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test`
4. Open a pull request describing the change and its motivation

Good first issues are labeled `good first issue`. For contract changes, please open an issue to discuss the design first, since this code moves money. See `CONTRIBUTING.md` and `SECURITY.md`.

## FAQ

**Does this make Spotify pay me faster?**
No. It makes everything that touches the contract instant. Streaming income needs a distributor that pays into it, or an advance pool (see the [Roadmap](#roadmap)).

**What currency do I get paid in?**
USDC (a dollar stablecoin) by default. You can cash out to local currency through supported anchors.

**What if a collaborator's wallet has a problem?**
Their share is held in the contract as a claimable balance and nobody else's payout is delayed.

**Can someone change the split behind my back?**
No. Changes require approval from every recipient, and a split can be locked permanently.

**Do my collaborators need to understand crypto?**
They need a Stellar wallet and a cash-out route. The app aims to hide the jargon, but wallet onboarding is a real UX challenge, and solving it is a core project goal.

**Is it safe?**
It is unaudited software today. Use testnet or small capped amounts until an audit is published.

**Why Stellar/Soroban?**
Fast, cheap transactions, native stablecoins, and a mature anchor network for local cash-out.

## License

MIT, see [LICENSE](./LICENSE).

---

*Built for the artists who make the music and deserve to be paid for it, on time.*
