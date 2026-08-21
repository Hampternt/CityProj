# Town colony sim (container)

**Status:** IN PROGRESS — gate signed 2026-08-21; packs 1–3 DONE
2026-08-21, pack 4 proposed shape only.
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

Item manifest: [2026-08-21-tcs-pack2-real-town.md](2026-08-21-tcs-pack2-real-town.md)
(DONE 2026-08-21; 114 → 119 tests; two measured deviations recorded there
and flagged to the owner: 6 businesses not 7–9, 16/14 employment split).

`town_world` ships as the scenario (`template_world` survives as the
cfg(test) fixture); `employees_of` replaces `employee_of`; produce and
pay_wages go multi-worker; the conservation re-pin (gold 60272 / silver
300 / copper 600); constants soak-tuned against the spec's pinned exit
criteria and frozen.

Observable: `cargo run` opens on a genuine town — 30 named agents,
competing sellers' prices leapfrogging in the feed, multi-worker payrolls,
unemployed agents flagged in `roster`, 100-tick soak green.

### Pack 3 — Labor clears

Item manifest: [2026-08-21-tcs-pack3-labor-clears.md](2026-08-21-tcs-pack3-labor-clears.md)
(DONE 2026-08-21; 119 → 136 tests; one measured deviation recorded there
and flagged to the owner: near-full employment frozen at 21/30, the
07-19 demand-composition ceiling — plus three spec Errata, owner-ack via
PR).

The phase-1 stub gained the wage market: `JobOffer`/`plan_application`/
`adjust_wage` + `stepped_wage` (pure mirrors of the goods shapes),
`Intent::TakeJob`/`Quit` on the decide→apply template with live headcount
re-checks, arrears-driven quitting with the deadbeat-employer exclusion,
the workplace commands woken with the role param writing both fields.
No money moves in this pack.

Observable: the seeded unemployed are hired within a few ticks (all open
slots filled by tick 4), wages drift up on unfilled slots and down under
applicant surplus — the cascade visibly leapfrogs venue wages in the
feed; a payroll-shorted worker quits and re-enters the pool (pinned in
tests; the *tuned* town is deliberately quit-free — a quitting town is an
insolvent town); the 50-tick soak reaches and holds the measured full
employment of 21/30 with no wage rising unboundedly.

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
- **2026-08-21** — **pack 2 closes** (six commits, d9008ca..72a4085).
  The soak forced two measured deviations, recorded in the pack ledger
  and flagged to the owner rather than silently absorbed: the market
  mechanics admit at most two solvent sellers per good (vs the spec's
  "7–9 businesses"), and ent/lux staffing at parity bankrupts those
  venues (16/14 employment split, the 14 being pack 3's hiring pool).
  3-lens review: zero blockers; criterion-3 and sampling tightenings
  applied. Close gate quoted in the pack manifest: `VERIFY OK` 119
  passed. Next: pack 3 item manifest, on your go.
- **2026-08-21** — **pack 3 closes** (nine commits, 7bd4e2e..close).
  Phase 1 lives: the wage market's pure layer, `Intent::TakeJob`/`Quit`
  on the template, arrears quitting with the deadbeat exclusion, wage
  tâtonnement with the net-of-arrears affordability gate. The manifest
  was adversarially verified BEFORE implementation (3 lenses — the
  untestable-decision findings became distinguishing tests) and the
  diff reviewed after (3 lenses, zero blockers). One measured deviation
  flagged to the owner: employment frozen at 21/30 — the 07-19 demand
  composition's ceiling, proven by the reverted lever-2 experiment
  (production cuts reach 27 momentarily, then churn and starve lux
  workers; the constraint is demand shares, not production rates). Four
  spec Errata recorded (same-tick payroll reading, `stepped_wage`,
  net-of-arrears affordability, the employment ceiling). The close
  review also measured the equilibrium's ~150-tick dis-saving fuse —
  recorded for pack 4, whose migration is the designed relief. Close
  gate quoted in the pack manifest: `VERIFY OK` 136 passed. Next:
  pack 4 item manifest, on your go.
