# Money & Game-Loop Skeleton — Design Spec

**Status:** approved in conversation 2026-07-02 · **Parent:**
[2026-06-20-economy-sim-design.md](2026-06-20-economy-sim-design.md)

This spec does double duty: it specifies the skeleton build, and it is the
**standing reference** future mechanics @-point to for *how to hook into the
game loop and money*. Sections marked ⟨REF⟩ are the standing-reference parts.

## Design (decisions — approve before reading on)

Build the game's core skeleton: the trusted money core (`money.rs`), person-agents
and houses with home/work/owner links, and the fixed 9-phase tick loop with the
conservation audit live from tick 0. No economic behavior runs yet — phases are
empty, `Accounts` starts empty, and money will only ever enter later through real
activity (a mint job, external sales), not a genesis handout. The deliverable is
a template world you can step through interactively (`Enter` advances a tick) and
inspect ("agent3 → balance, home"), plus this doc as the plug-in contract for
every future mechanic.

- **Fits into:** new `src/money.rs`, `src/agent.rs`, `src/housing.rs`,
  `src/world.rs`, `src/sim.rs`; `src/engine/game_loop.rs` slimmed to the
  interactive shell; `src/engine/game_state.rs` (placeholder factories) removed.
- **Invariants in play:** all of §8. §8.1 integer money, §8.2 single chokepoint
  (private `balances`), §8.3 audit every tick, §8.4 mint/burn sole
  creation/destruction, §8.5 no overdraft. §8.6 (markets) untouched — no pricing
  exists yet.
- **Out of scope (YAGNI):** firms (TODO marker only — designed later, attached
  to locations), needs logic, markets/pricing, genesis seeding, gold-reserve cap
  on mint, house traits beyond stubs, population growth mechanics (agent count
  is *designed* to vary; nothing varies it yet).

### Amendments to the parent design doc

1. **§7 layout gains `housing.rs`** — houses/living areas are a first-class
   entity: finite per area, carrying traits that will matter for agent needs.
2. **§6 Genesis seeding is deferred** — parent doc mints starting capital at
   genesis; we instead start at zero and let money enter only via earned paths
   (mint job, sales to External). Revisit if bootstrap proves impossible without
   seed capital.
3. **"Household" is reinterpreted** — the economic agent is the *person*; a
   household is the set of agents sharing a house, not a blob-agent.

### ⟨REF⟩ Data model: agents, houses, links

- `Agent { id, name, home: Option<HouseId>, workplace: Option<HouseId> }` —
  a person. Count is dynamic (can grow/shrink later).
- `House { id, address: String, owners: Vec<AgentId> /* traits TODO */ }` —
  a place. Count is finite once an area is defined. Firms will later attach to
  a `HouseId`/location, possibly shared with homes.
- **Link rule — one direction stored, the other derived.** Agent→house (`home`,
  `workplace`) and house→`owners` are stored facts. House→occupants is *always
  derived* by scanning agents (`World::occupants_of`). Never store both
  directions; two stored lists silently desync.
- Balances live **only** in `Accounts`, keyed by `AgentId` — never as a field
  on `Agent` (borrow-checker ergonomics + one-line audit).
- **Reserved ids:** `World::new` reserves two well-known `AgentId`s — **Mint**
  and **External** — as plain accounts (no structs, no behavior yet). They exist
  from day one so phase contracts can name them and ids never get reassigned.

### ⟨REF⟩ Tick phase contract table

`sim::tick` runs exactly this order (§6 of parent doc). Each phase is a plain
`fn(&mut World)`. A new mechanic lands *inside* its phase; new phases require
amending this table.

| # | Phase          | Purpose                              | Money ops allowed            | Future mechanic lands here        |
|---|----------------|--------------------------------------|------------------------------|-----------------------------------|
| 1 | `labor_market` | match hires, adjust wage offers      | none                         | firms + labor market              |
| 2 | `produce`      | labor + inputs → goods               | none                         | firm production                   |
| 3 | `pay_wages`    | firms pay agreed wages               | `transfer` only              | wages (needs firms)               |
| 4 | `goods_market` | agents buy goods, prices adjust      | `transfer` only              | needs-driven purchasing, pricing  |
| 5 | `consume`      | goods consumed toward needs          | none                         | needs fulfillment                 |
| 6 | `invest`       | expand capacity / take profit        | `transfer` only              | firm investment                   |
| 7 | `sinks`        | degradation, imports                 | `burn`, `transfer`→External  | demurrage, external purchases     |
| 8 | `mint_phase`   | new money from reserve               | `mint` only                  | mint job, gold backing cap        |
| 9 | *audit*        | conservation check                   | read-only                    | never gains behavior              |

### ⟨REF⟩ Money entry/exit map

- **In:** `mint` only, logged to `total_minted`. The mint job is the planned
  faucet. Export receipts (selling to External) are a second planned entry
  point — whether they mint new money or draw down External's accumulated
  balance is **decided when external trade is designed**, not here.
- **Out:** `burn` (degradation) and `transfer` to the `External` account
  (imports — out of circulation but still counted by the audit).
- Anything else that "creates" or "destroys" money is a bug by definition (§8.4).

### ⟨REF⟩ How agents act: decide → apply

Every behavior phase uses two passes:

1. **Decide (pure):** iterate agents read-only; each returns `Vec<Intent>` —
   what it *wants* to do (`Intent` is an enum; mechanics add variants). Pure
   functions of the tick-start snapshot: unit-testable, iteration-order-free.
2. **Apply:** execute intents; *this* is the only place `transfer`/`mint`/`burn`
   are called. Unaffordable intents fail cleanly (transfer errs) — wanting is
   unconstrained, paying is not.

The skeleton ships `Intent` as an empty enum plus the decide/apply scaffold in
one phase as the worked TODO-marked template.

## Contracts (the hard reference the plan is written from)

### Unit: Money  (money.rs)

Signature: `pub struct Money(u64)`
Given:  any money amount in the sim
Then:   represented in the smallest indivisible unit; all arithmetic uses checked ops (over/underflow is an explicit error path, never silent wraparound)
Error:  n/a (type-level)
Refs:   @src/money.rs, §8.1 integer money

### Unit: Accounts::transfer  (money.rs)

Signature: `pub fn transfer(&mut self, from: AgentId, to: AgentId, amount: Money) -> Result<(), MoneyError>`
Given:  `from` has balance ≥ `amount`
Then:   `from` debited and `to` credited by exactly `amount`; `total_money()` unchanged
Error:  insufficient funds -> `Err(MoneyError::InsufficientFunds)`, **no partial application** — no state change on error   // §8.5 no overdraft
Edge:   `amount == 0` -> `Ok`, no-op, creates no account entry · `from == to` -> `Ok` if funds suffice, net no-op
Refs:   @src/money.rs, §8.2 single chokepoint, §8.5

### Unit: Accounts::mint  (money.rs)

Signature: `pub fn mint(&mut self, to: AgentId, amount: Money)`
Given:  any
Then:   `to` credited by `amount`; `total_minted += amount` (gold-reserve cap deferred — spec amendment needed when the mint job arrives)
Error:  n/a in v1
Refs:   @src/money.rs, §8.4 mint sole creation

### Unit: Accounts::burn  (money.rs)

Signature: `pub fn burn(&mut self, from: AgentId, amount: Money) -> Result<(), MoneyError>`
Given:  `from` has balance ≥ `amount`
Then:   `from` debited by `amount`; `total_burned += amount`
Error:  insufficient funds -> `Err(MoneyError::InsufficientFunds)`, no partial application   // §8.5
Edge:   `amount == 0` -> `Ok`, no-op (same rule as transfer)
Refs:   @src/money.rs, §8.4 burn sole destruction

### Unit: Accounts::audit  (money.rs)

Signature: `pub fn audit(&self)`
Given:  called at the end of every tick (and permitted anywhere)
Then:   asserts `total_money() == total_minted − total_burned` (initial supply is zero — no genesis); **panics** on imbalance
Error:  panic, by design — never softened to a `Result`   // §8.3
Note:   the subtraction is `checked_sub` — `total_burned > total_minted` is itself an imbalance and panics (burn requires balance, so it is unreachable through the public API)
Test hook: a `#[cfg(test)]`-only balance mutator is the **sanctioned** §8.2 exception, existing solely so tests can force an imbalance and prove the panic fires — never compiled into the sim
Refs:   @src/money.rs, §8.3 conservation audit

### Unit: Accounts read queries  (money.rs)

Signature: `pub fn balance_of(&self, id: AgentId) -> Money` · `pub fn total_money(&self) -> Money`
Given:  any id (unknown id reads as zero — accounts are created implicitly at first credit)
Then:   read-only; `total_money` sums **all** balances including External
Refs:   @src/money.rs, §8.2 (only non-mutating public surface besides the three movers)

### Unit: sim::tick  (sim.rs)

Signature: `pub fn tick(world: &mut World)`
Given:  a world in any post-construction state
Then:   runs the 9 phases in exactly the table's order; `audit()` is unconditionally last — no early return path skips it
Error:  n/a (audit panics on imbalance)
Refs:   @src/sim.rs, §8.3; parent doc §6 tick order

### Unit: World::occupants_of  (world.rs)

Signature: `pub fn occupants_of(&self, house: HouseId) -> Vec<AgentId>`
Given:  any house id
Then:   derived fresh from agents' `home` fields — never from stored state; unknown house yields empty
Refs:   @src/world.rs, link rule (stored vs derived) above

### Mechanical / obvious (plan owns these — no contract needed)

`AgentId`/`HouseId` newtypes and counters, `World::new` and struct constructors,
the phase bodies 1–8 (empty except TODO markers; one hosts the decide/apply
scaffold as the worked template), the `Intent` empty enum, render helpers and
the slimmed `game_loop.rs` shell (clear/render/Enter/q — behavior unchanged
from today), removal of the factory scaffold. All trivial; follow existing
patterns.

## Acceptance tests (written from the contracts above)

The plan turns each into a `#[test]`; names are indicative, Given/Then is binding.

**money.rs**
- `transfer_moves_exact_amount` — mint 100 to A; transfer 30 A→B ⇒ A=70, B=30, `total_money()` unchanged.
- `transfer_insufficient_funds_is_atomic` — A=10; transfer 20 A→B ⇒ `Err(InsufficientFunds)` **and** A=10, B=0 (no partial application).
- `transfer_zero_is_noop` — transfer 0 between unknown ids ⇒ `Ok`, no accounts created, totals unchanged.
- `transfer_to_self` — A=50; transfer 20 A→A ⇒ `Ok`, A=50.
- `mint_credits_and_logs` — mint 100 to A ⇒ A=100, `total_minted`=100, audit passes.
- `burn_debits_and_logs` — A=100; burn 40 ⇒ A=60, `total_burned`=40, audit passes.
- `burn_insufficient_funds_is_atomic` — A=10; burn 20 ⇒ `Err`, A=10, `total_burned`=0.
- `audit_passes_after_op_sequence` — interleaved mint/transfer/burn (incl. failed ops) ⇒ audit passes after every step.
- `audit_panics_on_imbalance` — `#[should_panic]`: corrupt a balance via the `#[cfg(test)]` hook ⇒ audit panics.
- `total_money_includes_external` — mint to A; transfer A→External ⇒ `total_money()` unchanged, audit passes.

**world.rs / housing**
- `occupants_derived_from_agent_homes` — two agents with `home = h1`, one with `h2` ⇒ `occupants_of(h1)` returns exactly the two; changing an agent's `home` is immediately reflected (nothing stored).
- `occupants_of_unknown_house_is_empty`.
- `reserved_ids_exist` — `World::new` ⇒ Mint and External ids are present and distinct.

**sim.rs**
- `tick_runs_audit_last` — corrupt state via the test hook, call `tick` ⇒ panics (proves no path skips the audit).
- `n_ticks_run_clean` — integration: fresh world, 100 ticks ⇒ no panic, `total_money()` still 0 (nothing mints yet).

--- APPROVAL GATE — do not write the plan or any code above this line without sign-off ---

## Open questions

None — genesis (deferred), firms (deferred), and the decide/apply pattern were
resolved in the 2026-07-02 brainstorm.
