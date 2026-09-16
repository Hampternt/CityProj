# CR pack 2 — the recycle runs, and every invalidated artifact re-cut

**Container:** [`2026-09-07-conserved-recycle.md`](2026-09-07-conserved-recycle.md) ·
**Spec:** [`2026-09-07-conserved-recycle-design.md`](../superpowers/specs/2026-09-07-conserved-recycle-design.md)
(signed 2026-09-07) · **Status:** DONE 2026-09-07

## State on arrival

201 tests green at `c11a73e`. Both command-layer wrappers, `levy_amount` and
`tick_with_rate` exist and are wired to nothing; `RECYCLE_PERMILLE` is 0. Phase 7
runs emigration only; phase 8 is an empty stub.

## Decisions

- **Both legs land together, at rate 0 first.** A sink shipped without its matched
  issue is the measured-harmful arm (46 refused demurrage configurations), so it
  must never be a releasable state — hence one commit for both. Rate 0 gives a
  clean bisection handle and is the null the twins gate.
- **The rate-0 twins are cut BEFORE anything is re-cut**, into their own file. If
  one body served both the null and the cured criteria, re-cutting the cured ones
  would reshape the null ones too — laundering exactly the regression A9 exists to
  detect.
  *(Amended 2026-09-08: this pack's file header claimed the copies "buy a baseline
  that cannot drift". They do not — a copied body still reads live production
  constants, and pack 3's founding-headcount change moved this baseline without
  touching the file. What the copies freeze is the CRITERIA; the trajectory is now
  pinned separately.)*
- **Every re-cut carries the measurement that retired the old assertion**, in the
  test's own doc comment, as a table. A criterion is retired on the record or not
  at all.
- **The under-replacement defect is PINNED, not papered over.** The shock soak
  asserts employment lands at 19 (not the pre-shock 21) and that hunger persists,
  so pack 3's founding-template sweep has a baseline and a fix shows up as a
  deliberate re-pin rather than a silent improvement.

## Items

- [x] **1. Both legs wired at `RECYCLE_PERMILLE = 0`.** `sinks` returns the pot,
      `mint_phase` spends it, `tick_with_rate` threads it, `Event::Recycled`, the
      per-tick matched-issue assertion in `tick`, three forced shell `match` arms
      and the derived `net = minted − burned` column.
- [x] **2. `RECYCLE_PERMILLE` frozen at 20**, with the sweep justification, the
      **refuted** share-vs-basket account, and the n=1 note on the constant.
- [x] **3. The rate-0 twins, then the re-cuts.** `src/engine/null_baseline.rs`
      carries all three soaks verbatim with one edit each; then the 100-tick
      soak's `FROM`, the 200-tick soak (converted to shock-driven),
      `mint_phase_creates_no_money`, and `minimal_economy_…`'s supply pin.
- [x] **4. Amendments 20–23**, the 07-19 supersession, the counter
      reinterpretation with its written expiry, and the nine CLAUDE.md
      corrections.
- [x] **5. The `CLOSE_INSOLVENT_TICKS` obligation discharged** with the number.
- [x] **6. The ledger probe** (recorded, not gated).
- [x] **7. The t5000 acceptance soak** carrying A1–A8, A10, and A9 on the twins.

## Ledger

*(append-only; each entry quotes real gate output)*

- **2026-09-07 — pack 2 DONE, all seven items.** 201 → **207 tests**.

  ```
  VERIFY OK — fmt, clippy, build, tests all clean.
  test result: ok. 207 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```

### The cure, measured to t5000

Identical at t500, t1000, t1500 … t5000 — every figure below is the same at all
of them:

| | value |
|---|---|
| population / firms / employed | **30 / 6 / 21** |
| `Sold` events per tick | **82** |
| goods turnover per tick | **912 g** |
| wages per tick | **662 g** |
| units produced per tick | **508** |
| levy pot / per-head share | **984 g / 32 g** |
| agents below one 28 g basket | **0** |
| closures · foundings · departures · quits | **0 · 0 · 0 · 0** |
| hunger events | **68 total, all in t2–t14**, none after |
| max consecutive `insolvent_ticks` | **1** (fuse is 12) |
| max sell-out streak | **1** |
| cheapest Food / Ent / Lux at t5000 | 1 / 2 / 4 (t20: 2 / 2 / 4) |
| External · Mint | **600 · 0**, unmoved from genesis |
| supply · net | **52,148 · 52,148** |

**The limit cycle is real and measured under THIS spec's remainder rule**, not
inherited: the per-agent (gold, Food) vector recurs exactly at **lag 10**, and
differs at lags 1, 2 and 5. It is the same cycle at t2000 and t5000. The spec
declined to inherit the probe arm's bit-identity claim because the probe's
remainder rule was unrecorded; measured here, **L = 10 agrees with it**.

### Item 5 — the standing obligation, discharged

`CLOSE_INSOLVENT_TICKS`'s doc binds any pack that touches coffers. This one
changes demand → revenue → coffers, so the re-measure was owed. Healthy maximum
consecutive `insolvent_ticks` over t1..=t5000 of the cured town: **1**, unmoved
from the pre-cure figure, against a fuse of 12. **Confirmed at 12, not
re-frozen** — and asserted at exactly 1 in the acceptance soak, so a future pack
that eats the margin fails loudly.

### Item 6 — the ledger probe (recorded, not gated)

| | t100 | t500 | t1000 | t1500 | t2000 |
|---|---|---|---|---|---|
| levy pot | 986 | 984 | 984 | 984 | 984 |
| per-head share | 32 | 32 | 32 | 32 | 32 |
| agent-held gold | 49,839 | 49,839 | 49,839 | 49,839 | 49,839 |
| firm-held gold | 1,709 | 1,709 | 1,709 | 1,709 | 1,709 |
| residents with **zero** wage-or-draw income over the last 50 ticks | **9** | **9** | **9** | **9** | **9** |
| largest wallet | 3,186 | 3,006 | 3,006 | 3,006 | 3,006 |
| median wallet | 1,706 | 1,993 | 1,993 | 1,993 | 1,993 |

Two things this records deliberately.

1. **The nine.** Exactly nine residents receive no wage and no draw, ever, and
   live entirely on the dividend — the number the spec predicted from the
   technology (21 slots, 22.5 workers required, 30 residents). **That is what this
   milestone ships**, and the spec's A4 says so: the bottom-tail *stock* gate is
   near-self-fulfilling under a mechanic that credits every wallet every tick, so
   the *flow* number is the honest description and it belongs in the ledger.
2. **A divergence from the spec, as the spec asked for.** The `World::levy`
   contract *derived* the levy-exempt firm share at ≈3.9% (2,028 g = three full
   wage bills) and said "pack 2 item 6's ledger probe records the firm share …
   and a divergence is a finding." Measured: **1,709 g = 3.28%**. Coffers sit
   below three full bills in the cured steady state, so the derivation was close
   but not exact. Nothing depends on the figure — it was context for a decision
   already ruled — but the spec's number should be read as ~3.3%, measured, not
   3.9%, derived.

### The three re-cuts, each with what retired it

**(a) `town_soak_population_moves_both_directions` → `town_survives_and_rebuilds_after_a_forced_closure`.**
Its entire decay-driven premise is gone: six assertions invert and a seventh
*panics*. The full retirement table lives in the new test's doc comment. What
replaces it drives the cured town to t100, force-closes a Food seller, and
asserts the chain still fires — the house frees, founding answers within 20
ticks, someone is hired into the new venue, no further firm dies, nobody leaves,
and Food's price stays inside 4× its pre-shock level throughout the recovery.

Two measured findings came out of writing it, and both are recorded in the test
rather than smoothed away:

- **Founding does NOT take the freed house.** It takes the first fully-vacant
  house in houses order, and the town's two spare residences sort ahead of any
  house a closure frees — measured: founded into HouseId(4) while the victim sat
  vacant at HouseId(6). The retired soak asserted the same house because on the
  *decaying* trajectory the spares were already taken; asserting it here would
  pin an artifact of the old trajectory. The freed house's vacancy is asserted
  separately, so the chain's middle link is still proven.
- **The phoenix venue under-replaces, exactly as the spec predicted.** The Food
  template posts headcount 2 against the dead venue's 4, so after recovery
  employment settles at **19, never back to 21**, and 5–8 hungry agent-ticks fire
  every tick from t118 through t400 and beyond. **Pinned at those values** —
  fixing it (pack 3's sweep) will fail these assertions, which is the point.

**(b) The 100-tick soak's `FROM`, 10 → 15.** Criterion 1 failed at exactly one
window (t10–14, for bram). Under the cure the last agent to enter the Food market
buys first at **t15**, so a rolling 5-tick window opening before then tests
warm-up, not steady state. The arithmetic minimum that passes is 11; 15 is the
principled value. The criterion's *shape* is untouched, and in the cured steady
state the worst gap any agent posts is **2 ticks**. **Criterion 2's price clauses
were re-measured and hold unchanged** — Food moves 2 → 1, so it is neither
floor-pinned all span nor monotonically rising.

**(c) `mint_phase_creates_no_money` → `mint_phase_reissues_exactly_the_pot_and_creates_no_net_money`,
plus a new remainder test.** The old name asserted `total_minted` never moved,
which Amendment 22 retracts. What survives is stronger: the faucet creates no
*net* money. A second test sweeps pot/head combinations (7/3, 29/30, 30/30,
31/30, 1/4) and pins that the flooring remainder goes to the lowest ids and
**never evaporates** — dropping it would deflate the supply by up to 29 g/tick
while still passing the §8.3 audit, the exact class of bug that audit cannot see.
`minimal_economy_…`'s supply pin was likewise re-cut from `total_minted == 105`
to `total_money == 105` plus `minted − burned == 105`, with an added assertion
that the levy actually fires in that fixture (measured: `total_minted` reaches
111 after ten ticks).

### Migration items the spec named, and what measuring them found

- **The two multi-tick founding fixtures in `sim.rs`** — the spec flagged both as
  at-risk because they tick a *funded* world for 12 ticks and so run the cured
  sim. **Neither moved: both still pass unchanged, and no re-cut was made.**
  Stated carefully, because the spec's worry was specific and I did not
  instrument it away: the recycle demonstrably *does* fire inside small
  fixtures (measured on `seeded_minimal_economy`: 6 g burned and re-issued
  over ten ticks), so "the levy never reaches these tests" is NOT the
  explanation. Why `a_demand_death_founds_nothing`'s negative arm survives —
  whether the dividend is too small to clear its shelf, or lands too late in
  its 12-tick window — is **unmeasured**. The assertion holds; the mechanism
  behind it is not something this ledger claims to know.
- **The eight direct `sinks(...)` call sites** in emigration tests now pass rate
  0 explicitly. Those tests are about emigration, not the recycle; passing the
  live constant would have coupled them to a tuning value.

### Deviations from the pack plan

- **A ninth item appeared and is worth naming: `pack_one_ships_the_rate_at_zero_and_no_phase_reads_it`
  was retired in this commit**, exactly as pack 1's ledger said it must be — the
  test asserts `tick_with_rate` at any rate equals `tick`, which is false by
  design the moment phase 7 reads the rate. It was replaced by
  `at_rate_zero_the_recycle_is_wholly_inert` (the half that still means
  something, and A9's arithmetic) plus
  `the_levy_reaches_wallets_but_never_coffers_external_or_the_mint`, which makes
  the narrow id rule observable at phase level.
- **Open question 8 answered with the number it asked for.** The t5000 acceptance
  soak runs in **12.4 s**, taking `verify.sh` from ~2 s to ~15 s. It ships in the
  default `cargo test` as ruled: it is the milestone's central criterion, and a
  criterion that only runs when someone remembers to run it is not a gate.
- **A9 is discharged by the twins**, which is what the ruling intended, and their
  file carries a standing instruction never to "fix" them to match a re-cut cured
  soak — a red twin means the recycle changed behavior at rate 0, which is a bug
  in the mechanic.
