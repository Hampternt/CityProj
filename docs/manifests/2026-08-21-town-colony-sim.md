# Town colony sim (container)

**Status:** IN PROGRESS — gate signed 2026-08-21; pack 1 DONE 2026-08-21,
packs 2–4 proposed shape only.
**Branch:** `claude/town-colony-sim-p1s06q` (PR #2)
**Origin:** [2026-08-21-town-colony-sim-design.md](../superpowers/specs/2026-08-21-town-colony-sim-design.md)
(approved 2026-08-21; drafted from a judged three-proposal synthesis, then
adversarially verified against shipped code before the gate — both passes
recorded in the PR).

## Goal

The loops that already run (produce → wages → goods market → consume,
per-metal conserved gold) at genuine town scale, made watchable, plus the two
loops that make people move: the phase-1 labor market and migration through
the External seam. End state at `cargo run`: 25–35 named agents over 10–14
houses and 7–9 businesses, a narrated per-tick event feed under a town
header, the unemployed finding work as wages float, arrears-quitters
re-entering the pool, the destitute emigrating with every metal swept to
External, and newcomers arriving against the settlement fund — population
moving both directions with the audit green over 200-tick soaks.

Not in scope (spec, Out of scope): firm founding/closure (next milestone,
ruled at the gate), terrain wiring and travel, physical goods movement, the
Mint business, non-gold pricing, banking, births/deaths, housing market,
new Goods, everything in `docs/ideas.md`.

## Decisions (fixed by the spec and the gate rulings)

- **Amendments 15–17** to the 07-02 skeleton ride this container: `tick`
  returns `TickReport` (15, lands pack 1); phase 1's money-op row gains the
  immigration grubstake transfer (16, lands pack 4); phase 7's row gains the
  arrears-settlement transfer (17, ACTIVE by gate ruling 2, lands pack 4).
- **Firm lifecycle is the next milestone** (gate ruling 1) — this container
  ends at pack 4.
- **Emigrant arrears settle-then-write-off** (gate ruling 2): `remove_agent`
  settles `min(coffer, owed)` to the leaver before the sweep; the remainder
  is written off.
- **`Agent.hunger` is a throwaway stopgap** (gate ruling 3): single writer
  (consume), replaced wholesale by the future needs model.
- **`World::immigrate`, not `spawn_agent`**: the 3-arg constructor stays
  pinned (07-13); the migration command is a new name wrapping it.
- **`employed_role` lifecycle is contractual**: `assign_workplace` gains the
  role param and writes both fields; `vacate_workplace`/`remove_agent` clear
  both.
- **Determinism without RNG**: contended decides tie-break on ascending
  `AgentId`; role enumeration in `Role::ALL` order; event order = phase
  order then pinned iteration order; immigrant names from a table + counter.
- **Sweep completeness is proven per-account** (no-orphan assertions), never
  by the audit — a conservation-legal orphan is invisible to a totals-only
  check.
- **Soak exit criteria are spec-pinned** — pack 2 tunes until they hold,
  then freezes constants.

## Packs

### Pack 1 — Town console

Item manifest: [2026-08-21-tcs-pack1-town-console.md](2026-08-21-tcs-pack1-town-console.md)
(DONE 2026-08-21; 105 → 114 tests).

`Event` + `TickReport` (Amendment 15 executed), emissions from the live
phases, and the shell reworked into a town console: header, per-tick event
feed, `roster`, inspect extended to businesses and per-agent last-3-events.
Lands on the unchanged 4-agent world — zero money ops, zero worldgen churn.

Observable: on the unchanged 4-agent scenario, each Enter prints a narrated
feed instead of a static ledger dump — "Greenrow Farm produced 40 food",
"Greenrow Farm paid alice 35g", "bob bought 5 entertainment @ 2g" — under a
header showing tick, population, employed count, and per-metal totals;
`roster` lists every agent, and inspecting a name shows its last 3 events.

### Pack 2 — A real town

Proposed shape only (item manifest drafted when pack 1 closes). `town_world`
ships as the scenario (`template_world` survives as the test fixture);
`employees_of` replaces `employee_of`; produce/pay_wages go multi-worker;
one deliberate conservation re-pin item; constants tuned against the spec's
pinned soak exit criteria.

Observable: `cargo run` opens on a genuine town — 25–35 named agents,
competing Food sellers' prices visibly diverging in the feed, multi-worker
payrolls, unemployed agents flagged in `roster`, 100-tick soak green.

### Pack 3 — Labor clears

Proposed shape only. The phase-1 stub gains the wage market:
`JobOffer`/`plan_application`/`adjust_wage` (pure mirrors of the goods
shapes), `Intent::TakeJob`/`Quit` on the decide→apply template with live
headcount re-checks, arrears-driven quitting, the dead-code workplace
commands waking with the role param. No money moves in this pack.

Observable: the seeded unemployed get hired within a few ticks, wages drift
up on unfilled slots and down under applicant surplus, a payroll-shorted
worker quits and re-enters the pool; 50-tick soak reaches near-full
employment with no wage rising monotonically.

### Pack 4 — The town breathes

Proposed shape only. `Agent.hunger` (ruling 3), `Intent::Depart` + phase-7
push with `remove_agent`'s settle-then-sweep (rulings 2, Amendment 17),
`Intent::Arrive` + phase-1 pull with `World::immigrate` + the capped
External grubstake (Amendment 16). Conservation proven per-account at every
step. Container DONE: fold the 🚧 pointer in `docs/INVENTORY.md` and close
this ledger quoting real `verify.sh` output.

Observable: population moves both directions over a 200-tick soak —
"petra left town (took 12g 3s 5c)", "Mara arrived seeking work" — arrivals
take vacant residences and apply next tick; immigration visibly stalls when
External drains or no residence stands empty; audit green throughout.

## Open questions (carried)

None. All three spec questions were ruled at the 2026-08-21 signing (see the
spec's SIGNED paragraph). Upstream questions (reference currency, exchange
rates, seigniorage — 07-12) stay open where they live and block none of this.

## Ledger

- **2026-08-21** — **container drafted from the approved spec.** Baseline
  measured on `main`-equal branch head: `VERIFY OK — fmt, clippy, build,
  tests all clean.` 105 passed. Spec provenance: three-angle proposal panel
  (spatial-first / labor-market-first / town-scale-first) judged under
  repo-fidelity, incrementality, and colony-sim-payoff lenses;
  town-scale-first won. Adversarial verification (three critics vs. shipped
  code) found and fixed pre-gate: the `spawn_agent` name collision (→
  `immigrate`), the unspecified `employed_role` lifecycle, the impossible
  broken-sweep audit-panic test (→ per-account no-orphan assertions), the
  mis-targeted Amendment 15, the deadbeat-employer hire/quit livelock (→
  affordability gate + arrears-employer exclusion), the `HashMap` role-order
  determinism hole (→ `Role::ALL`), under-operationalized soak criteria.
- **2026-08-21** — **gate signed; container moves to PLANNED.** All three
  open questions ruled per the owner's direction (recommendations adopted):
  firm lifecycle → next milestone; emigrant arrears → settle-then-write-off
  (Amendment 17 ACTIVE); hunger stopgap → approved as throwaway. Owner's
  "start pack 1" is the go on pack 1's items — execution authorized for
  pack 1 only.
- **2026-08-21** — **pack 1 closes** (five commits, 06c4ed3..cbbe724):
  Amendment 15 executed, six event variants narrated, town console live
  (header, feed, `roster`, business + last-3-events inspect). 3-lens
  review: zero blockers, three nits applied. Close gate quoted in the pack
  manifest: `VERIFY OK` 114 passed. Next: pack 2 item manifest, on your
  go.
