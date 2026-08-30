# Firm lifecycle — Pack 3: Firms are born

**Status:** IN PROGRESS — drafted 2026-08-30 from the signed spec's
Contracts, a 260-tick gate-reachability probe on the pack-2 trajectory,
and a judged four-angle design panel.
**Container:** [2026-08-22-firm-lifecycle.md](2026-08-22-firm-lifecycle.md)
**Spec contracts executed here:** `market::plan_founding` (+
`SellerSnapshot`, `Prospectus`, `FOUND_SIGNAL` and the founding template),
`Intent::Found`, `World::found_business`, `Event::Founded`. This pack
closes the container: CLAUDE.md structure update + the INVENTORY fold.

Observable (verbatim from the container): after a venue dies, "mira
founded a food stall at 5 Weir Cottage (staked 420g)", then Hired events
restaffing it; zero foundings on the tuned equilibrium town.

## The measurement that reshaped this pack

Before writing a line, a probe scored the signed founding gates against
pack 2's measured 260-tick trajectory. **Founding as signed fires once
usefully in 260 ticks**, blocked three independent ways — any one fatal:

1. **The Entertainment and Luxury scarcity gates are structurally
   unreachable.** Not the failure the spec anticipated. Post-closure
   prices *rise* correctly; the problem is upstream. The duopoly price war
   drives BOTH sellers of a good to `PRICE_FLOOR` *before* either dies
   (Entertainment floored t137 against a first closure at t153; Luxury
   floored t159 against t171). The survivor therefore begins its scarcity
   ratchet **at the floor**, where the proportional step degenerates to
   +1/tick — while its own 12-tick insolvency fuse is already burning.
   Gilt Curtain lived 3 ticks alone and reached price 3 against a signal
   of 5; Silverthread lived 1 tick at price 1 against a signal of 8.
   Neither gate fires once in 260 ticks.
2. **`Good::ALL` order + one-founding-per-tick starves dead sectors.**
   Entertainment holds ZERO sellers from t156 and qualifies on the
   unconditional branch from t157 — but Food also qualifies t143–t181 and
   is scanned first, so Entertainment is never selected for 25 consecutive
   ticks. Luxury holds zero sellers from t172 and is selected **zero**
   times in the remaining 89 ticks. A persistently-qualifying good
   permanently locks out every good behind it, inverting the contract's
   own stated priority that "a dead sector must be recoverable".
3. **The founder pool empties exactly when the gate opens.** From t166 to
   t200 — 35 consecutive ticks with a satisfied gate and 5–10 vacant
   houses — not one unemployed agent holds even 120 gold. The unemployed
   dis-save and emigrate, while the phase-6 draw funnels the monopolist's
   entire scarcity rent into ONE *employed* owner's wallet: alice ends
   holding **51,706 of 52,148 gold — 99.15% of the supply** — and the
   unemployed-only eligibility rule makes exactly that wallet ineligible.

Two further measured facts the plan is built on:

- **All six pack-2 closures carried zero gold proceeds.** Every venue died
  with an empty coffer, so `close_business`'s residual sweep never funded
  a founder. Owner capital comes entirely from draws taken *before* the
  arrears began — which is why the laid-off-owner route works **3 times
  in 6**: a firm that dies slowly has already stopped drawing and already
  lost its owner to quitting and dis-saving.
- **Premises are never the binding constraint** (3–5 vacant houses at
  every tick an eligible founder exists), and immigration never once
  competed with a founder — though both use the same lowest-id
  fully-vacant predicate, so the collision is real in a denser town.

## Decisions (this pack's, within the spec's contracts)

### D1 — Two-tier scan: existential before scarcity (spec **Erratum A**)

`plan_founding` scans `Good::ALL` **twice**, returning the first hit:

- **Tier 1 (existential):** first good with `sellers == 0` → prospectus at
  the template's `FOUND_PRICE`.
- **Tier 2 (scarcity):** first good with `sellers == 1` AND the scarcity
  gate below → prospectus at the survivor's live cheapest price.

The signed `sellers < 2` carrying-capacity gate is preserved *exactly* as
the union of the two tiers; both structs, the signature, the builder
invariant, the price rule and the `None` semantics are untouched. Each
tier is still a `Good::ALL` scan, so the result stays pure, total and
iteration-order-free. This makes the code match the contract's own prose
("a dead sector must be recoverable"), which blocker 2 inverted.

Rejected as larger divergences: more than one founding per tick (discards
the one-arrival precedent's legible drain) and round-robin selection (adds
cross-tick state to a pure fn).

### D2 — The scarcity gate is a level AND a direction test (spec **Erratum B**)

A price *level* alone cannot carry this discriminator, because at the
floor a live sector and a dead one post the same number — Entertainment
sat at 1 while raising every tick, Luxury sat at 1 for 13 flat ticks —
and a *collapsing* sector's price passes back down through any threshold
(Food fell 26 → 1 across t163–t184, satisfying a level-only gate
throughout its own collapse). So:

```
sellers == 1 && cheapest_price >= FOUND_SIGNAL(good)
             && sold_out_streak >= FOUND_SIGNAL_TICKS
```

`SellerSnapshot` gains `sold_out_streak: u32` (the max over the good's
live sellers, so `sellers == 0` implies 0), fed by a new
`Business.sold_out_ticks` whose **single writer is phase 4's existing
price write-back** — which already holds the offered stock and the units
sold. Increment on `adjust_price`'s own raise predicate reused verbatim,
reset to 0 on a lower, and **hold** on `offered == 0` (`adjust_price`'s
documented "no signal").

### D3 — `FOUND_SIGNAL` is derived, not tuned: 2 / 2 / 3

`FOUND_SIGNAL(good) = max(PRICE_FLOOR + 1, ceil(wage / production_rate))`
— the price at which one staffer's full output covers his own wage. Food
`ceil(35/40) = 1` → 2; Entertainment `ceil(36/20) = 2`; Luxury
`ceil(24/8) = 3`. The signed provisional 4/5/8 (≈2× seed prices) was
sized against scarcity; this is sized against **viability**, which is what
the spec's own anti-churn note asks for.

A consequence worth stating rather than hiding: **Luxury's scarcity gate
is correctly unreachable.** A sector whose survivor cannot post a
wage-covering price has no room for an entrant; Luxury is recoverable
only through tier 1.

### D4 — Founded headcount 2, wages and prices unchanged

Headcount **2** for all three goods (signed provisional 4/3/3), single
`Labourer` slot — the spec's own named lever ("founding smaller than a
worldgen venue is legitimate"), so no erratum. Food's seeded supply
already mildly over-served demand (8 staff × 40 = 320 against 30 × 10 =
300 appetite), which is what floored both prices in the first place; an
entrant at the signed headcount would recreate the price war founding
exists to cure. The self-hire fills one seat and leaves **exactly one
open**, which keeps the container's fourth leg — the full-cycle chain's
`Hired` — satisfiable.

`FOUND_PRICE` 2/2/4, wages 35/36/24, `FOUND_CAPITAL_BILLS` 3 and
`FOUNDER_RESERVE` 200 are **unchanged signed values**. `FOUND_CAPITAL_BILLS`
is structurally forced rather than tuned: a firm founded at
`capital = wage_bill × 3` with zero arrears holds `coffer == buffer`
exactly, so `draw_amount` returns zero and the stake cannot round-trip to
the founder inside the same phase 6.

### D5 — `FOUND_SIGNAL_TICKS = 2`, provisional, with a named freeze

The one constant that **cannot be frozen from existing data**: the probe
recorded prices, not sell-through. The streaks behind it are a
*reconstruction* — a +1 step occurs only on `adjust_price`'s ≥9/10 raise
branch, so the price series is a direct readout of the predicate, but it
is inference, not measurement. It ships provisional and item 9 freezes it
against a real streak series, on the pack-1/pack-2 re-measure precedent.

### D6 — Plan-owned shapes

- `found_template(good) -> FoundTemplate { price, wage, headcount }` — the
  accessor that keeps §8.6 intact. `Prospectus` is **not** widened (that
  would be a third erratum) and the template numbers never appear in
  sim.rs.
- `World::is_fully_vacant(house) -> bool` factors the vacancy predicate
  written by hand in `immigrate` and the Arrive decide and about to become
  a fourth copy. `immigrate` keeps its `UnknownHouse` arm first so error
  ordering stays pinned.
- `FOUND_CAPITAL_BILLS` and `FOUNDER_RESERVE` live in sim.rs beside
  `invest` (firm bookkeeping, the `QUIT_ARREARS_BILLS`/`GRUBSTAKE`
  precedent); the founding template and both signals live in market.rs
  beside `plan_founding` (entry choice is market logic, §8.6).
- `Event::Founded`'s `capital` is **read back** as the new business's gold
  balance after the stake, so the event can never lie about a failed one
  (the balance-delta precedent).

### D7 — What this pack does NOT claim

Every number past the first founding is a **prediction on a diverged
trajectory**. The gate arithmetic pins the pre-divergence stretch — Food
founds at t142, Entertainment at t155 (or t157 via tier 1) — and item 9
measures the rest. Two measured facts bound the outcome and survive the
divergence: no capital constant above zero is satisfiable in the
t166–t200 hole, and Luxury's tier-1 refound lands inside it. **If the
diverged town still empties, pack 3 slows the cascade without arresting
it, and the ledger must say so in numbers rather than claiming a cure.**

The residual is a **circulation** failure, not a firm-count one: money is
conserved but `target_days` purchase caps make any wallet above the cap a
sink that never returns, so the town starves beside its own gold (99.15%
in one wallet, External at 403, every closure with zero proceeds). Phase
7's demurrage/imports and phase 8's mint are the standing TODO stubs that
address it — a successor container, not this one.

## Items

- [x] **1. `Business.sold_out_ticks` and its single writer.** The field
  with its doc contract (single writer = phase 4's price write-back;
  increment on the raise predicate, reset on a lower, HOLD on
  `offered == 0`); seeded 0 by `create_business`; the write-back extended
  in `goods_market`, which already holds the offered stock, the units sold
  and the `house_mut` handle. The predicate factored as one private
  `sold_out(offered, sold)`. Tests: single-writer (the
  `insolvent_ticks_has_a_single_writer` model), streak accumulates and
  resets, `offered == 0` holds. Done: `./scripts/check.sh` clean;
  `cargo test sim:: market::` quoted. Touches: src/business.rs, src/sim.rs,
  src/world.rs.
- [ ] **2. The market.rs founding band.** A third `// ---` band after
  `cheapest_offer`: `SellerSnapshot`, `Prospectus`, `FoundTemplate`, the
  private founding table + `FOUND_SIGNAL_TICKS` in the price/wage
  constant-block layout, `found_template`, private `found_signal`, and
  `plan_founding` as the two `Good::ALL` passes. Module doc widened to
  name the third pure planner. Tests: every gate case (2 sellers refuses;
  1 seller with streak but sub-coverage price refuses; 1 seller at
  coverage price with streak 0 refuses; 0 sellers founds at `FOUND_PRICE`;
  tier 1 outranks tier 2 across goods), reversed-input determinism, a
  table-driven template pin on the `constants_match_the_spec_table` model,
  and a payroll-coverage pin (`FOUND_PRICE × production_rate >= wage` and
  `FOUND_SIGNAL >= ceil(wage / production_rate)` for all three goods).
  Done: `./scripts/check.sh` clean; `cargo test market::` quoted.
  Touches: src/market.rs.
- [ ] **3. `World::is_fully_vacant` and the de-duplication.** The
  predicate beside `occupants_of`/`employees_of` (false for an unknown
  house, documented); `immigrate`'s two `HouseNotVacant` arms and the
  Arrive decide rewritten onto it, `immigrate` keeping its `UnknownHouse`
  arm first. Behaviour-preserving by construction — the ledger quotes the
  200-tick migration soak unchanged. Done: `./scripts/check.sh` clean;
  full suite green. Touches: src/world.rs, src/sim.rs.
- [ ] **4. `World::found_business`.** Signature verbatim, placed after
  `immigrate`: founder known FIRST (`UnknownAgent`), then `UnknownHouse`,
  then `is_fully_vacant` (`HouseNotVacant`), then forward to
  `create_business`. Money-free; writes no agent field. Tests:
  validates-founder-then-vacancy with `Err` changing nothing;
  money-free-and-wraps-the-constructor (account-only id, owner == founder,
  every counter 0, both ids zero on every metal, `total_money` unchanged,
  audit clean); and the stake seam succeeding immediately after. Done:
  `./scripts/check.sh` clean; `cargo test world::` quoted. Touches:
  src/world.rs.
- [ ] **5. Phase 6's founding.** `Intent::Found { founder, house, good,
  price }`; `Event::Founded { business, founder, house, good, price,
  capital }`; a private `seller_snapshots` shared by the decide and the
  apply's re-check; `decide_founding` as `invest`'s first statement, above
  the closure collect; `apply_found_intent` with the four kill-only
  re-checks collapsed to owned scalars before any `&mut`, then
  `found_business` → stake → self-hire → event, `capital` read back from
  the balance. The apply call goes between the closure loop and the draws
  collect (spec order: closures → Found → draws → write-back).
  `FOUND_CAPITAL_BILLS` and `FOUNDER_RESERVE` beside `invest`. The three
  shell matches gain arms (compile-forced). Tests: a founding lands and
  the new firm sits exactly at buffer with a ZERO draw the same tick;
  `FOUND_CAPITAL_BILLS == DRAW_BUFFER_BILLS` pinned; each re-check kills
  cleanly with nothing half-founded; at most one per tick; the decide
  reads the phase-start snapshot (a firm closing this tick still counts,
  so a refound is a t+1 event). Done: `./scripts/check.sh` clean;
  `cargo test sim::` quoted. Touches: src/sim.rs, src/engine/game_loop.rs.
- [ ] **6. Fixtures and the controlled soaks.** New sim.rs helpers
  (`capitalized_idle`, a demand pool, `set_price`, an unstaffed seller,
  a ticks-until-event runner), all built on `landlord_owner_business`,
  never `staffed_business`. Then the spec's two named fixtures: a
  single-seller closure with live demand founding within a pinned window
  (`tc+2..tc+4` — the phase-start snapshot makes this tick's closure
  invisible to this tick's decide, then `FOUND_SIGNAL_TICKS` sell-out
  ticks accumulate), and a demand-death founding **nothing**, with its
  positive control in the same test. Plus the vacant-house competition
  fixture: one vacancy, an aged clean slot and a qualifying founder in one
  tick — the arrival wins on phase order and the Found intent dies on its
  re-check. Done: `./scripts/check.sh` clean; `cargo test sim::` quoted.
  Touches: src/sim.rs.
- [ ] **7. Soak hardening.** Lands before the town re-measure so
  intermediate runs fail readably. 100-tick soak: an explicit
  `Event::Founded` tally asserted zero (the match ends `_ => {}` and would
  ship green observing nothing), and a per-tick "no good drops below 2
  sellers" assertion so the criterion names its own cause. 50-tick soak:
  guard the `PLATEAU` slice against a `usize` underflow **panic**. Done:
  `./scripts/check.sh` clean; all three soaks quoted green. Touches:
  src/engine/worldgen.rs.
- [ ] **8. The 200-tick soak raised and instrumented.** A per-good event
  stream via a `HashMap<AgentId, Good>` side table seeded from the boot
  `businesses()` set and extended on every `Founded`; the rolling-100-tick
  anti-churn bound per good; the full-cycle chain as four deliberate
  observations (`Closed{house}` → `is_fully_vacant(house)` → a strictly
  later `Founded{house}` or `Arrived{home}` → a `Hired` naming the founded
  business); and the floor **raised from this pack's own measured run**,
  replacing pack 2's two zero-margin bounds in the same change. Ledger
  records the new closure-tick series against pack 2's
  140/153/156/171/172/201. Done: `./scripts/verify.sh` green. Touches:
  src/engine/worldgen.rs.
- [ ] **9. The re-measure and freeze.** From item 8's run record, per
  good: the lone-survivor sell-out streak series, the streak at every
  `Founded`, the longest streak posted by a survivor that then closed, the
  founder wallet distribution at every firing tick, and the found→close
  cycle count per 100-tick window. Freeze or retune in this order:
  `FOUND_SIGNAL_TICKS` (strictly above the longest dying-survivor streak);
  the founded headcount (freeze if the churn bound holds, else climb the
  recorded ladder); `FOUNDER_RESERVE` (freeze only if it was binding at
  some firing tick — otherwise carry it forward PROVISIONAL and say so).
  Any retune re-runs item 8. Done: `./scripts/verify.sh` green; ledger
  carries the numbers. Touches: src/market.rs, src/sim.rs (constants only).
- [ ] **10. Pack and container close.** Errata A and B appended to the
  spec at the point of the wrong sentences; CLAUDE.md structure update;
  the INVENTORY 🚧 pointer folded into a real entry — **the container's
  definition of merged**; both ledgers closed with the measured
  before/after and an honest verdict; PR body updated. Done: `VERIFY OK`
  quoted. Touches: docs/*, CLAUDE.md.

## Not in this pack unless you say so

- No change to founder eligibility. Unemployed-only is the owner's gate
  ruling (open question 1) and it stands — **with its measured price tag
  now recorded**: it makes the wallet holding 99.15% of the gold supply
  ineligible to found. Flagged, not overridden.
- No per-good post-closure cooldown on tier 1. Recorded in Erratum A as
  the fallback if the anti-churn soak shows tier 1 is a churn engine; not
  built, because it would add cross-tick state nothing in `World` carries.
- No growth path for a founded firm (phase 6's still-TODO
  expand-capacity half).
- No demurrage, imports or mint — the circulation seam this pack's
  residual points at, and a successor container's job.
- No `House.owners` behavior. Still rule-inert.

## Ledger

- **2026-08-30** — pack drafted. A 260-tick probe on the pack-2
  trajectory scored the signed gates before any code was written and
  found founding-as-signed blocked three independent ways (the
  measurement section above). Two errata follow from it — the two-tier
  scan and the level-AND-direction scarcity gate — plus one derived
  constant set (`FOUND_SIGNAL` 2/2/3 from payroll coverage) and one
  spec-sanctioned lever (headcount 2). Four-angle design panel
  (as-signed / anti-churn-first / cascade-cure-first / skeptic) judged
  under three lenses; cascade-cure-first won on the combined reading, with
  the payroll-coverage derivation and the sell-out-streak observable
  grafted, and a uniform signal of 2 and a headcount of 1 killed as
  numerically unsound.
