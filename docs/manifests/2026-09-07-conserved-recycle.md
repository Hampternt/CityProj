# Conserved Recycle — container manifest

**Spec:** [`docs/superpowers/specs/2026-09-07-conserved-recycle-design.md`](../superpowers/specs/2026-09-07-conserved-recycle-design.md)
(signed 2026-09-07) · **Branch:** `claude/town-colony-sim-p1s06q` ·
**Status:** packs 1–2 DONE 2026-09-07 · pack 3 not started.

## Goal

Stop the town's decline. **This is a STABILITY milestone, explicitly** (spec open
question 7, ruled): every stable configuration measured across ~40,000 simulated
ticks parks at exactly the seeded 30 residents with arrivals 0 and foundings 0
forever, so **a flat population of 30 is the designed outcome, not a growth failure
to be discovered later**. Growth is a different problem and a different container.

The mechanic: each tick, phase 7 burns a per-mille of every living agent's gold
wallet, and phase 8 mints the same pot back out in equal shares to every living
agent. Money-conserving to the coin. At 20‰ the shipped town holds 30 residents,
6 firms and 21 employed with zero closures, zero departures and zero quits, and it
is a **live limit cycle** — 68–82 `Sold` events and ~5% of the money supply
changing hands every tick, identical at t500, t2000 and t5000 — not a rule-frozen
corpse.

## Decisions (from the spec's rulings; do not re-litigate in a pack)

1. **The diagnosis this container acts on is not the one it was commissioned
   under.** CLAUDE.md's standing `target_days`-cap mechanism is **measurably
   false** — 0 of 7,074 agent-ticks had a solvent agent buy nothing because it was
   at cap — and the remedy it implies is an accelerant at every setting. The real
   defect is **bottom-tail solvency on top of a technology fact**: 21 job slots and
   a technology needing 22.5 workers for 30 residents, so nine people receive 0 g
   of wages, ever.
2. **Burn + mint, not a transfer** (open question 1, ruled as recommended, and the
   one ruling flagged as reversible). The levy leg is row 7's pre-authorised
   `burn`; the payout leg is row 8's pre-authorised `mint`. **Amendment 20 is a
   closed, named row-8 purpose grant** — never the open class "matched re-issue of
   a same-tick sink".
3. **Households only, proportional.** The narrow id rule in both wrappers is the
   design, not an optimisation — changing it later is a signature-level change.
   The integer floor is the progressivity: at 20‰ every balance below 50 g levies
   exactly zero and still receives a full share, **by construction rather than by
   a guard**.
4. **`tick_with_rate` exists** (open question 4, ruled). It is the only way A9 can
   be an executable test rather than a claim, and it is the migration path if a
   successor ever turns the constant into policy data.
5. **Phase 6 is not touched.** Turning off the profit draw is the largest single
   t300 lever and worthless by t2000 — it relocates the sink rather than removing
   it. The one named exception is pack 3's founding-template sweep.
6. **The 🚧 pointer goes into `docs/INVENTORY.md` at pack 1** and is folded by
   whichever pack closes the container.

## Packs (one level deep)

| # | Pack | Manifest | Status |
|---|---|---|---|
| 1 | **The wrappers, the helper, and the null seam** — zero behavior change | [`2026-09-07-cr-pack1-wrappers-and-seam.md`](2026-09-07-cr-pack1-wrappers-and-seam.md) | **DONE** 2026-09-07 |
| 2 | **The recycle runs**, and every artifact it invalidates is re-cut in the same commit | [`2026-09-07-cr-pack2-the-recycle-runs.md`](2026-09-07-cr-pack2-the-recycle-runs.md) | **DONE** 2026-09-07 |
| 3 | **Shock recovery, lifecycle reachability, and the founding-template sweep re-run** (independently droppable) | — | not started |

## Open questions

None outstanding — all eight of the spec's were ruled at signing. The standing
*reversibility* note on ruling 2 (open question 1) is carried in the spec's signing
block, not here, so there is one copy of it.

## Ledger

- **2026-09-07 — spec signed, container opened.** The owner's "go ahead with the
  circulation seam spec" taken as the gate signature with the recommendations as
  the rulings (firm-lifecycle precedent). Recorded in the spec's signing block,
  including that the questions were not answered one by one and that ruling 2 is
  the one with a counterweight against it.
- **2026-09-07 — pack 1 DONE.** 191 → 201 tests; `VERIFY OK`. The two command-layer
  wrappers, `levy_amount`, `RECYCLE_PERMILLE` at 0, and `tick_with_rate` land with
  **zero behavior change, proven by a 300-tick event-stream diff against the
  pre-pack commit** (28,237 lines identical, wallets and per-metal totals
  included) rather than by assertion. `Accounts::burn` has a non-test caller for
  the first time. Pack 2 inherits one named obligation: retire the second half of
  `pack_one_ships_the_rate_at_zero_and_no_phase_reads_it` in the commit that wires
  phase 7, since reading the rate makes it false by design.
- **2026-09-07 — pack 2 DONE.** 201 → 207 tests; `VERIFY OK`. Both legs live at
  `RECYCLE_PERMILLE = 20`; Amendments 20–23 executed, including the supersession
  of the 07-19 stub contract and the correction of the audit-pins-the-supply
  misattribution wherever the repo carried it. **The town holds its size
  indefinitely** — measured to t5000: population 30, 6 firms, 21 employed, zero
  closures, zero departures, zero quits, hunger confined to t2–t14, and a live
  limit cycle recurring at lag 10 with 82 `Sold` events and 912 g of turnover
  every tick. Three invalidated acceptance artifacts re-cut, each with the
  measurement that retired it, after their pre-cure forms were frozen as rate-0
  twins. Pack 3 inherits one **pinned live defect**: the phoenix venue
  under-replaces (employment 21 → 19, standing hunger), asserted at its measured
  values so the founding-template sweep shows up as a deliberate re-pin.
