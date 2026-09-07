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
  every broken metal. Since conserved-recycle pack 2 `burn` has a live
  caller and has lost its `#[allow(dead_code)]`, and the two §8.4 counters
  are **gross lifetime throughput logs** rather than the supply — the
  number that means "money in this economy" is `minted − burned`, which the
  shell prints as `net`. That reinterpretation carries a written expiry:
  it holds only while every mint is matched by a same-tick burn, and the
  counters split into a pair the moment the §2.1 Mint needs a backing ratio.
- `src/metal.rs` — `Metal` (closed coinage-metal enum: gold/silver/copper)
  + hand-written `Metal::ALL` and lowercase `Display`; the orthogonal key
  for every balance and conservation total (07-12 spec).
- `src/agent.rs`, `src/housing.rs` — `Agent` (person, including the
  pack-4 `hunger` stopgap counter) and `House` data types.
- `src/role.rs`, `src/business.rs` — `Role` (closed job-role enum) and
  `Business`/`RoleSlot` (per-role wages, account-only money); phase 3
  (`pay_wages`) reads `employed_role` and `RoleSlot.wage` and keeps the
  per-worker `owed_to` wage-arrears ledger. Since firm-lifecycle pack 1
  every `Business` names a living `owner` (required, validated at
  creation, strictly distinct from the still-rule-inert `House.owners`);
  since pack 2 the always-living invariant is complete — forced
  liquidation in `remove_agent` means a live business always names a
  live owner, so the pack-1 dangling-owner draw skip is retired. Pack 2
  also added `insolvent_ticks`, the consecutive-arrears fuse phase 6
  liquidates on (single writer: `invest`'s tail write-back); pack 3
  added `sold_out_ticks`, the scarcity-direction streak founding reads
  (single writer: phase 4's price write-back, sharing `adjust_price`'s
  own raise predicate through `market::sold_out`).
- `src/goods.rs` — `Good` (closed consumable enum) + the 07-19 per-good
  constants table (consumption, weight, target days, production).
- `src/market.rs` — `plan_purchases`: pure greedy needs-shopping (§8.6);
  sim.rs builds `Offer`s from `businesses()` and applies via `pay`;
  `adjust_price`: per-business tâtonnement (sold out → raise, didn't
  sell → lower) with its tuning constants alongside. Since pack 3
  (town-colony) the wage market mirrors those shapes: `JobOffer`/
  `plan_application` (highest wage, skip arrears-owing employers, ties
  ascending business id then `Role::ALL` order) and `adjust_wage` +
  `stepped_wage` (raise on unfilled-and-affordable — affordability is
  net of arrears — lower on a stale applicant queue, floor 1). Since
  firm-lifecycle pack 3 a third band holds entry choice: `SellerSnapshot`
  / `Prospectus` / `plan_founding` (two `Good::ALL` passes — existential
  before scarcity — the scarcity tier conjoining a viability price level
  and a sell-out streak) plus the per-good founding template and
  `found_template`.
- `src/world.rs` — `World`: agents + houses + accounts; reserves the Mint and
  External account ids; occupancy is derived, never stored; the 07-03 command
  layer (`pay`, assign/vacate home/workplace) validates ids before forwarding
  to the §8.2 chokepoint — and since conserved-recycle pack 1 **`pay` is no
  longer the only wrapper over that chokepoint**: `levy` (burn) and the
  pot-scoped `disburse` (mint, refusing an over-issue with
  `WorldError::OverIssue`) join it, both taking a **spawned agent only**,
  deliberately narrower than `is_known_account` — households-only is the
  design, and widening it is a signature-level change; `create_business` allocates account-only business
  ids from the agent counter; `businesses()` is the shared phase query;
  `pay` recognizes business ids (refactor Am. 14) and, since pack 2, names
  its metal (`pay(from, to, metal, amount)`); pack 4 added the migration
  commands — `remove_agent` (validate, settle-then-write-off, per-metal
  sweep to External, owners strip, remove) and `immigrate` (vacancy-gated
  wrapper over the untouched `spawn_agent`, bumps the `arrivals` name
  counter); firm-lifecycle pack 1 widened `create_business` with the
  validated `owner` param (checked first — reserved/business/ghost ids
  refuse; the 07-13 "wrap, don't widen" precedent is distinguished in
  that spec: an un-widened path would manufacture ownerless firms);
  pack 2 added `close_business` (settle ascending → clear the ledger →
  lay off → per-metal residual to the OWNER → detach LAST, returning a
  `ClosureReceipt`; detach is last *mechanically* — `is_known_account`
  scans the live `businesses()` set, so detaching revokes the id) and
  the `NoBusinessHere` error, and gave `remove_agent` a step 0 that
  liquidates the leaver's own firms before the A17 settlement, returning
  those receipts (Amendment 19); pack 3 added `found_business` (the
  `immigrate` wrapper precedent for firms — founder checked first, then
  full vacancy, then `create_business`; money-free, so the stake is a
  separate `pay` in the apply) and `is_fully_vacant`, which collects v1's
  entire vacancy rule for its four readers.
- `src/sim.rs` — `tick()`: the fixed 9-phase order, audit unconditionally
  last, returning a `TickReport` of typed `Event`s (pure observation,
  Amendment 15); `goods_market` holds the worked decide→apply template;
  `Intent` is the enum mechanics extend — `Buy`, since pack 3
  `TakeJob`/`Quit` (phase 1's `labor_market`: JobOffer snapshot →
  quits-then-applications decide → live-re-check apply → wage
  write-back; quit fires when arrears exceed `QUIT_ARREARS_BILLS` ×
  slot wage), and since pack 4 `Arrive`/`Depart` (the migration pair:
  phase 1's vacancy pull and phase 7's destitution push). Phase 1's
  only money op is the immigration grubstake (Amendment 16); phase 7's
  are the Amendment-17 settlement and the sweep to External — both
  ride `World::pay`, event amounts measured as balance deltas. Since
  firm-lifecycle pack 1, phase 6 (`invest`) runs the profit draw: a
  direct pay_wages-style pass (no intents) paying each business's gold
  above `draw_amount`'s retained buffer — `DRAW_BUFFER_BILLS` (3,
  frozen) full-staffing bills plus `owed_total()`, net of arrears by
  contract — to its owner, `Event::ProfitDrawn`, under row 6's
  "transfer only" (Amendment 18 touched only that row's purpose text).
  Pack 2 put a closure pass ABOVE it (so a closing firm structurally
  cannot draw) and the `insolvent_ticks` write-back BELOW it, giving one
  tick of designed latency: the counter crosses at tick t's write-back
  and the firm closes at t+1's phase 6, so the effective fuse is
  `CLOSE_INSOLVENT_TICKS + 1` (12, frozen) arrears-ticks. `Event::Closed`
  / `Event::LaidOff` and both closure paths narrate from the
  `ClosureReceipt` via `emit_closure` — never from deltas around the
  command, which cannot attribute flows sharing a wallet. Pack 3 put the
  founding DECIDE at the very top of the phase (so this tick's own
  closure is invisible to it and a refound is a t+1 event) and the Found
  APPLY between the closures and the draws, giving the spec's full
  ordering: decide → closures → founding → draws → counter write-back.
  `Intent::Found` / `Event::Founded`; the founder self-hires, and
  `capital` is read back from the new firm's balance rather than
  assumed.
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
- `src/engine/worldgen.rs` — hand-seeded scenarios (split from game_loop
  in town-colony pack 2): `town_world`, the shipped 30-agent town (12
  houses, 6 multi-worker businesses, 16 seeded employed + open headcount
  for the labor market, soak-tuned frozen constants, per-metal totals
  pinned), and `template_world`, the small `#[cfg(test)]` fixture
  (07-19 farm/theater/jeweler). Since firm-lifecycle pack 1 each venue's
  staff spawn before `create_business` and its first seeded worker is
  its owner-operator (alice/ed/ivan/karl/marco/otto, pinned; per-metal
  totals unchanged by the reorder — gold 52148 / silver 300 / copper
  600). The three town soaks live here — the 100-tick pinned-criteria
  soak (zero quits asserted; since pack 1 also the coffer-at-buffer
  bound from t20 and every-venue-draws; since pack 2 criterion 7's
  zero-closures pin, which needs a hand-written vanish-detector because
  worldgen's `match event` arms end `_ => {}` and force nothing), the
  50-tick employment soak (`NEAR_FULL` = 21 of 30, unchanged under all
  three packs), and the 200-tick migration soak — which pack 3 re-cut and
  raised: its shock-answer criterion now reads over a `Founded` OR an
  `Arrived` (founding answers the shock, and no arrival fires once the
  town's own unemployed take every founded vacancy), and it gained a
  raised firm/population floor, a found→close anti-churn bound, and the
  full closure→vacancy→founding→hire chain. Pack 3 also added a
  zero-`Founded` tally to the 100-tick soak; like pack 2's zero-closures
  pin it is hand-written, because worldgen's `match event` arms end
  `_ => {}` and force nothing.
- `src/engine/game_loop.rs` — interactive shell only (Enter advances a
  tick, `roster` lists agents, a name or business address inspects,
  `map` exports map.json, q quits): town header, aggregated per-tick
  event feed, last-3-events per agent; no per-tick simulation behavior.
  The money summary prints one line per metal and every balance renders
  as compact `g/s/c` (pack 2, D2/D3).

The loops run: phases 1 (labor market — hiring, arrears quits, wage
tâtonnement, and since pack 4 the immigration pull: a `RoleSlot`
vacancy aged `VACANCY_PULL_TICKS`, a vacant residence, and External
covering the `GRUBSTAKE` bring an `Intent::Arrive` — `immigrate` + the
capped External→agent transfer, Amendment 16), 2 (produce, scaled by
staff), 3 (wages from business coffers, shortfalls carried as `owed_to`
arrears and repaid when revenue returns), 4 (goods market via
`Intent::Buy`, then per-business `adjust_price` write-back — new prices
take effect next tick), 5 (consume — also the single writer of
`Agent.hunger`), and 7 (emigration: `Intent::Depart` when hunger ≥
`DEPART_HUNGER_TICKS` and gold below the cheapest posted Food price —
since pack 2 `remove_agent` first force-liquidates any firm the leaver
owns (Amendment 19), then settles `min(coffer, owed)` per Amendment 17,
sweeps every metal to External, and strips the leaver), and 6 (the full firm lifecycle — the founding decide, then closures,
then the founding apply, then the profit draw, then the insolvency
write-back) have behavior. Since conserved-recycle pack 2, **phase 7 also
carries the levy** (a flooring `RECYCLE_PERMILLE` of every living agent's
gold, households only, strictly after emigration so a departing agent is
never levied) and **phase 8 is no longer a stub** — it re-issues that exact
pot in equal shares to every living agent, remainder to the lowest ids, and
`sim::tick` asserts `Δminted == Δburned == pot` per metal on every tick.
Worldgen's seed is still the entire money supply, but **not for the reason
this file used to give**: the §8.3 audit provably cannot detect a mint
(`mint` raises the balance and `total_minted` together), so what pins the
supply is that the re-issue is matched — Amendment 22. Imports and exports
are **refused with measurements** (see below), not pending; the literal
§2.1 Mint is still TODO. The shipped scenario is `town_world`. If you
change structure, update this section.

Multi-metal money is DONE
([`docs/manifests/2026-08-15-multi-metal-money.md`](docs/manifests/2026-08-15-multi-metal-money.md),
spec
[`docs/superpowers/specs/2026-07-12-multi-metal-money-design.md`](docs/superpowers/specs/2026-07-12-multi-metal-money-design.md)):
pack 1 (the metal-keyed core) landed 2026-08-17, pack 2 (the sim runs on
metals) 2026-08-20. Every balance and conservation total is per metal;
gold is the only *trading* metal — every wage and price is an explicit
`Metal::Gold` choice — and worldgen seeds inert silver/copper savings,
until the market layer can price non-gold metals (reference currency and
exchange rates stay open questions there). The wage-payment/hiring work
CLAUDE.md previously named as next landed as town-colony pack 3
([`docs/manifests/2026-08-21-tcs-pack3-labor-clears.md`](docs/manifests/2026-08-21-tcs-pack3-labor-clears.md)).
The **firm lifecycle is DONE**
([`docs/manifests/2026-08-22-firm-lifecycle.md`](docs/manifests/2026-08-22-firm-lifecycle.md),
spec
[`docs/superpowers/specs/2026-08-22-firm-lifecycle-design.md`](docs/superpowers/specs/2026-08-22-firm-lifecycle-design.md),
gate signed 2026-08-22): pack 1 (owners + the phase-6 profit draw)
landed 2026-08-22, pack 2 (closure, forced liquidation, Amendment 19)
and pack 3 (founding) 2026-08-30. The phoenix cycle runs — a venue dies,
its house frees, a capitalized resident founds into the scarcity, the
labor market restaffs it — and founding measurably answers the cascade
closure alone created: over the 200-tick town, **5 live businesses and
population 20 with founding, against 1 and 4 without**.

**Conserved recycle is IN PROGRESS**
([`docs/manifests/2026-09-07-conserved-recycle.md`](docs/manifests/2026-09-07-conserved-recycle.md),
spec
[`docs/superpowers/specs/2026-09-07-conserved-recycle-design.md`](docs/superpowers/specs/2026-09-07-conserved-recycle-design.md),
gate signed 2026-09-07): pack 1 (the two command-layer wrappers,
`levy_amount`, the `tick_with_rate` null seam — zero behavior change,
proven by a 300-tick event-stream diff) and pack 2 (both legs live at
`RECYCLE_PERMILLE = 20`, Amendments 20–23, every invalidated acceptance
artifact re-cut) landed 2026-09-07; pack 3 (shock recovery and the
founding-template sweep) is not started. **The town now holds its size
indefinitely**: measured to t5000, population 30, 6 firms, 21 employed,
zero closures, zero departures, zero quits, hunger confined to a t2–t14
warm-up — and it is a *live* limit cycle, not a frozen one: 82 `Sold`
events, 912 g of goods turnover and 662 g of wages every tick, with the
per-agent state recurring at lag 10 and never at lag 1/2/5. This is a
**stability** milestone, explicitly: a flat 30 is the designed outcome.

**The decline that firm-lifecycle handed forward, and what it actually
was.** On the pre-cure trajectory the town still declined: t300 left 10
residents and 3 businesses, and 12 of the 15 firms founded in 300 ticks
eventually closed. That record stands. **The diagnosis this file used to
carry for it does not** — the conserved-recycle container measured it
(~40,000 simulated ticks, 14 probes) and refuted it. Kept here because it
is the kind of thing a successor re-derives otherwise:

- **REFUTED — the `target_days`-cap mechanism.** The claim was that "a
  wallet above the cap is a sink that never returns". Measured: **0 of
  7,074 agent-ticks** had a solvent agent buy nothing because it was at
  cap, and 0 saturated agents at every sampled tick through t120. Phase 5
  consumes before phase 4 shops, so every agent re-enters below cap every
  tick and buys. The cap governs the *stockpile*, never the flow.
- **REFUTED — the remedy it implies.** `target_days` 7 → 14/30/70/100/1000
  gives t300 population 6/3/4/4/4 against the baseline's 10, and *raises*
  the largest wallet to 35.3%/64.6%/51.0%/99.3% of the 52,148 g supply; at
  ×50 there are zero businesses by t119. The cap is a RATIONING device
  holding concentration down — uncapping it lets rich wallets buy the
  shelves out and the poor cannot eat at any price.
- **REFUTED — "phase 7's demurrage/imports and phase 8's mint are the
  standing TODO stubs that address it", on all three counts as independent
  mechanics.** Demurrage as a *sink*: 46 configurations, not one clears the
  bar; the best buys one resident at t300 and is *worse* at t600, because
  the idle gold IS the savings the jobless live on. Imports: monotonically
  harmful. A minted stipend: works, but with no stable threshold (the
  sufficient value climbs 23 → 27 as the horizon grows to t10000) at 2.43M
  minted against a 52,148 g supply — re-opening exactly the faucet the
  07-19 spec deleted as "the accepted broken faucet". What *does* work is
  the conserved recycle, and phases 7 and 8 are its home **as a matched
  pair**, not as the independent sink and faucet that sentence promised.
- **REFUTED — the aggregate-deficit framing**, which is the trap the next
  reader falls into. Household income averages 772.5 g/tick against 772.3
  g/tick of purchases over t20–t120, and the 1000-tick household ledger
  closes to a **+0 g residual**. Firm revenue *is* household spend by
  construction; there is no aggregate gap for any mechanic to close. The
  gap is distributional.
- **WHAT IT ACTUALLY WAS: bottom-tail solvency on a technology fact.** 30
  residents, 21 job slots, and a technology strictly requiring 22.5 workers
  (0.75 per capita) to feed 30 people — so **nine residents receive 0 g of
  wages across 200 measured ticks**, burn savings at ~28 g/tick, and the
  first-departure tick is *linear in the seed* (`t = 6 + savings/28.05`,
  fitted across nine values). The whole 300-tick decline was that savings
  burn with a closure cascade bolted to its end.

**n = 1, everywhere.** Every figure in that record comes from ONE scenario,
`town_world`, at one wealth distribution. `RECYCLE_PERMILLE`, like
`DRAW_BUFFER_BILLS`, `CLOSE_INSOLVENT_TICKS` and `GRUBSTAKE`, is frozen
against that seed and nothing else; there is no derivation rule for a
second one.

**The successor is growth, not circulation**, and it is named with a
number: nothing measured makes the town *grow* — every stable configuration
parks at exactly the seeded 30 with arrivals 0 and foundings 0 forever.
Growth needs new headcount, from phase 6's untouched expand-capacity half.
On the shipped (cured) trajectory the draw stream is **62–138 g/tick**,
worth 2.2–4.9 livelihoods at the 28 g basket against nine residents with no
income source — so the successor must find roughly two thirds of its wage
bill somewhere other than the existing draws. *(The 221–278 g/tick figure
is the pre-cure trajectory's and must not be quoted as the successor's
sizing.)*

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
  staffed Mint business* that turns a precious-metal reserve into coins
  (seigniorage formula deferred by the 07-12 multi-metal spec). **This line
  used to say "consumes precious-metal goods", which contradicts §2.1 and
  hard invariant 4 below — both hold that gold is *held in reserve, not
  consumed*.** Corrected here; the Mint's own spec rules how the reserve
  and the backing ratio actually work.
  **Outside trade through `External` is NOT part of that roadmap item any
  more:** measured, External's entire capital is 600 g and every export
  setting moves 623–796 g across 300 ticks while starving most of them —
  the seam is *unfunded*, not underpowered — and funding it is lethal,
  because outside demand outbids residents and `adjust_price` has no
  ceiling (Food 179 g by t100, the series diverging to 3.3×10¹⁰/unit).
  Anyone reviving it needs a price ceiling first.
- **Wage market.** *(Shipped as town-colony pack 3 — the `adjust_price`
  pattern on `RoleSlot.wage`, arrears-driven quitting, the deadbeat
  exclusion.)* Still future: employee happiness and voluntary
  job-switching by the employed.
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
- **Agents:** Household, Firm, Mint, External, plus a *planned* node modifier
  layer (government/policy — undesigned; nothing in code yet, so there is no
  stub to find. The parent design doc's §7 layout files it under a `node.rs`
  that has never been created, and its "modifier-layer hook exists" line reads
  as intent, not shipped state).
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

Both of those references — the user-global `CLAUDE.md` and the planvisualiser
doc — live on the owner's machine and are **not reachable from cloud/remote
sessions**, which see only this repo. Working without them, take the shipped
manifests as the templates: [`docs/manifests/2026-08-22-firm-lifecycle.md`](docs/manifests/2026-08-22-firm-lifecycle.md)
for a container (Goal / Decisions / Packs / Open questions / Ledger) and
[`docs/manifests/2026-08-22-fl-pack1-owners-draw.md`](docs/manifests/2026-08-22-fl-pack1-owners-draw.md)
for a pack (Status / State on arrival / Decisions / numbered items with `[ ]`
boxes / append-only Ledger quoting real gate output).

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
