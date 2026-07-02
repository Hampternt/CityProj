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
cargo test           # run all tests (none yet)
cargo test NAME      # run a single test by (sub)name
cargo test -- --nocapture   # show stdout from tests
```

## Current code state vs. target layout

The §7 layout below (`money.rs`, `agent.rs`, `market.rs`, …) is the **target**, not
what exists yet. The repo today is a pre-economy scaffold:

- `src/main.rs` → `engine::game_loop::run()`.
- `src/engine/game_loop.rs` — interactive tick loop: clear screen, render, wait for
  Enter, then each factory produces goods into a `HashMap<String, u64>` storage.
- `src/engine/game_state.rs` — `GameState` (a `Vec<Factory>`), `Factory`, and the
  `Goods` map type, seeded with placeholder factories.

There is **no money, agent, market, or node code yet** — none of the §8 invariants
are enforced because nothing moves money. When you build out the economy, create the
modules from §7 and migrate the `engine/` scaffold into them rather than bolting money
onto the current factory loop. If you change structure, update this section.

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
