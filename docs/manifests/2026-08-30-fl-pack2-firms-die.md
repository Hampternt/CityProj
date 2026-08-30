# Firm lifecycle — Pack 2: Firms die

**Status:** IN PROGRESS — drafted 2026-08-30 from the signed spec's
Contracts, a fresh 200-tick measurement probe, and a judged four-angle
design panel on the one genuinely open decision (the closure trigger).
**Container:** [2026-08-22-firm-lifecycle.md](2026-08-22-firm-lifecycle.md)
**Spec contracts executed here:** `Business.insolvent_ticks` + the closure
trigger, `World::close_business` (+ the `ClosureReceipt`), the phase-6
closure pass, the Arrive decide's arrears exclusion, `remove_agent` gains
forced liquidation, `Event::Closed` / `Event::LaidOff` (+ `Settled`'s
widened doc), Amendment 19. `market::plan_founding`, `Intent::Found`,
`World::found_business` and `Event::Founded` are pack 3.

Observable (verbatim from the container): on a stress fixture the whole
death narrates — quits, then "The Brass Bell closed — karl pockets 12g" —
and the freed address inspects as a vacant residence.

## State on arrival (measured 2026-08-30)

| What | Measured |
|---|---|
| Head | `92cc955` on `claude/town-colony-sim-p1s06q`; tree clean; PR #2 green |
| Test suite | 160 passed, 0 failed |
| Phase 6 | `invest(world, report)` runs the draw only — a collect over `businesses()` (sim.rs:889–902) then a pay/emit loop (:903–922) |
| `Business` | `owner` live since pack 1; no counter field; `owed_to` arrears ledger live |
| `Business {` literals | 5 — business.rs tests ×4, `World::create_business` ×1. **`create_business`'s signature does NOT widen in this pack**, so its 32 call sites are untouched |
| `remove_agent` | `Result<(), WorldError>`, 9 call sites; one (world.rs:1062) is inside an `assert_eq!` |
| `AgentId` | derives `Debug, Clone, Copy, PartialEq, Eq, Hash` — **no `Ord`**; nothing in the repo has ever sorted by id |
| Exhaustive `Event` matches | 3, all in the shell (game_loop.rs:78, :213, :283) — these force. worldgen's two soak matches (:453, :690) end `_ => {}` and **do not** force |
| Arrears, healthy window (t≤100) | max consecutive owed-ticks **1**, peak `owed_total` 11g — Longacre Farm only; three venues never accrue a coin in 200 ticks |
| Arrears, shock window (t≤200) | terminal streaks 73 / 60 / 57 (Longacre / Brass Bell / Gilt Curtain), peaks 659g / 1033g / 569g = 4.7–7.2 bills |
| First quits | t134 / t146 / t148 at those same three venues |
| Zero-staff venues | **none** in 200 ticks (min live staff 2 of 4) — the deadbeat exclusion is per-applicant, so every quit is re-staffed within 1–2 ticks |

Every figure the pack-1 ledger published reproduces exactly under the
fresh probe. Three new facts it did not have are recorded under Decisions.

## Decisions (this pack's, within the spec's contracts)

### D1 — The trigger: the signed predicate, retuned to `CLOSE_INSOLVENT_TICKS = 12`

The pack-1 ledger framed pack 2's choice as a dichotomy: *either* make the
trigger magnitude-aware *or* set the threshold above the measured 73. **The
measurement refutes both horns**, so we ship the signed rule with a retuned
constant and no predicate change.

`Business.insolvent_ticks` counts consecutive ticks ending with
`owed_total() > Money::ZERO` — the signed predicate, verbatim. What changes
is only the number the spec itself hands the plan:
`CLOSE_INSOLVENT_TICKS` 6 → **12**.

Why a retune is the right currency: the spec lists `CLOSE_INSOLVENT_TICKS`
among "all tuning constants' exact values — provisional here, soak-tuned
then frozen per the `adjust_price` pattern", and the
`insolvent_ticks_single_writer_and_healthy_town_control` acceptance test
prescribes the procedure verbatim — "the observed per-venue maximum
recorded in pack 1's re-measure and the threshold frozen above it".
Observed maximum **1**; frozen at 12. That is the contract executing as
signed, not a divergence from it. Every alternative buys its fix with
predicate text, a field name, or the writer's phase — all of which *are*
signed contract.

Why the retune is *sufficient*: **arrears are bimodal, not a gradient.**
The probe's `owed > wage_bill/4` and `owed > wage_bill/2` columns are
identical at every venue at every threshold — no tick's arrears ever sat
between a quarter-bill and a half-bill. Healthy flicker is one isolated
tick at 1.4–8% of a bill; sick persistence is 57–73 ticks at 4.7–7.2
bills; there is nothing in between. Any threshold in [8, 20] separates the
two modes with a bare counter, so a magnitude floor buys only healthy-side
margin that a longer fuse already buys in time — at the price of a second
soak-tuned constant, a cross-constant invariant, a two-branch predicate,
and an erratum against the spec's most emphatic closure sentence.

Why not the other horn: above the measured 73, nothing closes inside the
200-tick soak at all, the pack's whole observable becomes fixture-only, and
the doomed fixture needs 74+ payroll ticks. Unnecessary — the long streaks
belong precisely to the venues the mechanic exists to kill.

**The three measured bounds, re-derived from the probe's own reach-ticks:**

- **Healthy ceiling = 1.** Max consecutive owed-ticks over t≤100 is 1, at
  Longacre only (ten isolated 2–11g flickers against a 140g bill). 12
  clears it by 11.
- **Doomed floor.** Terminal streaks 73 / 60 / 57 = 6.1× / 5.0× / 4.75× the
  fuse — every doomed venue dies with room.
- **Ordering floor = 7.** The crossing law `counter = k at end of tick
  (last-clean + k)` reproduces all twelve published reach-ticks exactly
  (127+k / 140+k / 143+k). With one tick of write-back latency, closure =
  128+k / 141+k / 144+k against first quits t134 / t146 / t148, so a strict
  one-tick lead needs **k ≥ 7** (Longacre binding). At k = 12: closures
  t140 / t153 / t156, slack **+6 / +7 / +8**.

**One tick of designed latency**, stated so the ledger reconciles: the
counter reaches 12 at tick *t*'s write-back and the firm closes at tick
*t+1*'s phase 6, so the effective fuse is 13 consecutive arrears-ticks.
Every closure tick quoted here is crossing + 1.

Rejected alternatives, recorded with their reasons so a later pack does not
re-litigate them from scratch:

- **A magnitude floor** (`owed > wage_bill/N`) — refuted by the bimodality
  above, and it introduces an immortality hole a strict-positive predicate
  cannot have: frozen arrears never grow again, so a venue frozen at 100g
  against a 50g bill would sit forever just under a 2×-bill gate.
- **A flow trigger** (consecutive ticks failing to cover payroll) — its
  diagnosis is right and is kept as a ledger note, but it voids the signed
  "write-back iterates the LIVE `businesses()` set after closures" clause
  (a phase-3 writer cannot run after phase-6 closures), and it is the one
  candidate whose threshold **cannot** be frozen from existing data: the
  probe recorded streaks and peaks, never uncovered-tick density. It is
  also blind to the case the mechanic must kill — a zero-staff venue
  accrues nothing and emits neither `WagePaid` nor `PayrollShort` ever
  again, so an event-keyed trigger never fires on a zombie. **The trigger
  reads the `owed_total()` field, never `PayrollShort` events.**
- **An abandonment disjunct** (close a firm with no live staff) — provably
  dead code in the shipped town, and OR-ing it into a counter named
  `insolvent_ticks` would make the feed narrate "closed: insolvent 12
  ticks" for a firm that owed nothing. Kept as a named pack-3 hazard.

**Recorded fallback, pre-derived so a red re-measure is a one-line decision
rather than a reopened design:** if the pack-2 re-measure finds a doomed
venue whose live-staff ledger clears intermittently under closure (the
probe records a sick venue's arrears "accrue-and-repay oscillating" above
its frozen floor), reset-to-zero is the fragile part and
`saturating_sub(1)` on a clear tick — a leaky bucket — is the answer.

**Standing obligation, into the constant's doc comment:** the healthy
flicker moved 0 → 1 when pack 1's draw thinned coffers. Any future pack
that touches coffers (a larger `DRAW_BUFFER_BILLS`, demurrage, imports)
must re-measure the healthy max streak and re-freeze this. Soak criterion 7
is the tripwire.

### D2 — One spec erratum: the "~4-tick quit horizon"

The signed ordering sentence reads: *"On deep (total-shortfall)
trajectories the threshold sits beyond the ~4-tick quit horizon, so worker
churn — the cheaper correction — fires first."* The "~4" came from reading
`QUIT_ARREARS_BILLS` against a full-bill-per-tick shortfall. Measured,
shortfalls are partial and the horizon in counter units is **6 / 5 / 4**
(Longacre t128→t134, Brass Bell t141→t146, Gilt Curtain t144→t148) — right
at Gilt Curtain, understated at the other two. The ordering claim is
therefore a tuning **constraint** on the constant, not a property of the
rule. Erratum recorded in the spec, dated, with the replacement written out
in full.

Two arithmetic corrections the erratum carries, both of which change what
this manifest may *claim* without changing the recommendation:

- **"6 is confirmed unshippable" was overstated.** With the write-back
  latency counted, k = 6 closes Longacre at t134 — the same tick as its
  first quit — and the quit decide is phase 1 while the closure pass is
  phase 6, so the quit still fires first *within* the tick. 6 is a
  same-tick tie, not an ordering violation. It is rejected for leaving zero
  slack on a trajectory closure itself perturbs.
- The strict-lead floor is therefore **k ≥ 7**, not 8.

### D3 — Plan-owned names and shapes

- **`WorldError::NoBusinessHere(HouseId)`**, placed directly after
  `BusinessAlreadyExists` so the pair reads as mirrors.
- **`ClosureReceipt`** lives in world.rs beside `WorldError` (a
  command-layer return value; business.rs holds no command-layer types),
  `#[derive(Debug, Clone, PartialEq)]` — `Debug + PartialEq` is a hard
  compile constraint from world.rs:1062's `assert_eq!`. Fields:
  `business: AgentId`, `house: HouseId`, `owner: AgentId`,
  `settlements: Vec<(AgentId, Money)>` (ascending creditor, **positive
  amounts only** — the receipt means "what actually moved", matching
  `apply_sinks_intent`'s existing `> ZERO` filter), `laid_off: Vec<AgentId>`,
  `residual: Vec<(Metal, Money)>` (every `Metal::ALL` entry in that order,
  **zeros included** — the `Departed { took }` visible-zeros precedent).
  **No `owner_name`:** the owner is alive at every call site's emission
  point (phase 6 reads the live agent; `apply_sinks_intent` already clones
  the leaver's name before the command), so the caller supplies it.
- **`remove_agent` returns `Result<Vec<ClosureReceipt>, WorldError>`** — a
  bare `Vec`, empty in the overwhelmingly common case. **No `#[must_use]`:**
  five world.rs tests call it in statement position and would fail
  `clippy -D warnings`.
- **Ascending settlement is `debts.sort_by_key(|(id, _)| id.0)`** — local,
  zero blast radius. `AgentId` gains no `Ord` derive (a type-level change
  with no other consumer), and settlement order is explicitly NOT derived
  by scanning `world.agents`: that yields the same sequence today only
  because `remove_agent` happens to guarantee every `owed_to` key is a live
  agent, a coupling `close_business` must not inherit.
- **`CLOSE_INSOLVENT_TICKS` is `pub(crate)`** so the soak criterion and the
  shell name the same symbol (the `DRAW_BUFFER_BILLS` precedent). A soak
  that re-spelled the predicate would agree today and diverge silently the
  first time it is tuned — a false green, not a failure. Same reason
  `insolvent_now(owed_total) -> bool` is a `pub(crate)` pure fn beside
  `draw_amount` rather than an inline comparison.

### D4 — The pack-1 dangling-owner tolerance is retired, not softened

The draw's `world.agent(owner).is_none()` skip and its test
`draw_skips_a_dangling_owner_cleanly` are **deleted** in item 3, once the
owner-emigration fixture is green — the spec provides for exactly this
("retired/retargeted when pack 2's forced liquidation makes the state
unreachable"). `close_business`'s owner transfer uses `.expect()`,
asserting the always-living-owner invariant in `remove_agent`'s existing
style, with the invariant pinned by test rather than by defensive code.
Two hard-panic dependents on one invariant is the deliberate choice: a
silent skip would orphan money on a dead account, which the totals-only
audit cannot see.

### D5 — Detach is last for a mechanical reason, not a stylistic one

`is_known_account` answers "is this a business id?" by scanning the live
`businesses()` set, so `house.business = None` is a **capability
revocation**: the instant it runs, every `pay` naming that id returns
`Err(UnknownAgent)` and the `.expect()` idiom panics. Mirrored in
`remove_agent`: the leaver is removed from `self.agents` last, so during
step 0 they are still a known account and `pay(dying_business, leaver, …)`
succeeds. Move liquidation after removal and the residual sweep fails
`is_known_account` on the `to` side.

### D6 — A dead firm's stock vanishes with it

`close_business` drops the `Business` and its `stock` with it (Longacre
carries 6414 unsold units of Food by t200), so **pack 2 quietly ships the
sim's first goods sink**. Chosen over the alternatives on the invariants:
transferring stock to the owner's inventory is a windfall they did not earn
and cannot consume at scale, and a fire-sale needs pricing logic in phase 6,
which §8.6 forbids. Consistent with the roadmap's "goods do not physically
move yet"; named here rather than left for the physical-goods spec to
discover, and handed to it as an open question. No invariant is touched —
every money flow rides `World::pay`, the audit stays unconditional per
metal, and goods carry no conservation rule.

### D7 — Amendments 16 and 17 are back-filled with 19

**Finding, 2026-08-30:** the 07-02 spec's header records rows 1/7 as
"amended by … Am. 15–17", but the table **cells** were never edited — row 1
still reads `none` and row 7 still reads ``burn`, `transfer`→External``.
`grep` finds neither amendment's text anywhere in that doc, while the code
follows both and pack 4's ledger calls Am. 16 "executed". Amendment 18
(pack 1) *did* edit its cell in place, which is the convention. Since
Amendment 19 edits row 7's money-ops cell, it cannot be written onto a cell
still missing Amendment 17's text. Item 4 executes 19 **and** back-fills 16
and 17, recorded as a correction rather than a silent edit.

### D8 — The endgame between packs 2 and 3, accepted as expected texture

With closure live and founding not yet landed, a 200-tick run of the
shipped town ends with all three demand-losing venues dead, one seller per
good, and no mechanism that can replace them. This is the signed sequence
working as designed ("death before birth — a mis-founded firm must die by
working closure rules, not zombie"), and the container's re-cut decision
already anticipates the demand-shocked venue dying. Holding the closure
pass behind pack 3 would gut this pack's only observable, so it is
**accepted and recorded**, not deferred. Scope of the exposure, measured:
nothing closes before t140, so the 100-tick soak, the 50-tick employment
soak, and the shipped interactive experience below t140 are all untouched.
Flagged in the ledger and PR #2 for owner acknowledgement.

## Items

- [x] **1. The counter and its predicate.** `Business.insolvent_ticks: u32`
  (doc: single writer, live-set-after-closures, the one-tick latency, and
  "do not reorder inside `invest`"); `insolvent_now(owed_total) -> bool`,
  `pub(crate)`, beside `draw_amount` in sim.rs.
  **`CLOSE_INSOLVENT_TICKS` moves to item 3**, where its first consumer
  lands: the constant has no non-test reader until the closure pass, and
  `clippy -D warnings` rejects a dead one (`#[cfg(test)]` use does not
  count for the bin target). The write-back appended as the LAST block of `invest`, copying
  `produce`'s collect-then-`house_mut` template (no `businesses_mut` — four
  phases already mutate `Business` fields that way). All five `Business`
  literals gain `insolvent_ticks: 0`. Tests:
  `insolvent_now_is_strict_at_zero`,
  `insolvent_ticks_single_writer_and_healthy_town_control` (the counter
  moves only in `invest`; increments while owed, resets on a clear tick).
  Done: `./scripts/check.sh` clean; `cargo test sim:: business::` quoted.
  Touches: src/business.rs, src/sim.rs, src/world.rs.
- [ ] **2. `close_business` + the `ClosureReceipt`.** Per D3 and D5, in the
  contract's exact order: validate (`UnknownHouse`, then `NoBusinessHere`)
  → collect-and-sort the ledger, settle `min(remaining coffer, owed)`
  ascending → `owed_to.clear()` (one call; removes zero-amount entries by
  construction) → `vacate_workplace` every `employees_of(house)` →
  `Metal::ALL` residual to the owner → `house.business = None` LAST. The
  collect-then-loop split is mandatory: holding a `&Business` across a
  `self.pay` is E0502, and `remove_agent`:367–384 is the shape to copy. No
  caller yet. Test:
  `close_business_settles_ascending_writes_off_and_sweeps_no_orphans` —
  multi-creditor coffer-short fixture **including the owner-as-creditor
  case**; ascending order pinned, remainders written off, zero entries
  stripped, every `Metal::ALL` balance of the dead id zero **per account**,
  `residual` = the step-3 sweep, owner wallet delta = own-arrears
  settlement **plus** summed proceeds (never "proceeds = owner delta",
  which conflates the two flows), the freed house passes the `immigrate`
  vacancy predicate, audit green; plus the two `Err` paths changing
  nothing. Done: `./scripts/check.sh` clean; `cargo test world:: money::`
  quoted. Touches: src/world.rs.
- [ ] **3. Phase 6 kills.** The closure pass at the TOP of `invest`, from
  the phase-start snapshot, houses order, one `close_business` per doomed
  house — placed above the draws collect so "a closing firm never draws" is
  structural rather than a guard that can rot. `Event::Closed { business,
  house, owner, owner_name, proceeds }` and `Event::LaidOff { agent,
  business }`; `Settled`'s doc widened from "phase 7" to "a business
  settles arrears outside payroll"; all three shell matches gain arms
  (compile-forced), including the per-report address index so `Settled` and
  `LaidOff` still render a detached business id. **Retire D4**: delete the
  dangling-owner skip and `draw_skips_a_dangling_owner_cleanly`. Tests:
  `closure_fires_on_persistence_after_quits` (the doomed fixture — workers
  quit first, the counter ages on frozen post-quit arrears, closure fires
  at the threshold) and a decide-driven stress fixture that closes with
  **both workers still employed** so `LaidOff` is non-empty; events sourced
  from the receipt, never re-derived. The fixture's revenue rig is
  load-bearing and must be commented with its bound — per worker per tick
  the shortfall `s` must satisfy `s ≤ 3w/(k+1)`, or the test silently
  becomes a quit-first test with an empty `LaidOff` vector, still green,
  proving less. Done: `./scripts/check.sh` clean; `cargo test sim::`
  quoted. Touches: src/sim.rs, src/engine/game_loop.rs.
- [ ] **4. Forced liquidation (Amendment 19).** `remove_agent` gains step 0
  after validation and before the A17 settlement — collect
  `Vec<HouseId>` of houses whose business's owner is the leaver (the
  snapshot is mandatory: iterating `businesses()` while calling
  `close_business` is E0502), close each, return the receipts.
  `apply_sinks_intent` binds them and emits `Closed`/`LaidOff`/`Settled` in
  causal order before `Departed`, and its **A17 creditors snapshot gains
  the owned-firm exclusion** — without it a leaver-owned firm's closure
  flows are re-narrated as one bogus A17 `Settled`. `remove_agent`'s own
  internal debts snapshot is deliberately left alone: it is re-derived from
  `businesses()` *after* step 0, so a closed firm is already detached and
  cannot be double-settled — a filter there would be dead code that reads
  as load-bearing. The `External` delta recipe feeding `Departed.took`
  stays valid unchanged (closure pays the owner, never External, so
  External still has exactly one flow inside the command). Amendment 19
  executed in the 07-02 spec **plus the D7 back-fill of 16 and 17**. Test:
  `owner_emigration_forces_liquidation_no_orphans_on_either_id` — the
  dedicated fixture (owner is simultaneously creditor and leaver, with a
  laid-off non-owner also owed), both dead ids zero per `Metal::ALL`,
  External's delta exact, no bogus `Settled`, causal event order, audit
  green. Also re-cut the three sim.rs tests the `staffed_business` fixture
  silently makes owner-emigrations — `settlement_is_narrated_before_the_departure`
  (rebuild with a non-owner leaver: it is the ONLY test pinning pure A17
  narration order and must stay on that path),
  `departed_workers_slot_ages_into_a_pull` (needs an off-premises owner, or
  its venue is detached at the moment of departure and there is no slot to
  age — pack 3 needs this test alive to restore the soak's arrival
  criterion), and any third the compiler surfaces. Done:
  `./scripts/check.sh` clean; `cargo test world:: sim::` quoted. Touches:
  src/world.rs, src/sim.rs, docs/superpowers/specs/2026-07-02-*.md.
- [ ] **5. The pull stops recruiting into deadbeats.** One conjunct,
  `business.owed_total() == Money::ZERO`, in the OUTER closure of both the
  Arrive decide's `slot_aged` scan and the apply's `still_hiring` re-check.
  Test: `arrive_pull_skips_arrears_carrying_venues` — an aged slot at an
  owing venue pulls nobody; the same slot clean pulls. The existing four
  pull tests all build on roleless-landlord fixtures whose `owed_total` is
  zero, so the exclusion costs the suite nothing — verify, do not assume.
  Done: `./scripts/check.sh` clean; `cargo test sim::` quoted. Touches:
  src/sim.rs.
- [ ] **6. The shell shows distress and death.** Distress on the business
  inspect view (`insolvent_ticks` + "closes at N", mirroring the agent
  view's hunger line) and beside the frame's `owed` when non-zero; a
  vacant/freed-house `inspect` branch (owners, occupants, vacancy stated
  against the `immigrate` predicate) so a closed venue's address still
  resolves; the `closures` feed bucket placed between `hungry` and `draws`
  so phase-6 order reads correctly and a forced liquidation still precedes
  its `Departed` line. Done: `./scripts/check.sh` clean; behavior eyeballed
  at `cargo run` on a fixture that closes, quoted in the ledger. Touches:
  src/engine/game_loop.rs.
- [ ] **7. Re-measure, re-cut, freeze.** Re-run all four soaks with closure
  live. **Criterion 7** added to the 100-tick soak: per-venue max
  `insolvent_ticks < CLOSE_INSOLVENT_TICKS` across the span, **paired with
  a vanish-detector** (`world.businesses().count() == 6` every tick and a
  hand-written zero-`Event::Closed` tally) — a closed venue drops out of
  `businesses()` and a max-tracking map would score its absence as "never
  got high", and worldgen's match arms do **not** compile-force. **The
  named re-cut** of the 200-tick migration soak: BOTH
  `assert!(first_arrived_after_departure.is_some(), …)` **and**
  `assert!(rose, "population never rose across a tick")` move to pack 3 —
  the spec's re-cut language names only the first, but `rose` is satisfiable
  only by an arrival (`immigrate` is the sole path that grows
  `world.agents`), so moving one without the other still ships the soak
  red. Departure criteria stay; the departure timeline is **re-measured,
  not re-pinned** — only Longacre's t140 closure is a clean counterfactual,
  everything after it comes from code in which closure did not exist, and
  layoffs push more agents into the dis-saving pool. Record the observed
  per-venue counter maximum and **freeze `CLOSE_INSOLVENT_TICKS`** above it;
  sample in phase 6, not end-of-tick (the probe's series is biased high on
  departure ticks). Every number quoted in the ledger as the pack-3
  baseline. Done: all four soaks green; ledger carries the new baseline.
  Touches: src/engine/worldgen.rs, src/sim.rs (constants only, if retuned).
- [ ] **8. Pack close.** `./scripts/verify.sh` green; multi-lens close
  review; ledger quotes real output + the new test count; container entry
  updated; CLAUDE.md code-state section (phase 6 closes firms; `Business`
  gains `insolvent_ticks`; `remove_agent` force-liquidates); INVENTORY's
  🚧 pointer folded for the closure half; PR body updated. Done: `VERIFY OK`
  quoted. Touches: docs/manifests/*, CLAUDE.md, docs/INVENTORY.md.

## Not in this pack unless you say so

- No `market::plan_founding`, no `Intent::Found`, no `World::found_business`,
  no `Event::Founded`, no `FOUND_*` constants — pack 3.
- No restoration of the 200-tick arrival criterion — pack 3 owns it, by the
  container's named re-cut.
- No magnitude floor and no abandonment disjunct on the trigger (D1) — both
  considered, both rejected on the measurement, both recorded so pack 3
  need not re-derive the argument.
- No ownership transfer, sale or inheritance on closure; the residual goes
  to the owner and the firm ends.
- No `House.owners` behavior of any kind — still rule-inert.
- No goods disposal mechanic beyond D6's vanish; no fire-sale, no pricing
  in phase 6.

## Ledger

- **2026-08-30** — pack drafted. The one open decision (the closure
  trigger) was settled by a fresh 200-tick instrumentation probe plus a
  judged four-angle design panel (persistence-purist / magnitude-aware /
  flow-not-stock / from-first-principles, scored under spec-fidelity,
  measured-satisfiability and implementation-determinism lenses). The
  probe reproduced every pack-1 published figure exactly and added three
  facts the ledger did not have: arrears are **bimodal** (the `bill/4` and
  `bill/2` columns are identical at every venue at every threshold — no
  tick's arrears ever sat between them); **no venue ever reaches zero live
  staff** in the shipped town, so the zombie case is fixture-only and an
  event-keyed trigger would be permanently blind to it; and **emigration
  writes frozen arrears down** (`Settled` fired at four venues' departure
  ticks), so the "freeze is permanent" reading holds only while the ghost
  creditor remains in the world. Outcome: ship the signed predicate,
  retune 6 → 12 (D1), one erratum on the "~4-tick horizon" sentence (D2).
  The pack-1 ledger's either/or dichotomy is recorded as **refuted** — it
  conflated "the streak is long" with "the threshold must exceed the
  streak", and nothing requires the latter because the long streaks belong
  to the venues the mechanic exists to kill. Separately found and recorded:
  Amendments 16 and 17 were never executed in the 07-02 phase table (D7).
