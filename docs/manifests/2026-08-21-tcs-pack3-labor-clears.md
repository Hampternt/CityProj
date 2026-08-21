# Town colony sim — Pack 3: Labor clears

**Status:** IN PROGRESS 2026-08-21 — drafted on the owner's "start pack 3".
**Container:** [2026-08-21-town-colony-sim.md](2026-08-21-town-colony-sim.md)
**Spec contracts executed here:** `market::adjust_wage`,
`market::{JobOffer, Application, plan_application}`, `Intent::{TakeJob, Quit}`
(Arrive/Depart stay pack 4), the `assign_workplace` role-param widening the
spec's Intent contract names; the spec's pack-3 soak criteria (near-full
employment by tick 50, no posted wage rising monotonically) become a test.
No money moves in this pack — phase 1's money-op row stays "none"
(Amendment 16 is pack 4's).

Observable (verbatim from the container): the seeded unemployed get hired
within a few ticks ("mira hired at Greenrow Farm, Labourer @ 38g"), wages
drift up on unfilled slots and down under applicant surplus, a
payroll-shorted worker quits and re-enters the pool; 50-tick soak reaches
near-full employment with no wage rising monotonically.

## State on arrival (measured 2026-08-21)

| What | Measured |
|---|---|
| Test suite | 119 passed, 0 failed (pack-2 close `verify.sh` at a94fdaf) |
| Phase 1 | `labor_market(_world)` TODO stub (sim.rs), takes no report |
| Open slots in `town_world` | **zero** — all 6 businesses seeded staffed to headcount (16 = Σ headcount); the 14 unemployed have nothing to apply to until worldgen opens slots |
| Workplace commands | `assign_workplace(agent, house)` / `vacate_workplace(agent)` dead code (`#[allow]`), callers = own tests; neither touches `employed_role` |
| `Role::ALL` / `Role::Engineer` | `#[allow(dead_code)]` — "no caller until the labor market lands" |
| Structural job ceiling under the frozen 07-19 goods table | **~22 of 30**: Food must run surplus (8 staff × 40 ≥ 300 demand — more farm staff floors prices), ent/lux must run scarcity (supply < demand ⇒ ent staff ≤ 7 at rate 20 vs 150, lux staff ≤ 7 at rate 8 vs 60) — both regimes measured in pack 2, not taste. 8 + 7 + 7 = 22. Near-full (27) is reachable only by cutting ent/lux production rates — a goods-table deviation the spec sanctions when the soak cannot pass without it |

## Decisions (this pack's, within the spec's contracts)

- **Phase 1 mirrors the goods template exactly**: snapshot → pure decide →
  apply with live re-checks → tâtonnement write-back. Snapshot: `JobOffer`s
  from `businesses()` in houses order × `Role::ALL` order (never `HashMap`
  iteration), plus each agent's `owed_by` list (businesses whose `owed_to`
  names them > 0). Decide pass 1 (quits): employed agents in `world.agents`
  order. Decide pass 2 (applications): unemployed agents in `world.agents`
  order — ascending `AgentId` by construction, the contended-pass
  tie-break. A same-tick quitter does NOT also apply — they re-enter the
  pool next tick (spec observable).
- **`open_slots` is counted per role**: `headcount − |employees at the
  house holding that employed_role|`. The spec's parenthetical formula
  (`headcount − employees_of().len()`) is written for v1's single-role
  businesses, where the two are identical on every shipped world; per-role
  is what it means once a business slots two roles. Not a deviation —
  a refinement recorded here.
- **Quit rule**: quit when `owed_to[worker] > N × their slot's live wage`
  (N = `QUIT_ARREARS_BILLS`, first guess 3, soak-tuned). A worker whose
  role isn't slotted accrues nothing and never quits this way; a zero-wage
  slot likewise. N must clear the tuned economy's transient arrears — the
  pack-2 100-tick soak must not fire spurious quits (tuning constraint,
  item 5).
- **`adjust_wage`'s `applicants` param is the stale queue** — applications
  for that (business, role) that did NOT land this tick ("queue of
  applicants → lower", CLAUDE.md roadmap). Post-matching, `open_slots > 0`
  implies the queue is empty (applicants only die when the slot fills), so
  the rule is total and non-overlapping: open > 0 && affordable → raise
  one step; open == 0 && queue > 0 → lower one step (floor `Money(1)`);
  else hold. Step mirrors `adjust_price`: `max(1, wage / 10)`,
  constants alongside. `market::stepped_wage` (small pure helper) is
  exported so the caller can build `affordable` (coffer ≥ one tick's
  full-staffing bill at the stepped wage) without duplicating the step
  formula outside market.rs (§8.6).
- **Write-back timing**: end of phase 1, same as price write-back ends
  phase 4. "Effective next tick" binds the *matching decide* (it only ever
  sees the snapshot); phase 3 payroll reads the live slot wage the way
  produce reads live staffing — so a raise lands in the same tick's
  payroll. `Event::Hired` carries the snapshot wage the agent was hired
  at; the market may move it before payday (display-level, documented).
- **`hire_earns_role_wage_next_pay_wages` pins phase order**: a tick-T
  hire is paid at tick T's `pay_wages` — phase 1 precedes phase 3 inside
  one tick. (The spec's acceptance list says "the following tick" in
  passing; the test name and the Intent contract's rationale — the
  `employed_role` write is what makes payroll see the hire — pin the
  phase-order truth. Recorded, not silently absorbed.)
- **Event variants (plan-owned fields)**:
  `Hired { agent, business, role, wage }`,
  `Quit { agent, business, owed }` (owed = the preserved arrears at
  walkout; distinct enum from `Intent::Quit`, same name),
  `WageMoved { business, role, from, to }` (a held wage emits nothing).
  In-phase order: quits, then hires, then wage moves. Phase order puts
  all three ahead of `Produced` in the feed.
- **Shell**: all three render as individual lines ("mira hired at
  Greenrow Farm as labourer @ 35g", "karl quit The Brass Bell (still
  owed 108g)", "Greenrow Farm raised labourer wages to 38g"); hires
  arrive a few per tick (one business's slots fill per tick — the argmax
  steers the whole pool at one offer), so no aggregation needed.
  `Hired`/`Quit` star the agent in the last-3 history; `WageMoved` stars
  nobody.
- **No `businesses_mut`**: the wage write-back uses the goods pattern
  (collect house ids, `house_mut`) — the spec's conditional stays unmet.
- **Worldgen opens slots without touching seeded staffing**: the
  businesses table gains `seeded_staff` separate from `headcount`
  (16 seeded employed stays exactly pack 2's 4/4/2/2/2/2); headcounts
  rise to the structural ceiling first (farms hold at 4 — Food's mild
  surplus is load-bearing; ent 3+4, lux 3+4 ⇒ 22 jobs). Shelves seed
  `2 × rate × seeded_staff` (not headcount — shelves match the staff that
  actually produce); coffers stay 3 × `wage_bill()` (bills grow with
  headcount — deliberate slack for the wider payroll). Conservation
  re-pin is this item's own deliberate close, quoting `money::`/`market::`
  output (pack-2 precedent).
- **Near-full = ≥ 27 of 30 (90%) at tick 50**, plus ≥ 1 `Hired` by
  tick 3 ("within a few ticks"), plus per-(business, role) the posted
  wage series — sampled in force, before each tick — is never
  (nondecreasing ∧ rose) over the soak (the affordability gate's pin,
  same form as the price criterion). The pack-2 100-tick soak must stay
  green with phase 1 live: tuning satisfies the UNION of both soaks.
- **Authorized tuning levers, in order**: worldgen constants (headcounts,
  seeded wages, wallets, shelves, coffer depth, N); then the 07-19
  goods table (production-rate cuts for ent/lux — spec-sanctioned "only
  as a recorded deviation", 07-19 spec table updated first, its pinned
  test after). The arrival arithmetic says 27 needs the second lever
  (e.g. ent 20→12, lux 8→5 ⇒ 8+10+10 = 28 jobs). If even that cannot
  hold the union, freeze at the measured maximum and record the
  deviation, flagged to the owner (pack-2 precedent).

## Items

- [ ] **1. Workplace commands learn roles.**
  `assign_workplace(agent, house, role)` writes `workplace` and
  `employed_role` together (the agent.rs reserved extension);
  `vacate_workplace` clears both. Existing tests updated to the new
  signature; new pins: assign writes both, re-assign moves both, vacate
  clears both, `Err` changes neither. `#[allow(dead_code)]` stays until
  item 3 wires the phase (recorded so review doesn't flag it). Done:
  `./scripts/check.sh` clean; `cargo test world::` green. Touches:
  src/world.rs, src/agent.rs (doc note).
- [ ] **2. The pure wage market.** `JobOffer`/`Application`/
  `plan_application` (highest wage with `open_slots > 0`, skipping
  `owed_by`; ties ascending business id then `Role::ALL` order; `None`
  when nothing open — asserted iteration-order-free) and `adjust_wage`
  (+ step/floor constants, `stepped_wage` helper) with the full
  `adjust_price`-mirror test suite plus the affordability gate
  (`raises_on_vacancy` only when affordable) and the owed_by exclusion.
  Lands in ONE COMMIT with item 3 — an unconsumed pure layer is dead
  code under the clippy gate (third use of the recorded forcing).
  Touches: src/market.rs.
- [ ] **3. Phase 1 wakes.** `Intent::{TakeJob, Quit}`;
  `Event::{Hired, Quit, WageMoved}`; `labor_market(world, report)` per
  the decisions above; apply re-checks live state (agent still
  unemployed; live staff-in-role < headcount) and forwards through the
  widened commands — a failed command drops the intent cleanly; wage
  write-back last; shell renders and stars the new events; dead-code
  allows drop (workplace commands, `Role::ALL`). Tests: emission tests,
  `stale_takejob_dies_on_live_headcount`,
  `hire_earns_role_wage_next_pay_wages`,
  `quit_on_arrears_fires_at_N_preserves_owed_to_and_clears_role`
  (+ `plan_application` never returns the quitter to a business still in
  their `owed_by`), write-back-affects-next-decide-only, unaffordable
  raise held. Done: `./scripts/check.sh` clean; `cargo test sim::`,
  `market::`, `money::`, `world::` output quoted in the ledger. Touches:
  src/sim.rs, src/engine/game_loop.rs, src/world.rs, src/role.rs.
- [ ] **4. Worldgen opens the slots (+ re-pin).** `seeded_staff` joins
  the businesses table (staffing unchanged at 16); headcounts rise to
  the no-goods-change ceiling (farms 4+4, ent 3+4, lux 3+4 ⇒ 22 jobs,
  6 open at boot); shelves re-seed by `seeded_staff`; shape test updates
  (staff == seeded_staff per business, headcount ≥ staff, Σ open
  pinned); the conservation re-pin in the same deliberate item, new
  per-metal constants + `money::`/`market::` output quoted. Done:
  `./scripts/check.sh` clean; quoted gates. Touches:
  src/engine/worldgen.rs.
- [ ] **5. The 50-tick employment soak + tuning.**
  `town_soak_reaches_near_full_employment`: ≥ 1 `Hired` by tick 3;
  employed ≥ 27 at tick 50; no posted wage (per business/role, sampled
  in force) rises monotonically. The pack-2 100-tick soak stays green
  with hiring live — the union is the gate. Levers in the decided order;
  goods-table changes only as recorded deviations (07-19 spec first);
  measured shortfall → freeze at maximum, record, flag. Constants
  frozen; the saga traced in the ledger. Done: both soaks green in
  `cargo test`; final constants + deviations recorded. Touches:
  src/engine/worldgen.rs, possibly src/goods.rs +
  docs/superpowers/specs/2026-07-19-*.
- [ ] **6. Pack close.** `./scripts/verify.sh` green; 3-lens review
  (spec-contract fidelity / §8 invariants / economy quality); ledger
  quotes real output and the new count; container entry updated;
  CLAUDE.md code-state section (phase 1 no longer a stub); PR body
  updated. Done: `VERIFY OK` quoted with count. Touches:
  docs/manifests/*, CLAUDE.md.

## Not in this pack unless you say so

- No job-switching by the employed — quit-on-arrears is the only exit;
  wage-chasing waits for the roadmap's happiness mechanic.
- No money ops in phase 1 — the grubstake transfer is pack 4's
  (Amendment 16); nothing here touches `pay`/`transfer`.
- No `Arrive`/`Depart`, no `immigrate`/`remove_agent`, no
  `Agent.hunger` — pack 4.
- No new `Role` variants; `Agent.specialization` stays dead code (spec:
  no role eligibility in v1).
- A departed worker's arrears stay on `owed_to` unpaid — settlement
  exists only at emigration (Amendment 17, pack 4). The `owed_by`
  exclusion is what keeps that from becoming a livelock.
- No INVENTORY body rewrite — the 🚧 pointer stands until container DONE.

## Ledger

- **2026-08-21** — **drafted and started.** Go given ("start pack 3").
  Items 1–6 as above; baseline re-measured on arrival at a94fdaf:
  `VERIFY OK — fmt, clippy, build, tests all clean.` 119 passed, 0 failed.
