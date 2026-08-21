# Town Colony Sim — Design Spec

**Status:** DRAFT 2026-08-21 — awaiting gate sign-off ·
**Parent:** [2026-07-02-money-gameloop-skeleton-design.md](2026-07-02-money-gameloop-skeleton-design.md)
(parent: [2026-06-20-economy-sim-design.md](2026-06-20-economy-sim-design.md))
**Shape:** container — four packs, one level deep; the container manifest
(`docs/manifests/`) is written from this spec's Contracts after sign-off. Pack 3
is the wage-payment/hiring body of work CLAUDE.md names as next.
**Branch:** `claude/town-colony-sim-p1s06q`

## Design (decisions — approve before reading on)

Small town v1 = the loops that already run (produce → wages → goods market →
consume, per-metal conserved gold) at genuine town scale, made watchable, plus
the two loops that make people move: the phase-1 labor market, and migration
through the External seam. Four packs in strict risk order, every one
observable at `cargo run`. **Observability first** — `tick()` returns a
`TickReport` of typed `Event`s and the shell narrates each tick, landing on
today's 4-agent world with zero money ops. **Then scale** — `town_world`
(25–35 agents, 10–14 houses, 7–9 businesses over the existing three Goods,
≥2 competing Food sellers, multi-worker slots, several agents seeded
unemployed, spare zero-occupant residences), `employee_of` generalized to
`employees_of`, the conservation totals re-pinned in one deliberate item,
tuning against pinned soak exit criteria. **Then labor** — the `adjust_price`
pattern on `RoleSlot.wage`, `Intent::TakeJob`/`Quit` through the worked
decide→apply template with apply re-checking live headcount so stale intents
die like stale Buys, arrears-driven quitting off the `owed_to` ledger, the
dead-code assign/vacate_workplace commands waking — phase 1's money-op row
stays "none". **Last, the only money-op widening** — migration: destitute
emigrants swept per-metal to External under phase 7's *existing*
transfer→External allowance; immigrants arriving in phase 1 as labor pull
with a capped External-funded gold grubstake (Amendment 16). Everything
decides deterministically — contended passes tie-break on ascending
`AgentId`, immigrant names come from a fixed table + counter, no RNG
anywhere. Immigration halting when External drains or no residence stands
empty is a designed bound, not a bug: the faucet stays closed, worldgen's
seed remains the entire supply, and the audit keeps pinning it. End state:
population visibly moves in both directions while the per-metal audit stays
green over 200-tick soaks.

- **Fits into:** the phase-1 and phase-7 TODO stubs and the `Intent`
  decide→apply template in `src/sim.rs`; `World::businesses()` and the
  pre-built dead-code `assign_workplace`/`vacate_workplace`
  (@src/world.rs:226,241); the `owed_to` arrears ledger as the recorded quit
  driver; External as the rest-of-world purse (node-trade seam untouched);
  `market.rs`'s `Offer`/`plan_purchases`/`adjust_price` shapes, mirrored by
  `JobOffer`/`plan_application`/`adjust_wage`; the shell in
  `src/engine/game_loop.rs`.
- **Invariants:** §8.1 (every new amount is `Money`); §8.2 (grubstake and
  emigration sweep move only via `World::pay` / `Accounts::transfer` — no new
  movers, no grant/take wrappers); §8.3 (per-metal audit unconditionally last
  across every arrival and departure, plus a deliberately-broken-sweep panic
  test); §8.5 (grubstake capped by External's live balance — no overdraft);
  §8.6 (all wage logic in `market.rs`). §8.4 untouched — no mint or burn
  anywhere in this container.
- **Out of scope (YAGNI):** firm founding/closure/ownership (phase 6 — next
  milestone, or open question 1); terrain wiring, agent travel, physical
  goods movement (the `Offer` shelf-to-venue seam stays unexercised); staffed
  Mint business and seigniorage (phase 8 stays inert); non-gold pricing and
  exchange rates (silver/copper stay inert savings — but the emigration sweep
  carries them, no orphan balances); banking/credit/debt; births, deaths,
  aging; housing market, rent, house capacity (House traits stay undesigned —
  v1 vacancy is "zero occupants, hosts no business", nothing more); new Goods
  or production chains; roads/zones/ore/water (`docs/ideas.md` — graduate via
  brainstorming, never straight to code); auction/solver clearing
  (tâtonnement remains the §5 stand-in).

### Amendments to the running series
*(continues 07-02's 1–3, wage spec's 4–6, multi-metal's 7–9, refactor's
10–14. Both amend the 07-02 ⟨REF⟩ sections, which require it: "new phases
require amending this table" — these don't add phases, they widen the tick
signature and one money-op row.)*

15. ⟨REF⟩ tick contract: `pub fn tick(world: &mut World) -> TickReport`.
    Live phases append typed `Event`s to the report; the report is pure
    observation — dropping it changes no state. Audit stays unconditionally
    last, emits nothing, still panics on imbalance. No early-return path.
16. ⟨REF⟩ phase table, row 1 (`labor_market`), money-ops column: "none" →
    "`transfer` External→agent, immigration grubstake only". Everything else
    phase 1 does (matching, wage adjustment, hiring, quitting) stays
    money-free; emigration needs no amendment — its sweep rides phase 7's
    existing `transfer`→External allowance.

### Proposed pack sequence (the container manifest is written from this)

**Pack 1 — Town console.** `Event` + `TickReport` (Amendment 15), emissions
from the live phases (produced, wages paid / payroll short, sold, price
moved, went hungry), shell reworked into header (tick, population,
employed/unemployed, per-metal totals) + per-tick event feed, `roster`
command, name-inspect extended to businesses and to an agent's last 3
events. Lands on the unchanged 4-agent world — zero money ops, zero worldgen
churn. *Observable:* each Enter narrates the tick ("Greenrow Farm paid alice
35g", "bob bought 5 Entertainment @ 2g") instead of a static ledger dump.

**Pack 2 — A real town.** `town_world` ships as the scenario
(`template_world` survives as the test fixture); `employees_of` replaces
`employee_of` (deleted, not kept alongside); produce and pay_wages handle
multiple employees; one deliberate item re-pins the per-metal supply
constants; constants tuned until the pinned soak exit criteria hold.
*Observable:* `cargo run` opens on a genuine town — competing Food sellers'
prices visibly diverge and move, multi-worker payrolls in the feed,
unemployed agents flagged in `roster`.

**Pack 3 — Labor clears.** The phase-1 stub gains the wage market:
`JobOffer` snapshot from `businesses()`/`employees_of`, pure
`plan_application` decide, `Intent::TakeJob`/`Quit` applied with a live
open-headcount re-check, `adjust_wage` tâtonnement write-back effective next
tick, quit-on-arrears reading `owed_to`. No money moves in this pack.
*Observable:* the seeded unemployed get hired within a few ticks ("mira
hired at Greenrow Farm, Labourer @ 38g"), wages drift up on unfilled slots
and down under applicant surplus, a payroll-shorted worker quits and
re-enters the applicant pool; a 50-tick soak reaches near-full employment.

**Pack 4 — The town breathes.** Population varies for the first time:
`Agent.hunger` (single writer: consume), `Intent::Depart` push rule in phase
7 with `World::remove_agent`'s full per-metal sweep to External,
`Intent::Arrive` pull rule in phase 1 with `World::spawn_agent` + the capped
External→agent grubstake (Amendment 16). Conservation proven at every step;
container DONE folds the 🚧 pointer in `docs/INVENTORY.md`. *Observable:*
"petra left town (took 12g 3s 5c)", "Mara arrived seeking work"; the
population count moves in both directions over a 200-tick soak; immigration
visibly stalls when External drains or no residence stands empty.

## Contracts (the hard reference the plan is written from)

### sim::tick  (sim.rs, modified)

`pub fn tick(world: &mut World) -> TickReport`
Runs the fixed 9-phase order unchanged — audit unconditionally last, no
early-return path — while live phases append typed `Event`s. The report is
pure observation: dropping it changes no state; the audit emits nothing and
still panics on per-metal imbalance, never softened to `Result`. Amendment 15
covers the signature change and event-threading.
Refs: @src/sim.rs:39, 07-02 ⟨REF⟩ tick contract (amended here), §8.3.

### Event / TickReport  (sim.rs, new)

`pub enum Event { … }` · `pub struct TickReport { pub events: Vec<Event> }`
Data-only — no behavior on either type. Each pack adds its own variants
(pack 1: produced / wages-paid / payroll-shortfall / sold / price-moved /
went-hungry; pack 3: hired / quit / wage-moved; pack 4: arrived / departed);
the shell matches `Event` exhaustively so a new variant forces the renderer
at compile time. Exact variant fields plan-owned.
Refs: @src/sim.rs (owner) → @src/engine/game_loop.rs (consumer); no §8
invariant in play — read-only observation of phases whose money-op rows are
unchanged.

### World::employees_of  (world.rs, replaces employee_of)

`pub fn employees_of(&self, house: HouseId) -> Vec<AgentId>`
Every agent whose `workplace == house`, in ascending `AgentId` order — the
deterministic-order contract all contended decide passes inherit. Derived by
scan, never stored (07-02 link rule); unknown house yields empty; no error —
it is a query, id validation stays on commands. Read by produce, pay_wages,
and the pack-3 `JobOffer` snapshot. `employee_of` (@src/world.rs:304) is
deleted, not kept alongside.
Refs: @src/world.rs:304, 07-02 ⟨REF⟩ link rule; feeds the phase-2/3 money
paths but is itself §8.2-neutral (reads only).

### town_world  (engine, new)

`pub fn town_world() -> World`
Deterministic and seedless — no RNG, matching `template_world` practice. The
shipped scenario: 25–35 named agents (several deliberately unemployed),
10–14 houses including zero-occupant spare residences, 7–9 businesses over
the existing three Goods with ≥2 Food sellers and `headcount > 1` slots,
inert silver/copper savings per agent (pack-2 D1 precedent), and an External
gold settlement fund sized for pack 4's grubstakes. Its per-metal totals are
the entire money supply, pinned by tests and the audit forever — the faucet
stays closed. Whether worldgen splits into `engine/worldgen.rs` is the
plan's call.
Refs: @src/engine/game_loop.rs (`template_world` precedent), §8.3 (audit
pins the seed), §8.4 (no tick-time mint — worldgen is the only source).

### market::adjust_wage  (market.rs, new)

`pub fn adjust_wage(wage: Money, open_slots: u32, applicants: u32) -> Money`
Pure tâtonnement mirroring `adjust_price`: unfilled openings → raise one
step; surplus applicants with nothing unfilled → lower one step; neither →
unchanged; floor `Money(1)`; step and thresholds are constants alongside
(values plan-owned, integer arithmetic only). Wages stay an explicit
`Metal::Gold` choice per the multi-metal pack-2 decision — single-metal, the
bundle question stays open where 07-12 left it. Write-back per business/role
after matching, effective next tick only — decide never sees this tick's
adjustment.
Refs: @src/market.rs beside `adjust_price` (:48) and its constants, §8.6;
CLAUDE.md roadmap "adjust_price pattern applied to RoleSlot.wage in phase 1".

### market::{JobOffer, Application, plan_application}  (market.rs, new)

`pub struct JobOffer { pub business: AgentId, pub role: Role, pub wage: Money, pub open_slots: u32 }`
`pub struct Application { pub business: AgentId, pub role: Role }`
`pub fn plan_application(offers: &[JobOffer]) -> Option<Application>`
Pure mirror of `Offer`/`plan_purchases`, evaluated for one unemployed agent
against the phase-1 snapshot (`open_slots = headcount −
employees_of().len()`): picks the highest-wage offer with `open_slots > 0`;
ties break ascending business `AgentId`, then `Role` declaration order;
`None` when nothing is open. No role eligibility in v1 —
`Agent.specialization` stays dead code.
Refs: @src/market.rs:16–30 (`Offer`/`Purchase` shapes mirrored), §8.6
(job-search ranking is market logic, so it lives here).

### Intent::{TakeJob, Quit, Arrive, Depart}  (sim.rs, extended)

`TakeJob { agent: AgentId, business: AgentId, role: Role }` ·
`Quit { agent: AgentId }` · `Arrive { name: String, home: HouseId }` ·
`Depart { agent: AgentId }`
Emitted only by pure decide passes over their phase-start snapshot:
TakeJob/Quit by phase 1's labor decide (Quit when `owed_to` arrears exceed N
wage-bills); Arrive by phase 1's pull rule (a slot unfilled K consecutive
ticks + a zero-occupant residence + External covers the stake); Depart by
phase 7's push rule (hunger ≥ H and gold below the cheapest Food offer).
H/K/N and the grubstake size are constants alongside, plan-owned. Apply
re-checks live state, mirroring the goods apply: TakeJob caps against live
open headcount and forwards through `World::assign_workplace` — stale
intents die cleanly; Quit forwards through `vacate_workplace`, the `owed_to`
entry persists (subject to open question 2 for leavers); Arrive =
`spawn_agent` then the capped External→agent gold grubstake via `World::pay`
(arrivals join the applicant pool next tick); Depart = `remove_agent`. A
failed World call drops that intent cleanly — nothing partially applied,
books unchanged on `Err`. Every match on `Intent` stays exhaustive.
Refs: @src/sim.rs:18, the goods_market decide→apply template; §8.2 (all
money via `World::pay` → chokepoint), §8.5 (stake capped by External's live
balance); Amendment 16 grants phase 1's transfer, Depart's sweep rides phase
7's existing row.

### World::remove_agent  (world.rs, new)

`pub fn remove_agent(&mut self, agent: AgentId) -> Result<(), WorldError>`
Given a real agent id — not the reserved Mint/External ids, not a business
id. Validates first, then sweeps every `Metal::ALL` balance to External
through the chokepoint (inert silver and copper included — no orphan
balances survive), vacates home and workplace (derived occupancy simply
updates), and removes the `Agent`. `owed_to` entries naming the leaver are
handled per the gate ruling on open question 2. Atomic at the command layer:
`Err(WorldError::UnknownAgent)` means nothing changed.
Refs: @src/world.rs command layer (07-03 validate-then-forward; `WorldError`
at :138), §8.2 (sweep moves only via the chokepoint), §8.3 (audit green
across every departure — proven additionally by the `#[cfg(test)]`
broken-sweep panic test), phase 7's existing transfer→External allowance.

### World::spawn_agent  (world.rs, new)

`pub fn spawn_agent(&mut self, name: String, home: HouseId) -> Result<AgentId, WorldError>`
Given a residence with zero occupants that hosts no business — v1's entire
vacancy rule. Allocates the next `AgentId` from the agent counter, creates
the agent housed at `home` — unemployed, empty inventory, zero balances.
Moves no money itself (07-03's deliberate refusal of free-money wrappers
stands); the grubstake is a separate, capped External→agent `World::pay` in
the Arrive apply, so a failed stake leaves a penniless-but-valid newcomer,
never a conservation break. `Err` means nothing changed.
Error: `WorldError::UnknownHouse`; a non-vacant or business-bearing house is
rejected (exact variant plan-owned — lift back here if it grows beyond one
case).
Refs: @src/world.rs (`create_business` id-allocation precedent at :258),
§8.2 (spawn is money-free; the stake rides the chokepoint via `pay`).

### Agent.hunger + consume rule  (agent.rs + sim.rs, new field)

`pub hunger: u8` (field on `Agent`)
Single writer — consume only: a tick where Food inventory falls short of one
tick's `consumption_rate` saturating-increments hunger and emits the
went-hungry event; a fully-fed tick resets it to 0. Read by phase 7's Depart
rule and the shell. A deliberately minimal stopgap ahead of the needs model,
not the needs model — subject to the gate ruling on open question 3.
Refs: @src/agent.rs, @src/sim.rs consume (money-op row "none" — hunger is
not money, no §8 invariant in play); housing.rs's "traits designed together
with agent needs" deferral stands.

### Mechanical / obvious (plan owns these — no contract needed)

Event-emission plumbing through the live phases (audit emits nothing); all
shell presentation (header layout, feed rendering, `roster` columns,
business inspect, the last-3-events buffer, any readability pass the
35-agent frame needs); per-pack `Event` variant fields — the exhaustive
shell match is the safety net; `businesses_mut` only if pack 3's write-back
actually needs it (per world.rs's recorded note); all tuning constants
(wage step/thresholds, arrears-quit N, hunger H, vacancy-pull K, grubstake
size) as constants alongside their fns per the `adjust_price` pattern;
vacancy-age tracking for the pull rule (where the K-counter lives — lift
back only if it turns shared or invariant-adjacent); the fixed immigrant
name table + counter; wiring the binary to `town_world` while
`template_world` stays the test fixture; the one-item conservation re-pin in
pack 2 (new pinned per-metal constants, updated snapshots, `money::` /
`market::` output quoted in that item — never scattered); the 50/100/200-tick
headless soak harnesses.

## Migration impact (breaks existing shipped code)

- `src/sim.rs` — `tick` signature change (Amendment 15) touches every caller
  and test that invokes it; phase fns thread the report.
- `src/world.rs` — `employee_of` deleted; its callers (produce's staffed
  filter @src/sim.rs:63, pay_wages @src/sim.rs:91, its own tests) move to
  `employees_of`. Multi-worker semantics land in the same pack so produce and
  payroll scale by staff count rather than assuming one worker.
- `src/engine/game_loop.rs` — shell frame reworked around header + feed
  (pack 1); shipped binary switches to `town_world` (pack 2). Behavior-visible,
  layout plan-owned.
- Conservation pins — the pack-2 scenario swap re-pins per-metal totals in
  one deliberate item; every test asserting the 4-agent world's totals moves
  to fixture (`template_world`) or gets the new constants there.
- `docs/INVENTORY.md` — gains the 🚧 pointer at container start; folding it
  is pack 4's definition of DONE.

## Acceptance tests (written from the contracts above)

- `tick_report_is_pure_observation` — two identical worlds, one report
  dropped: end states equal; audit still last (existing
  `tick_runs_audit_last` extended, not weakened).
- Per-phase emission tests on fixture worlds — produce/pay_wages/goods/
  consume each emit exactly the expected events (pack 1); hired/quit/
  wage-moved (pack 3); arrived/departed (pack 4).
- `employees_of_is_derived_ascending_and_scales_payroll` — order pinned;
  produce and wages correct at headcount > 1.
- `plan_application_prefers_wage_then_ties_break_ascending` — asserted
  iteration-order-free, like the goods decide tests.
- `adjust_wage_{raises_on_vacancy, lowers_on_surplus, holds, floors}` —
  mirror of the `adjust_price` suite.
- `stale_takejob_dies_on_live_headcount` — two hires race one slot; exactly
  one lands, books untouched by the loser.
- `quit_on_arrears_fires_at_N_and_preserves_owed_to`.
- `remove_agent_sweeps_every_metal_no_orphans` — inert silver/copper leave
  with the emigrant; per-metal totals identical before/after.
- `broken_sweep_panics_the_audit` — `#[cfg(test)]` deliberately skips one
  metal; audit must panic naming it (§8.3 proven, not assumed).
- `spawn_is_money_free_and_stake_is_capped` — failed stake ⇒ penniless valid
  newcomer; External never overdrafts (§8.5).
- `immigration_halts_on_drained_external_and_on_zero_vacancy` — the designed
  bounds, asserted as behavior.
- Soaks: 100-tick (pack 2) and 200-tick (pack 4) headless runs — audit green
  every tick, plus the liveness criteria below; 50-tick near-full-employment
  soak (pack 3).

### Pinned soak exit criteria (pack 2 tunes against these, not taste)

Over a 50-tick `town_world` soak: every agent keeps purchasing Food; no
Good's price sits pinned at floor `Money(1)` for the whole soak nor rises
monotonically; at least one price moves in both directions. Constants are
iterated inside the pack until these hold, then frozen.

--- APPROVAL GATE — do not write the plan or any code above this line without sign-off ---

## Open questions

1. **Fifth pack or next milestone for the firm lifecycle?** Founding/closure/
   ownership (`Business.owner`, phase-6 profit draw, `Intent::Found`, the
   closure/layoff/arrears-payoff procedure) is the most dramatic missing
   event class and the most sprawl-prone pack. Recommendation: hold it as
   the immediately-following milestone so every pack here stays bounded —
   but ordering it as Pack 5 of this container is a legitimate call only you
   can make. Blocks nothing in packs 1–4 either way.
2. **Emigrant arrears.** When `remove_agent` runs, `owed_to` entries naming
   the leaver: settle `min(coffer, owed)` into their wallet before the sweep
   and write off the remainder (recommended — economically honest, books
   stay clean), preserve the dangling entry against a removed `AgentId`, or
   write the debt off as a business windfall? Bookkeeping only — the audit
   is blind to `owed_to` either way — but it changes business solvency
   texture and what the arrears ledger means. Blocks pack 4's
   `remove_agent` detail only.
3. **The hunger stopgap.** `Agent.hunger` as a minimal consume-written
   counter front-runs the future needs model (House traits are recorded as
   "designed together with agent needs"). Approve it as deliberately
   throwaway, or veto it — in which case pack 4's push rule falls back to a
   wallet-only destitution signal (gold below the cheapest Food offer for K
   ticks) and hunger waits for a needs spec. Blocks pack 4's Depart rule
   shape only.
