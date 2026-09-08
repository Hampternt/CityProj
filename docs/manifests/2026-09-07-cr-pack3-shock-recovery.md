# CR pack 3 — shock recovery, and the founding-template sweep re-run

**Container:** [`2026-09-07-conserved-recycle.md`](2026-09-07-conserved-recycle.md) ·
**Spec:** [`2026-09-07-conserved-recycle-design.md`](../superpowers/specs/2026-09-07-conserved-recycle-design.md)
(signed 2026-09-07) · **Status:** DONE 2026-09-07 — **closes the container**

## State on arrival

207 tests green at `6e73b47`. The cure runs at `RECYCLE_PERMILLE = 20` and the
town holds 30 residents indefinitely. Pack 2 left one **pinned live defect**: under
a forced closure the phoenix venue under-replaces, so employment settles at 19 and
hunger never stops. Pack 2's shock soak asserts those two values deliberately, so
fixing them must show up as a re-pin.

## Decisions

- **The sweep decides, not the plan.** The spec authorised a re-run of the
  founding-headcount sweep against the corrected criterion, with a stated
  fallback: if no headcount satisfies both the recovery clause and anti-churn,
  formally weaken the recovery clause at the gate with the measurement quoted.
  **The fallback was not needed** — a value satisfies everything.
- **Per-good, not one knob.** The old sweep moved all three goods together, which
  is why it read Entertainment's churn as a verdict on bigger entrants in general.
  Swept per-good, only Food needed to move.
- **The frozen null twins stay green.** The shipped value was chosen partly
  *because* it keeps them passing.
  **CORRECTED 2026-09-08 — this bullet originally said "stay frozen … no twin was
  touched", and that was wrong in substance.** The twins copy test *bodies* but
  read *live* production constants, so raising Food's founding headcount moved the
  pre-cure trajectory they exist to pin **without editing a line of that file**,
  and every criterion kept passing on its slack. Measured, same test, same rate 0,
  only the constant differing:
  headcount 2 → closures `[140, 153, 156, 179, 182, 199]`, Food births `[142]`,
  5 businesses; headcount 4 → closures `[140, 153, 156, 166, 190, 193, 195]`,
  Food births `[142, 169, 194]`, 6 businesses. Passing is not the same as
  unchanged, and conflating them is exactly the laundering that module exists to
  prevent. The trajectory is now pinned outright in the twin, so any future drift
  fails loudly and must be re-pinned with its cause named.

## Items

- [x] **1. The sweep, re-run on the cured town.** Six configurations, each shocked
      at t500 and run to t1200, each also checked against the frozen null twins.
- [x] **2. `FOUNDING_TEMPLATE` re-frozen** — Food 2 → 4; Entertainment and Luxury
      unchanged at 2 — with the full sweep table and the per-good reasoning on the
      constant, and the old comment's mistake named rather than deleted.
- [x] **3. The A11/A12 soak.** Pack 2's shock soak upgraded in place: shock moved
      to t500 per A11, the two defect pins re-pinned to their fixed values, and
      A11's remaining clauses added — the price band, the volume floors after
      recovery, the recovery windows, and the dividend-ordering clause.
- [x] **4. Three downstream re-pins** the template fix forces:
      `founding_template_matches_the_manifest_table`,
      `founding_stakes_self_hires_and_draws_nothing_that_tick`, and pack 2's shock
      soak.
- [x] **5. Spec erratum 1** on A12's reachability bound.
- [x] **6. `docs/INVENTORY.md`'s 🚧 pointer folded** into a real entry, and
      CLAUDE.md updated.

## Ledger

*(append-only; each entry quotes real gate output)*

- **2026-09-07 — pack 3 DONE, all six items. The container closes.** 207 tests
  (unchanged in count: this pack re-pinned rather than added).

  ```
  VERIFY OK — fmt, clippy, build, tests all clean.
  test result: ok. 207 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```

### Item 1 — the sweep, and the finding that made it easy

Cured town, shock at t500 (force-close a Food seller), measured to t1200:

| Food/Ent/Lux | employment t1200 | re-staffed | founded | founded deaths | hunger | null twins |
|---|---|---|---|---|---|---|
| 2 / 2 / 2 *(pack 2)* | 19 | **never** | 1 | 0 | 4,241, never stops | PASS |
| 3 / 3 / 3 | 17 | never | 12 | 8 | 2,547, never stops | FAIL |
| 4 / 4 / 4 | **21** | t571 | 1 | 0 | 6, ends t579 | FAIL |
| 5 / 5 / 5 | 23 | t571 | 26 | 24 | 0 | FAIL |
| 4 / 3 / 3 | **21** | t571 | 1 | 0 | 6, ends t579 | FAIL |
| **4 / 2 / 2** | **21** | **t571** | **1** | **0** | **6, ends t579** | **PASS** ← shipped |

**The churn was Entertainment's, not Food's.** Rows 3 and 5 differ from row 6 only
in Entertainment's and Luxury's headcount, and only they break the anti-churn
criterion — two founded Entertainment firms dying inside one 100-tick window, on
the *pre-cure* trajectory. Raising Food's entrant alone fixes the recovery defect
at **zero cost to any other criterion**.

That is why the spec's fallback went unused. The clause said to weaken the
recovery criterion if nothing satisfied both; something does.

**What the old comment got wrong, named on the constant rather than quietly
replaced:** it treated headcount as a single knob across all three goods, so the
Entertainment churn it measured at 3 looked like a general verdict on bigger
entrants. Per-good it is not. Its *trade* was correctly described; its assumption
that the trade was unavoidable was not.

### Item 3 — A11 and A12, measured then pinned

Shock at t500 on the settled fixed point:

| | tick |
|---|---|
| founding answers | **t570** (lag 70) |
| full re-staffing | **t571** (`STAFFING_LAG` = 71) |
| last hunger event | **t579** (`K` = 79) |

**Two recoveries, pinned separately rather than conflated.** Staffing returns in
71 ticks; the food pipeline takes eight ticks more to refill. Collapsing them into
one number would have hidden the second, so the soak pins both.

Also asserted: exactly one founding, **zero** further closures, **zero**
departures, population never dips, the sixth venue returns, Food's cheapest price
returns inside its pre-shock band and never exceeds 4× it *at any tick of the
recovery*, and every tick after recovery clears A2's volume floor.

**The dividend-ordering clause — CORRECTED 2026-09-08, and the correction
reverses the finding.** This pack put the check inside the shock soak and reported
"zero violations … the check is live". It was neither. The soak also asserts zero
departures, so the clause was **unreachable** and proved nothing; and it read the
wallet captured *before* the tick, whereas phase 7's decide reads it after phases
1–6. Both caught by the consolidation review.

Re-homed in `the_dividend_can_arrive_too_late_to_save_a_leaver`, running the one
fixture that actually emigrates someone — **and the asymmetry FIRES.** At t7 the
leaver holds 0 g, the cheapest Food price is 1 g, and that tick's per-head share
is 1 g: the dividend would have cleared the price exactly, but phase 7 sweeps them
to External against their pre-dividend balance and phase 8 then pays a share to a
roster they are no longer on.

This is the behavioural cost the spec's **open question 1** named and could not
measure. A11's instruction is followed literally — recorded as a known asymmetry
of the burn/mint split against a single-phase transfer expression — and pinned at
its measured values. Scope, so it is not read as larger than it is: **zero
departures occur in the shipped cured town at any horizon**, so this fires in a
fixture, not in the shipped scenario. It is a live defect of the mechanism, not of
the town — and it is new evidence bearing on a ruling the owner has not revisited.

### Item 5 — spec erratum 1

A12 required founding within `FOUND_SIGNAL_TICKS + 2` = **4 ticks**. Measured,
that is wrong by an order of magnitude and wrong in kind: the bound counts only
the sell-out streak and forgets that the surviving seller's price must first climb
off `PRICE_FLOOR` to the viability signal before the scarcity tier can fire at
all. The lag also depends on where in the 10-tick limit cycle the shock lands — a
shock at t100 is answered in **13** ticks, one at t500 in **70**. Since the sim is
deterministic and seedless, A12 ships as an exact pin on the measured tick rather
than a guessed window. The criterion's substance is unweakened.

### What this pack did NOT fix, stated plainly

The under-replacement is fixed for **Food**, which is the good the shock soak
kills and the only one measured to matter. Entertainment and Luxury still found at
headcount 2 against worldgen venues of 3. **Nobody has measured a shock to those
sectors**, and raising them was measured to churn — so the honest position is that
their entrants are *smaller than the venues they would replace and this is
untested*, not that it is fine. A successor shocking an Entertainment or Luxury
seller should expect to repeat this pack's sweep for that good.

The nine residents with no wage or draw income are unchanged and unchangeable
here: that is the technology, not the founding template, and growth is the
successor container's problem.

---

## Consolidation review, 2026-09-08 (post-close)

The three conserved-recycle packs were the only un-code-reviewed work on the
branch — the earlier milestones had five-lens close reviews. A high-effort review
raised **14 findings**; all real ones are applied, and the two that matter are
recorded above at their point of use rather than only here.

**The two that mattered were both false claims of mine, not sim defects** — the
same pattern every close review on this branch has found:

1. **"The frozen null twins stay frozen."** They did not. The twins copy bodies,
   not constants, so pack 3's template change moved the baseline while every
   criterion passed on slack. Reproduced by reverting one constant. The trajectory
   is now pinned outright.
2. **"The dividend-ordering clause is live … zero violations."** It was
   unreachable (the same soak asserts zero departures) and read the wrong balance.
   Re-homed where a departure actually happens — **and it fires**.

Also applied: `levy_amount` now rejects a rate above 1000‰ (it silently truncated
`u64`→`u32` and could overdraw two frames away, panicking on an `.expect` whose
message denied what had happened); three stale `#[allow(dead_code)]` retired; the
shell's recycle line no longer states false arithmetic when the pot doesn't divide
evenly; the closure's vacancy check moved from 700 ticks late to immediately after
the closure; the `FROM` re-pin's true cost measured per criterion (criterion 2
loses nothing, criterion 3 loses 30 `PriceMoved` events — the first draft of that
comment claimed both lost nothing and probing refuted it); the headcount knob's
coupling to the founder capital bar (+51%) documented; a duplicate `cfg(test)`,
a dead rebinding and two stale doc lines cleaned up.

**One finding rejected, with reasoning.** The review flagged that both recycle
legs re-validate ids by linear scan inside `World::levy`/`disburse`, making each
leg O(n²) per tick. That scan **is** the command-layer validation — the §8.2
chokepoint's guarantee that `Err` means nothing changed. Bypassing it for callers
that "know" the id is valid is precisely the widening this design refused when it
made both wrappers households-only. The cost is ~1,800 comparisons per tick in a
30-agent town; the correct fix, if it ever matters, is an id→index map inside
`World`, not a trusted back door.
