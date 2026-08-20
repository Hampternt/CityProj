# CLAUDE.md — CityProj

Economy city simulation in Rust (edition 2024). Simulates a noded market economy
from the household level up, with strictly conserved money.

## Read this first

The architecture reference is
[`docs/superpowers/specs/2026-06-20-economy-sim-design.md`](docs/superpowers/specs/2026-06-20-economy-sim-design.md).
Read it before non-trivial work. If code and that doc disagree, stop and reconcile
— do not silently diverge.

What the sim *is*, at feature altitude, lives in
[`docs/INVENTORY.md`](docs/INVENTORY.md); the plans live in
[`docs/manifests/`](docs/manifests/). This file is structure and invariants.

## Commands

```sh
./scripts/check.sh   # item gate: fmt + clippy (-D warnings) + build — after every item
./scripts/verify.sh  # pack gate: check.sh + the full test suite — the acceptance line
cargo run            # run the sim (interactive: Enter advances a tick, q quits)
cargo check          # fast type-check
cargo test NAME      # run a single test by (sub)name
cargo test -- --nocapture   # show stdout from tests
cargo fmt            # format (fmt sweeps are part of the workflow)
```

The item gate cannot vouch for logic — nothing here has compile-time query
checking, so an item touching money, wages, the market or the tick order runs
that area's tests too (`cargo test money::`, `cargo test market::`).

## Current code state vs. target layout

The economy skeleton from
[`docs/superpowers/specs/2026-07-02-money-gameloop-skeleton-design.md`](docs/superpowers/specs/2026-07-02-money-gameloop-skeleton-design.md)
is in place — that spec's ⟨REF⟩ sections are the standing contract for hooking
new mechanics into the loop and money:

- `src/money.rs` — `Money`, `MoneyError`, `Accounts` (the §8 trusted core:
  `transfer`/`mint`/`burn` movers, `audit` panics on imbalance). Since
  08-17 (multi-metal pack 1) the books are keyed `(AgentId, Metal)` with
  per-metal totals; every mover and reader takes a `Metal`, and `audit`
  checks each `Metal::ALL` entry independently, panicking once naming
  every broken metal.
- `src/metal.rs` — `Metal` (closed coinage-metal enum: gold/silver/copper)
  + hand-written `Metal::ALL` and lowercase `Display`; the orthogonal key
  for every balance and conservation total (07-12 spec).
- `src/agent.rs`, `src/housing.rs` — `Agent` (person) and `House` data types.
- `src/role.rs`, `src/business.rs` — `Role` (closed job-role enum) and
  `Business`/`RoleSlot` (per-role wages, account-only money); phase 3
  (`pay_wages`) reads `employed_role` and `RoleSlot.wage` and keeps the
  per-worker `owed_to` wage-arrears ledger.
- `src/goods.rs` — `Good` (closed consumable enum) + the 07-19 per-good
  constants table (consumption, weight, target days, production).
- `src/market.rs` — `plan_purchases`: pure greedy needs-shopping (§8.6);
  sim.rs builds `Offer`s from `businesses()` and applies via `pay`;
  `adjust_price`: per-business tâtonnement (sold out → raise, didn't
  sell → lower) with its tuning constants alongside.
- `src/world.rs` — `World`: agents + houses + accounts; reserves the Mint and
  External account ids; occupancy is derived, never stored; the 07-03 command
  layer (`pay`, assign/vacate home/workplace) validates ids before forwarding
  to the §8.2 chokepoint; `create_business` allocates account-only business
  ids from the agent counter; `businesses()` is the shared phase query;
  `pay` recognizes business ids (refactor Am. 14) and, since pack 2, names
  its metal (`pay(from, to, metal, amount)`).
- `src/sim.rs` — `tick()`: the fixed 9-phase order, audit unconditionally
  last; `goods_market` holds the worked decide→apply template; `Intent` is
  the enum mechanics extend — `Intent::Buy` is its first variant.
- `src/terrain.rs` — world coordinates (`Point3`, 1 unit = 0.1 m) and the
  triangulated integer heightmap (`Terrain`, `elevation_at`); pure movement
  math (`grade`, `travel_time` + `SpeedProfile`) with its tuning constants
  alongside; deterministic value-noise `generate`; `to_json` for the
  viewer. No in-sim consumer yet — the shell holds a display terrain and
  the `map` command exports `map.json` for `tools/map_viewer.html`
  (self-contained; serve it — `.claude/launch.json`'s `map-viewer` config
  does, on :7431 — since the in-app preview renders a bare `file://` open
  as a scriptless snapshot with no localStorage), which is also a terrain
  playground: in-browser generation parity-pinned to `generate` by the
  `generate_matches_viewer_canary` test, parameter knobs, and named maps
  saved to localStorage (parameters only; not wired into the sim).
- `src/engine/game_loop.rs` — interactive shell (Enter advances a tick, an
  agent name inspects it, `map` exports map.json, q quits) plus
  `template_world`, the worldgen that seeds the 07-19 farm/theater/jeweler
  scenario — in gold, plus inert silver/copper savings per agent (pack 2,
  D1); no per-tick simulation behavior. The money summary prints one line
  per metal and every balance renders as compact `g/s/c` (pack 2, D2/D3).

The 07-19 loops run: phases 2 (produce), 3 (wages from business coffers,
shortfalls carried as `owed_to` arrears and repaid when revenue returns),
4 (goods market via `Intent::Buy`, then per-business `adjust_price`
write-back — new prices take effect next tick), and 5 (consume) have
behavior; phases 1, 6, 7, and 8 are TODO stubs. The tick-time mint
faucet is closed: worldgen's seed is the entire money supply and the
audit pins it there. Worldgen seeds the farm/theater/jeweler scenario.
If you change structure, update this section.

In flight: multi-metal money
([`docs/manifests/2026-08-15-multi-metal-money.md`](docs/manifests/2026-08-15-multi-metal-money.md),
spec
[`docs/superpowers/specs/2026-07-12-multi-metal-money-design.md`](docs/superpowers/specs/2026-07-12-multi-metal-money-design.md)).
Pack 1 (the metal-keyed core) landed 2026-08-17: every call site outside
`money.rs` writes the literal `Metal::Gold`, so runtime behavior is
unchanged and silver/copper are zero by design. Pack 2 — the semantic
migration (`World::pay` gains its metal parameter, worldgen chooses seed
metals, per-metal shell summary) — is next; its manifest is drafted
(`docs/manifests/2026-08-20-mmm-pack2-sim-on-metals.md`, awaiting
approval) and it has no go. Its migration list regenerates via
`grep -rn 'Metal::Gold' --include=*.rs src/ | grep -v '^src/money.rs' | grep -v '^src/metal.rs'`.
After that: a wage-payment/hiring behavior spec built on
`World::businesses()`.

The terrain playground landed on 2026-08-15 —
[`docs/manifests/2026-08-15-terrain-playground-merge.md`](docs/manifests/2026-08-15-terrain-playground-merge.md)
records what was verified in the browser before the merge.

## Roadmap (recorded 2026-07-19; future specs design these)

- **Physical goods movement.** Goods will physically travel: producer →
  market / wherever needed, moved by hauler/logistics-type businesses.
  Agents already have inventories; houses and market venues get their own
  storage; agents travel to a market to buy and back home to stow. Until
  then businesses sell from their own `stock` — the seam where "shelf"
  becomes "market venue inventory" is the `Offer` struct, so don't bake
  in the assumption that stock lives only on businesses.
- **Money creation.** As designed in the parent doc §2.1: a *literal
  staffed Mint business* that consumes precious-metal goods to mint coins
  (seigniorage formula deferred by the 07-12 multi-metal spec), plus
  trade with outside markets through `External`.
- **Wage market.** The `adjust_price` pattern applied to `RoleSlot.wage`
  in phase 1 (can't fill a slot → raise, queue of applicants → lower),
  plus employee happiness / job-switching driven by the arrears ledger.
- **Building volumes.** Buildings will occupy 3D volumes (footprint +
  vertical extent, above or below ground) addressed by `Point3` — never
  tile-locked to the terrain grid, which only stores the ground surface.
- **Ideas parking lot:** [`docs/ideas.md`](docs/ideas.md) — non-binding
  theorycraft for coordinate-system mechanics (ore fields, roads, zones,
  water). Ideas graduate from there via brainstorming, never straight to
  code.

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

- **Trivial change** → skip both, just do it. That is one item's worth of work.
- **Feature-sized** → one spec doc from
  [`docs/superpowers/specs/_template.md`](docs/superpowers/specs/_template.md):
  a short Design section, a **Contracts** section, then an approval gate. The
  plan is written *from* the contracts, as a manifest in `docs/manifests/`.
- **Major / multi-subsystem** → the spec, then a container manifest whose
  packs are one level deep.

**The plan artifact is a manifest** — `docs/manifests/YYYY-MM-DD-<name>.md`,
plan, progress record and history in one file, per the item/pack/container
workflow in the user-global `CLAUDE.md`. Specs keep their own directory and
keep their approval gate; a spec is what a manifest is written *from*.
`docs/superpowers/plans/` is frozen history from the previous `writing-plans`
format — read it for context, never add to it. The general retrofit procedure
and the manifest templates live in
`~/projects/planvisualiser/docs/CONVERTING-A-PROJECT.md`.

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
- Verify before claiming done: `./scripts/check.sh` after an item,
  `./scripts/verify.sh` before a pack closes — quote real output, not "it
  compiles." "It compiles" is an item-gate claim, never a pack-gate one.
- Keep `docs/INVENTORY.md` current: folding a pack's 🚧 pointer into a real
  entry is part of that pack's definition of merged.
