# Multi-Metal Money Substrate — Design Spec

**Status:** draft — pending approval · **Parent:** 2026-07-02-money-gameloop-skeleton-design.md (parent: 2026-06-20-economy-sim-design.md)
**Blocks:** 2026-07-12-business-wages-alpha-design.md (on hold until this lands)

## Design (decisions — approve before reading on)

Gold, silver, copper become independently-spendable currencies, not raw material converted to one abstract Money. Revises shipped `money.rs`: `Accounts` keys balances by `(AgentId, Metal)` instead of `AgentId`. `transfer`/`mint`/`burn`/audit become per-metal. Pricing, exchange rates, minting formula NOT decided here — ledger only.

- **Fits into:** new `src/metal.rs`; modifies `src/money.rs` (signatures + all 10 tests); ripples to every existing money call site (see Migration impact).
- **Invariants:** all of §8, per-metal now. §8.1 unchanged (`Money(u64)` stays currency-agnostic). §8.2 same shape, wider key. §8.3 amended: N independent assertions, not one. §8.4 amended: logged per metal. §8.5 unchanged, scoped per metal.
- **Out of scope:** seigniorage-gated minting formula (`mint()` stays bare/uncapped); raw metal as tradeable good (needs `goods.rs`); reference currency / exchange rates (market-layer, deferred); wage spec (stays on hold).

### Amendments to the parent design doc
*(continues 07-02's 1–3, wage spec's 4–6)*

7. §2.1 "Money — sole circulating medium" → three independently-conserved media (gold/silver/copper coin), each its own ledger. `Money(u64)` unchanged; `Metal` is the new orthogonal key. No cross-metal summing in the trusted core.
8. §8.3 audit → N independent assertions, one per `Metal::ALL`. New metal = extend enum + `ALL`; compiler forces every match to update.
9. Gold-reserve cap (deferred since 07-02) now per-metal, still deferred. `mint()` stays uncapped; gating logic is a future spec.

## Contracts (the hard reference the plan is written from)

### Metal  (metal.rs, new)
`pub enum Metal { Gold, Silver, Copper }` · `pub const ALL: [Metal; 3]`
`Copy + Eq + Hash` for HashMap keys. `ALL` = hand-written enumeration — no crate, zero-dep convention.

### Accounts storage  (money.rs, modified)
`balances: HashMap<(AgentId, Metal), Money>` · `total_minted`/`total_burned: HashMap<Metal, Money>`
Unknown pair reads `Money::ZERO` — same implicit-account rule, now 2D key.

### Accounts::{transfer, mint, burn}  (money.rs, modified)
`transfer(&mut self, from: AgentId, to: AgentId, metal: Metal, amount: Money) -> Result<(), MoneyError>`
`mint(&mut self, to: AgentId, metal: Metal, amount: Money)`
`burn(&mut self, from: AgentId, metal: Metal, amount: Money) -> Result<(), MoneyError>`
Same semantics as unmetaled versions (zero/self no-ops, no partial application on error), scoped to one metal. `transfer` never moves value between metals — cross-metal conversion is a future market trade, two separate calls.

### Accounts::audit  (money.rs, modified)
`pub fn audit(&self)` — signature unchanged.
Asserts conservation for every `Metal::ALL` entry independently; panics naming the failing metal. One failure must not mask checking the rest.

### Accounts read queries  (money.rs, modified)
`balance_of(&self, id: AgentId, metal: Metal) -> Money`
`total_money(&self, metal: Metal) -> Money`
`total_minted(&self, metal: Metal) -> Money`
`total_burned(&self, metal: Metal) -> Money`
No cross-metal total by design — summing metals without a market rate is meaningless; signature makes it impossible to write by accident.

### set_balance_for_test  (money.rs, modified, `#[cfg(test)]`)
`pub fn set_balance_for_test(&mut self, id: AgentId, metal: Metal, amount: Money)` — unchanged exception, adds metal.

## Migration impact (breaks existing shipped code)

- `src/money.rs` — all 10 tests gain a `Metal` arg (default `Metal::Gold` unless testing cross-metal isolation).
- `src/world.rs` — `reserved_ids_exist`: 2 `balance_of` calls need a metal.
- `src/sim.rs` — `tick_runs_audit_last`: `set_balance_for_test` call needs a metal.
- `src/engine/game_loop.rs` — `render`'s money summary assumes one global total; needs per-metal display. Behavior-visible, not just a signature fix. Layout left to the plan.
- `src/world.rs` — `World::pay` (07-03, implemented single-metal before this spec): gains a `metal: Metal` param and forwards it to `transfer`; the `WorldError::Money` path carries per-metal `InsufficientFunds` unchanged. Resolves the pay×Metal gap: 07-03 ships against single-metal money as written, and THIS migration pass revises it — decided 2026-07-15.

### Mechanical / obvious (plan owns these — no contract needed)
`Metal` Debug/Display for the shell; iterating `Metal::ALL` in audit/render. New metal later = extend enum + `ALL`, let compiler find call sites.

## Acceptance tests (written from the contracts above)

Existing 10 money.rs tests ported to take a metal (mechanical). New:
- `metals_are_independently_conserved` — mint gold to A, silver to B; transfer/burn on one metal must not touch the other's balance or totals.
- `audit_checks_every_metal` — gold+silver both correct → passes. Corrupt only silver via test hook → panics (proves audit doesn't stop at the first metal).
- `unknown_metal_pair_reads_zero` — untouched `(agent, metal)` reads `Money::ZERO`.

--- APPROVAL GATE — do not write the plan or any code above this line without sign-off ---

## Open questions

- Reference/unit-of-account currency: one designated metal for prices, others trade against it? Blocks `market.rs` and the wage spec, not this one.
- Seigniorage-gated `mint_coin` formula (metal + labor cost vs face value) — own future spec, needs `goods.rs`/`market.rs`.
- `RoleSlot.wage` (from 2026-07-13-housing-agent-business-refactor-design.md, which superseded the old wage spec): single metal or bundle across metals? Decide when unblocked.
