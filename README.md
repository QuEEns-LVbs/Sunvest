# Sunvest

**A solar companion app for Nigeria: track your power system, earn XLM for verified clean energy, invest in community solar projects, and send money to friends, all on the Stellar Network.**

![License: MIT](https://img.shields.io/badge/License-MIT-green) ![Network: Stellar](https://img.shields.io/badge/Network-Stellar-blue) ![Contracts: Soroban](https://img.shields.io/badge/Contracts-Soroban-purple) ![Monorepo: pnpm](https://img.shields.io/badge/Monorepo-pnpm-orange) ![Build: Turborepo](https://img.shields.io/badge/Build-Turborepo-black)

> Sun plus invest. Sunvest turns every kilowatt-hour you generate into something you can see, prove, and earn from.

---

## What is Sunvest?

Millions of Nigerians depend on solar panels, inverters, and batteries because the grid is unreliable. Yet most of them cannot easily see how their system performs, how much money it saves compared to petrol or diesel generators, or whether it is failing. Meanwhile, Stellar needs everyday, real-world applications in Africa that go beyond trading.

Sunvest connects the two. Users link their solar system (or log it manually), and the app turns real energy data into three useful things:

1. **Insight:** a clear dashboard of generation, consumption, battery health, and money saved.
2. **Proof:** verified readings anchored on Stellar by Soroban smart contracts.
3. **Earnings and opportunity:** XLM rewards for clean energy, and the ability to invest small amounts in real community solar projects.

| Problem today | Sunvest's answer |
|---|---|
| People don't know how their solar system is performing | Live dashboard, alerts, and an AI energy advisor |
| Clean energy use is never rewarded | Sponsor-funded XLM rewards for verified generation |
| Small savers cannot invest in energy projects | Fractional stakes in community solar installations |
| Crypto apps in Nigeria focus on speculation | A daily-use utility that gives people a real reason to hold a Stellar wallet |
| Sending money and staying in touch are separate apps | Built-in chat, payments, and referrals |
| Hard to trust solar installers | Installer credentials issued as non-transferable Stellar assets |

---

## Feature Overview

### 1. Solar Link
- Connect a supported inverter or battery through the manufacturer's cloud API or a low-cost smart meter. Supported brands will expand over time (for example Victron, Growatt, and Deye).
- **Manual mode** for users without connected hardware: enter meter readings and get the same insights.
- Live dashboard: solar generation, household consumption, battery level, grid or generator use.
- **Money Saved tracker:** compares your solar usage against generator fuel and grid tariffs.
- Alerts for low battery, abnormal consumption, and possible panel faults.

### 2. Proof-of-Sun Rewards
- Verified energy readings are hashed and anchored on Stellar through a Soroban contract.
- Users earn small **XLM rewards** for verified clean energy generated, funded by sponsors such as solar companies, impact funders, and carbon-credit buyers.
- Anti-fraud checks compare readings against expected output for the location and weather, flagging impossible numbers.

### 3. Sun Shares (community solar investing)
- Users can buy small stakes in real solar installations, such as a school, clinic, or market, using USDC or XLM.
- Returns come from energy sold or saved and are distributed automatically by a Soroban contract.
- Launches as a **regulated pilot with a licensed partner** (see [Compliance](#compliance-and-responsible-design)).

### 4. Friends, payments, and referrals
- Send and request XLM or USDC using a username, with local-currency on and off ramps through Stellar anchors.
- Chat with built-in payments, for example splitting a solar installment or fuel bill.
- **Referral rewards** are paid only when the referred friend connects a real system and produces verified data, which prevents fake-account farming.

### 5. Sun Feed (community)
- A focused feed for tips, troubleshooting, installer reviews, and project updates.
- Groups by city, system type, or installer.
- Verified installers display on-chain credential badges.

### 6. AI Energy Advisor
- Ask questions such as "Why is my battery draining so fast?" or "Can I run a freezer on my system?" and get answers based on your own usage data.
- Recommends system sizing, appliance changes, and best times to run heavy loads.
- **Daily Digest:** a plain-language summary of solar, Nigerian energy, and crypto news.
- Advice is informational only and does not replace a qualified installer.

### 7. Market Pulse (read-only)
- Watchlist for energy-sector stocks (Nigerian and foreign), XLM, and major crypto assets.
- News and price alerts only. **No trading and no payouts tied to price movements.**

---

## Monorepo Structure

```
sunvest/
├── packages/
│   ├── contracts/     # @sunvest/contracts — Soroban smart contracts (Rust)
│   ├── gateway/       # @sunvest/gateway   — Device data ingestion and verification (TypeScript)
│   ├── api/           # @sunvest/api       — Backend API, chat, and payments (Node.js)
│   ├── brain/         # @sunvest/brain     — AI advisor and news digest (TypeScript + Python)
│   └── app/           # @sunvest/app       — Mobile-first PWA / React Native client
├── docs/              # Architecture, methodology, API reference
├── scripts/           # Testnet bootstrap, seed data, deploy helpers
├── audits/            # Smart contract audit reports
├── pnpm-workspace.yaml
├── turbo.json
└── README.md          ← you are here
```

---

## Packages

### @sunvest/contracts
Soroban smart contracts written in Rust, deployable to Stellar Testnet and Mainnet.

| Contract | Purpose |
|---|---|
| `SunProof` | Anchors hashes of verified energy readings per device and period |
| `RewardPool` | Holds sponsor funds and releases XLM rewards against verified readings |
| `SunShares` | Manages community solar projects, investor stakes, and revenue distribution |
| `ReferralVault` | Tracks referrals and pays rewards once verification conditions are met |
| `InstallerRegistry` | Issues and revokes installer credentials |

```
packages/contracts/
├── src/
│   ├── sun_proof.rs
│   ├── reward_pool.rs
│   ├── sun_shares.rs
│   ├── referral_vault.rs
│   └── installer_registry.rs
├── tests/
├── Cargo.toml
└── README.md
```

**Stack:** Rust · Soroban SDK · Stellar CLI

### @sunvest/gateway
Collects readings from connected devices, validates them, and submits proofs on-chain.

**Responsibilities**
- Pull data from inverter cloud APIs and smart meters, or receive manual entries
- Normalise readings into a single format
- Run anti-fraud checks (expected output by location, time, and weather)
- Batch hashes and submit them to `SunProof`
- Expose health metrics for monitoring

```
packages/gateway/
├── src/
│   ├── connectors/      # One adapter per inverter brand
│   ├── validator.ts     # Anomaly and fraud checks
│   ├── batcher.ts       # Daily proof batching
│   ├── submitter.ts     # Soroban invocation
│   └── metrics.ts
├── Dockerfile
├── package.json
└── README.md
```

**Stack:** TypeScript · @stellar/stellar-sdk · Docker · Prometheus

### @sunvest/api
The backend for accounts, feed, chat, referrals, and payments.

**Key endpoints**
```
POST /v1/auth/challenge                 SEP-10 wallet challenge
POST /v1/devices                        Link a solar device
GET  /v1/devices/:id/stats              Generation, usage, savings
POST /v1/readings                       Submit a manual reading
GET  /v1/rewards                        Reward history and pending claims
GET  /v1/projects                       Community solar projects
POST /v1/projects/:id/invest            Invest in a project
POST /v1/referrals                      Create a referral link
GET  /v1/feed                           Community feed
POST /v1/payments/send                  Send XLM or USDC to a friend
GET  /v1/market/watchlist               Read-only market data
```

**Stack:** Node.js · Fastify · WebSockets · PostgreSQL · Redis · @stellar/stellar-sdk

### @sunvest/brain
The AI layer, kept separate so models can be swapped or upgraded independently.

```
packages/brain/
├── src/
│   ├── advisor/          # Energy Q&A using the user's own data
│   ├── sizing/           # System and appliance recommendations
│   ├── digest/           # Daily news summarisation
│   └── fraud_signals/    # Optional model-based anomaly scoring
├── package.json
└── README.md
```

**Stack:** TypeScript · Python · Anthropic Claude API · pgvector

### @sunvest/app
The user-facing app, built mobile-first for low-bandwidth devices with offline support.

| Screen | Description |
|---|---|
| `/` | Home dashboard: power, battery, savings, daily reward |
| `/devices` | Connect and manage solar systems |
| `/rewards` | XLM earnings and pending claims |
| `/invest` | Browse and fund community solar projects |
| `/wallet` | Balances, send, receive, on and off ramp |
| `/chat` | Messages and payment requests |
| `/feed` | Community posts and installer reviews |
| `/advisor` | AI Energy Advisor |
| `/market` | Read-only watchlist and news |
| `/refer` | Referral link and status |

**Stack:** Next.js (PWA) or React Native · TailwindCSS · @stellar/stellar-sdk · Freighter / LOBSTR / xBull · Recharts

---

## How It Works

```
User / Device           Gateway                    Stellar / Soroban
     │                     │                              │
     ├─ Solar data ───────▶│ validate + normalise         │
     │                     ├─ daily batch hash ──────────▶ SunProof.anchor()
     │                     │                              │
     │◀──── XLM reward ────┼──────────────────────────────┤ RewardPool.release()
     │                     │                              │
     ├─ Invest (USDC/XLM) ─┼─────────────────────────────▶ SunShares.invest()
     │◀──── revenue share ─┼──────────────────────────────┤ SunShares.distribute()
     │                     │                              │
     ├─ Refer a friend ────┼─────────────────────────────▶ ReferralVault.register()
     │◀──── bonus once friend's data is verified ─────────┤
```

---

## Why Stellar?

- **Low fees:** micro-rewards and small payments are practical, which would be hard on higher-cost networks.
- **Real-world anchors:** Stellar's anchor ecosystem (SEP-24 and SEP-31) supports moving between Naira and USDC.
- **Soroban smart contracts:** rewards, investment payouts, and referral rules need programmable logic.
- **Asset controls:** authorization flags and clawback allow revocable, non-transferable installer credentials.
- **Wallet reach:** SEP-10 authentication and mobile wallets avoid the need for a traditional bank account.

---

## Token Economics

Sunvest creates **no proprietary token**. Everything runs on XLM and Stellar USDC.

| Action | Asset | Funded by |
|---|---|---|
| Verified clean energy generated | XLM | Sponsor-funded reward pools |
| Successful referral (friend connects and verifies a system) | XLM | Referral budget in `RewardPool` |
| Community solar investment | USDC / XLM | Investors |
| Revenue share from solar projects | USDC | Energy sales or savings from each project |
| Installer credential issuance | XLM | Small issuance fee |
| Platform fee | USDC | Small percentage on investments and payments |

Rewards are capped per user per day so that the reward pool stays sustainable.

---

## Pilot Goal and Reward Funding

**Measurable target:** 10,000 verified kWh anchored on Stellar during the pilot, from real households and small businesses.

**How rewards are funded (not just promised):**
- Sponsors such as solar companies and impact funders that want proof of real clean energy use
- Carbon-credit buyers who need verifiable generation data
- A small platform fee on investments and payments

If sponsor funding runs low, rewards scale down through daily caps rather than being promised and left unpaid.

---

## Compliance and Responsible Design

- **Investing:** Sun Shares involves investment products. The pilot will be run with a licensed partner and in line with Nigerian regulations. Features may be limited by region until compliance is confirmed.
- **No speculation rewards:** Market Pulse is read-only. Sunvest does not pay rewards for share or crypto price movements.
- **Privacy:** energy data is personal. Only hashes go on-chain, raw readings stay off-chain, and users control what is shared.
- **AI limits:** the Advisor offers guidance, not electrical engineering sign-off. Safety-related advice always recommends a qualified installer.
- **Fraud resistance:** rewards depend on verified readings, anomaly checks, and daily caps.

---

## Getting Started

### Prerequisites
- Node.js ≥ 20
- pnpm ≥ 9
- Rust stable + Stellar CLI
- Python ≥ 3.11 (for AI workers)
- A funded Stellar Testnet account (use Friendbot or Stellar Laboratory)

### Install
```bash
git clone https://github.com/sunvest-protocol/sunvest.git
cd sunvest
pnpm install
```

### Environment
```bash
cp .env.example .env
# Set STELLAR_NETWORK=testnet, HORIZON_URL, SOROBAN_RPC_URL,
# DATABASE_URL, REDIS_URL, ANTHROPIC_API_KEY
```

### Run in development
```bash
pnpm dev
# or run a single package
pnpm --filter @sunvest/app dev
pnpm --filter @sunvest/gateway dev
```

### Build and test
```bash
pnpm turbo build
pnpm turbo test
cd packages/contracts && cargo test
```

### Deploy contracts to Testnet
```bash
cd packages/contracts
stellar contract build
bash ../../scripts/deploy_testnet.sh
```

---

## Roadmap

| Phase | Focus |
|---|---|
| **1. Foundation** | Wallet login, manual readings, dashboard, send and receive, referrals |
| **2. Proof-of-Sun** | Device connectors, `SunProof`, sponsor-funded `RewardPool` |
| **3. Community** | Sun Feed, chat, installer credentials, AI Energy Advisor |
| **4. Sun Shares pilot** | One or two community solar projects with a licensed partner |
| **5. Growth** | Daily Digest, Market Pulse, more inverter brands, local-currency ramps |
| **6. Mainnet** | Security audit, bug bounty, public launch |

---

## Open Source and Contributing

Sunvest is open source under the MIT license. Good places to start:
- Add a connector for a new inverter brand
- Improve the anti-fraud validation rules
- Translate the app into Hausa, Yoruba, Igbo, and Pidgin
- Write tests for Soroban contracts
- Improve documentation and onboarding guides

```bash
git checkout -b feat/your-feature
pnpm turbo test
pnpm turbo lint
# Open a PR against `main`
```

See `CONTRIBUTING.md` and `docs/architecture.md` for details.

---

## Security

- Smart contracts will be audited before Mainnet; reports will be published in `audits/`.
- Responsible disclosure: security@sunvest.example

---

## License

MIT © 2026 Sunvest Contributors

---

*Built on Stellar · Powered by Soroban · Clean energy, proven and rewarded.*