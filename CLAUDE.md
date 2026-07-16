# CLAUDE.md — CityProj

Economy city simulation in Rust (edition 2024). Simulates a noded market economy
from the household level up, with strictly conserved money.

## Read this first

The architecture reference is
[`docs/superpowers/specs/2026-06-20-economy-sim-design.md`](docs/superpowers/specs/2026-06-20-economy-sim-design.md).
Read it before non-trivial work. If code and that doc disagree, stop and reconcile
— do not silently diverge.

## Commands

```sh
cargo run            # run the sim (interactive: Enter advances a tick, q quits)
cargo check          # fast type-check
cargo clippy         # lint
cargo test           # run all tests
cargo test NAME      # run a single test by (sub)name
cargo test -- --nocapture   # show stdout from tests
```

## Current code state vs. target layout

The economy skeleton from
[`docs/superpowers/specs/2026-07-02-money-gameloop-skeleton-design.md`](docs/superpowers/specs/2026-07-02-money-gameloop-skeleton-design.md)
is in place — that spec's ⟨REF⟩ sections are the standing contract for hooking
new mechanics into the loop and money:

- `src/money.rs` — `Money`, `MoneyError`, `Accounts` (the §8 trusted core:
  `transfer`/`mint`/`burn` movers, `audit` panics on imbalance).
- `src/agent.rs`, `src/housing.rs` — `Agent` (person) and `House` data types.
- `src/world.rs` — `World`: agents + houses + accounts; reserves the Mint and
  External account ids; occupancy is derived, never stored.
- `src/sim.rs` — `tick()`: the fixed 9-phase order, audit unconditionally
  last; `goods_market` holds the worked decide→apply template; `Intent` is
  the (empty) enum mechanics extend.
- `src/engine/game_loop.rs` — interactive shell only: Enter advances a tick,
  an agent name inspects it, q quits.

No economic behavior runs yet: phases 1–8 are TODO stubs, `Accounts` starts
empty, and nothing mints — money enters only through earned paths once the
mint job exists. Firms are deferred (TODO markers in `sim.rs`). If you change
structure, update this section.

Next up (specs approved, not yet implemented), in order:

1. [`docs/superpowers/specs/2026-07-03-world-agent-commands-design.md`](docs/superpowers/specs/2026-07-03-world-agent-commands-design.md)
   — a `World` command layer (`pay`, home/workplace assign/vacate) whose
   Contracts section is the source of truth for its plan. Ships single-metal;
   the multi-metal migration pass revises `pay` later (see that spec's
   Migration impact).
2. [`docs/superpowers/specs/2026-07-13-housing-agent-business-refactor-design.md`](docs/superpowers/specs/2026-07-13-housing-agent-business-refactor-design.md)
   — `Business`/`Role`/`RoleSlot` structs and agent specialization fields,
   struct-only, no behavior. Amends 07-03's `pay` to recognize business ids
   (its Amendment 14). Supersedes the historical business-wages-alpha spec
   (never checked in).

Pending approval (no plan until signed off):
[`docs/superpowers/specs/2026-07-12-multi-metal-money-design.md`](docs/superpowers/specs/2026-07-12-multi-metal-money-design.md)
— `Accounts` keyed by `(AgentId, Metal)`; gold/silver/copper independently
conserved. Revises shipped `money.rs` and both specs above at its listed
migration points.

## Hard invariants (never violate)

These come from §8 of the design doc. Breaking one is a bug even if tests pass.

1. **Integer money only.** Money is `Money(u64)` in the smallest unit. Never a float.
2. **Single chokepoint.** Money moves *only* via `transfer` / `mint` / `burn` in
   `money.rs`. Nothing else mutates the `Accounts` store.
3. **Conservation audit every tick.** `total_money() == initial + minted − burned`
   is asserted each tick and panics on imbalance. Never remove or weaken it to make
   something pass.
4. **Mint is the only money creation; burn the only destruction.** Both log to
   `total_minted` / `total_burned`. Gold is held in reserve, not consumed.
5. **No overdraft (v1).** `transfer` returns `Err` rather than allowing a negative
   balance.
6. **Pricing stays in `market.rs`.** Agents and money never embed pricing/wage logic.

## Model at a glance

- **Gold** = finite reserve commodity (input-only in v1). **Mint** = the only faucet
  (gold reserve → money). **Money** = the sole circulating medium.
- **Sinks** prevent inflation: degradation (`burn`) and imports (money → `External`
  account, still tracked, out-of-node). `External` is the seam for future
  node-to-node trade.
- **Agents:** Household, Firm, Mint, External, plus a stubbed node modifier layer
  (government/policy — undesigned, fill in as needed).
- **Two markets**, both adjustment-based and swappable: goods prices and competing
  wages.
- **Discrete ticks**, fixed order: labor clears → produce → wages → goods clear →
  consume → invest → sinks → mint → audit.
- **Nodes:** v1 is one node, but built `create_node()`-ready for many.

## Spec & plan policy

Two artifacts, one review gate between them — do not merge them, but scale the
ceremony to the change:

- **Trivial change** → skip both, just do it.
- **Feature-sized** → one spec doc from
  [`docs/superpowers/specs/_template.md`](docs/superpowers/specs/_template.md):
  a short Design section, a **Contracts** section, then an approval gate. The
  plan (separate file, from `writing-plans`) is written *from* the contracts.
- **Major / multi-subsystem** → separate spec and plan files as usual.

**Contracts are the hard reference.** Write a spec-level I/O contract
(signature + Given/Then + Error + invariant ref) for a unit only when it is
load-bearing: it crosses a module boundary, enforces a §8 invariant, is the
feature's entry point, or could behave more than one way. Leave single-caller
helpers and obvious mechanical code to the plan — and name them as "obvious" so
the plan-writer does not invent interpretations.

**When writing a plan:** treat the spec's Contracts as the source of truth for
names, signatures, and error behavior — copy them verbatim, never rename. The
plan discovers the sub-functions these need and owns *their* signatures. If a
discovered helper turns out to be load-bearing (shared / touches an invariant),
lift it back into the spec and re-approve rather than burying it in one task.

## Conventions

- Keep each module to one clear job (see §7 layout). When a file grows past its
  purpose, split it.
- Follow the existing patterns before introducing new ones.
- Verify before claiming done: `cargo check`, `cargo clippy`, `cargo test` — quote
  real output, not "it compiles."
