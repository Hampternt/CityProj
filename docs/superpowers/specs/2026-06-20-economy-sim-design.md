# Economy City Simulation — Architecture Design

**Project:** `CityProj` · **Status:** v1 design (approved 2026-06-20) · **Language:** Rust (edition 2024)

This document is the shared reference for the human and for Claude. It states
*what we are building and why* in plain terms, then the *rules and structure*
Claude must follow. When code and this doc disagree, treat it as a bug in one of
them and reconcile — do not silently diverge.

---

## 1. Goals (plain language)

- Simulate an economy from the **household level up** — individual households and
  firms with their own money and decisions, not just aggregate supply/demand curves.
- **Money is real and conserved.** It circulates; it is never silently created or
  deleted. Every coin comes from a known source and goes to a known destination.
- The world is made of **nodes** — one market/area per node. v1 simulates **one
  node**, but is built so that adding more nodes (`create_node()`) and node-to-node
  trade later requires no rework of the core.
- A **fluent market**: prices and wages form from supply and demand, not by fiat.
- Build incrementally. The government/policy layer is intentionally undesigned —
  it is a hook to fill in as needs arise.

---

## 2. The economic model

### 2.1 Gold, the Mint, and money
Gold and money are **distinct**:

- **Gold** — a finite **backing/reserve commodity**. It is *not* the circulating
  medium. In v1 gold is **input-only** (not consumer-traded) and starts as a
  **fixed reserve held at the Mint**.
- **Money** — the **sole circulating medium**. The only thing agents spend.
- **The Mint** — the **only faucet**. Gold reserve in → money out. Total money
  supply is **capped by the gold reserve** (× a backing ratio). Gold is **held in
  reserve, not consumed** by minting.

> Future extension: a "mine" entity that produces gold; gold becoming a tradeable
> good. Out of scope for v1.

### 2.2 Money sinks (anti-inflation)
A faucet with no drain inflates forever. Money/gold leaves circulation two ways,
both through logged paths so the system stays auditable:

- **Degradation** — slow decay/demurrage that **burns** money out of existence
  (`burn()`). Passive downward pressure balancing the Mint.
- **External trade (imports)** — the node spends money to an **`External` /
  rest-of-world account** in exchange for a resource it cannot produce locally.
  The money **leaves node circulation** but is still **tracked** (it sits in the
  `External` account; it has not vanished). This is the seam that becomes real
  **node-to-node trade** later: today's "outside source" is tomorrow's other node.

**Macro balance we are tuning toward:** `mint inflow ≈ degradation + net imports`,
which keeps a node's *circulating* supply stable instead of inflating without bound.

---

## 3. Money substrate (the trusted core)

Decision: **per-agent balances behind a single transfer chokepoint** (not a
double-entry ledger). Conservation is enforced by funneling every movement
through a tiny, audited set of functions.

- **Integer money only.** `Money(u64)` in the smallest indivisible unit. **No
  floats** — conservation must be exact with zero rounding drift.
- **One central balance store:** `Accounts: HashMap<AgentId, Money>`. Balances do
  **not** live as a field inside each agent struct (that fights the borrow checker
  when moving money between two agents, and makes auditing hard).
- **The only ways money moves:**
  - `transfer(from, to, amount) -> Result<(), Error>` — debit `from`, credit `to`
    by equal amounts. Errors on insufficient funds. **No overdraft in v1.**
  - `mint(to, amount)` — the **only** creation path. Consumes gold-reserve
    headroom; records the event in `total_minted`.
  - `burn(from, amount)` — the **only** deletion path. Records in `total_burned`.
- **Conservation audit, asserted every tick:**
  ```
  total_money() == initial_money + total_minted − total_burned
  ```
  `total_money()` sums all accounts **including** `External` (money there is
  out-of-node but still tracked). If the assertion ever fails, the sim **panics** —
  the invariant is *checked*, not assumed.

These three functions plus the audit are the trusted core. Everything else goes
through them.

---

## 4. Agents (v1)

- **Household** — holds money, supplies labor, consumes goods, can save/invest.
- **Firm** — holds money, hires labor (pays wages), produces goods, posts prices,
  can invest in expanding its own production capacity.
- **Mint** — special agent; converts reserve gold → money.
- **External** — pseudo-agent representing the rest of the world; destination for
  import spending and source of imported resources. Becomes another node later.
- **Node modifier layer** — a stubbed hook on the node for taxes / policy / market
  interventions. **Government lives here, intentionally undesigned.** Add behavior
  as needed without touching the core.

Every agent is identified by an `AgentId`; its money lives in `Accounts`, keyed by
that id.

---

## 5. Markets — price & wage discovery (the hard, iterative part)

Two markets, both with adjustment-based price discovery, both behind one swappable
`Market` interface. This is "market accuracy," so it is a named, central component,
**explicitly the part most likely to change.**

- **Goods market** — each firm posts a price. An **inventory/sales adjustment rule**
  (tâtonnement-style) moves it: sold out ⇒ raise next tick; unsold stock ⇒ lower.
  The market clears at posted prices each tick.
- **Labor market** — **wages float to compete for workers; they are not fixed.**
  Firms bid wages to attract labor: a firm that cannot fill its labor needs **raises**
  its wage next tick; a firm with surplus applicants **lowers** it. Households supply
  labor to the best offers. Wages are a **clearing price for labor**, discovered the
  same way goods prices are.

Both are isolated behind `market.rs` so an auction or true market-clearing solver
can replace the adjustment rule later without touching money or agents.

---

## 6. Simulation time — discrete ticks

A tick is one period (interpret as a day/week — unit is a tuning choice). Order of
operations within a tick matters for bootstrap and is fixed.

### Genesis (before tick 1)
The Mint converts the **initial gold reserve → starting money**, seeded to firms
(and some to households) as starting capital. This solves the chicken-and-egg:
firms need cash to pay wages *before* their first sale.

### Each tick, in order
1. **Labor market clears** — firms post wage offers (adjusted from last tick),
   households accept best offers, hires are matched.
2. **Produce** — firms turn hired labor + inputs into goods.
3. **Wages** — firms `transfer` agreed wages → households.
4. **Goods market clears** — households buy goods; prices adjust; `transfer`
   household → firm.
5. **Consume** — goods consumed/depleted.
6. **Invest** — profitable firms expand capacity / owners take profit.
7. **Sinks** — apply degradation (`burn`) and any external imports
   (`transfer` → `External`).
8. **Mint** (conditional) — create new money from reserve if mint rules trigger.
9. **Audit** — assert `total_money()` balances. Panic if not.

---

## 7. Rust module layout

```
src/
  money.rs    Money type, Accounts store, transfer/mint/burn, total_money() audit
  agent.rs    AgentId, Household, Firm structs
  goods.rs    Good identifiers, inventories
  market.rs   goods + labor price formation and clearing (the swappable part)
  node.rs     Node = agents + market + modifier layer; create_node()
  sim.rs      World state, tick loop + ordering, genesis
  main.rs     wiring, run N ticks
```

- `money.rs` is the trusted core; every money movement in the codebase goes through
  it. Nothing else mutates `Accounts` directly.
- `node.rs` owns the agents and market for one area and exposes `create_node()` so
  the single-node v1 scales to many without restructuring.

---

## 8. Invariants Claude must never violate

1. **Integer money only** — never represent money as a float.
2. **Single chokepoint** — money moves *only* via `transfer` / `mint` / `burn`.
   Nothing else writes to `Accounts`.
3. **Conservation audit runs every tick** and panics on imbalance. Never remove or
   weaken it to "make a test pass."
4. **Mint is the only creation, burn the only destruction**, and both are logged
   into `total_minted` / `total_burned`.
5. **No overdraft in v1** — `transfer` fails rather than allowing a negative balance.
6. **Markets stay behind `market.rs`** — agents and money never embed pricing logic.

---

## 9. Out of scope for v1 (deferred, not forgotten)

- Node-to-node trade and transport intermediaries (the `External` account is the seam).
- A gold mine / gold as a tradeable good.
- A real government/policy model (modifier-layer hook exists; behavior is TBD).
- Auction-based or solver-based market clearing (adjustment rule is the v1 stand-in).
- Banking, credit, and debt (would change the money model — revisit deliberately).

---

## 10. Companion files

- This design doc — the architectural reference.
- `CLAUDE.md` (project root) — operational rules pointing at this doc, restating the
  §8 invariants as hard constraints. Written immediately after this doc.
