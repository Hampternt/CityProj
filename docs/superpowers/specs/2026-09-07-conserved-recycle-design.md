# Conserved Recycle — Design Spec

**Status:** DRAFT — awaiting the approval gate. ·
**Parent:** [2026-07-02-money-gameloop-skeleton-design.md](2026-07-02-money-gameloop-skeleton-design.md)
(grandparent: [2026-06-20-economy-sim-design.md](2026-06-20-economy-sim-design.md));
successor to [2026-08-22-firm-lifecycle-design.md](2026-08-22-firm-lifecycle-design.md),
whose closing ledger handed the residual decline forward.
**Shape:** container — three packs, one level deep; the container manifest
(`docs/manifests/`) is written from this spec's Contracts after sign-off.
**Branch:** `claude/town-colony-sim-p1s06q`

> **Commissioned as "the circulation seam"; renamed on measurement.** The
> firm-lifecycle container closed by naming a successor: *"the circulation seam:
> demurrage, imports through External, and the Mint business — phase 7's and
> phase 8's standing stubs."* Roughly 40,000 simulated ticks across fourteen
> probes were run before a line of this design was written, and they refuted that
> framing at its mechanism, refused two of its three named mechanics outright, and
> left the third viable only in a shape the framing did not anticipate. The
> milestone is therefore named for what it ships. **The old name is kept here so
> the handoff is traceable, not because the old diagnosis survived.**

## Design (decisions — approve before reading on)

The town's decline is not caused by the `target_days` purchase cap, is not an
aggregate income/spend deficit, and is not a firm-count failure. It is
**bottom-tail solvency on top of a technology fact**. Measured: household income —
wages plus draws — averages 772.5 g/tick against 772.3 g/tick of purchases over
t20–t120, balanced to 0.03%, and the 1000-tick household ledger closes to a
**+0 g residual**. That near-equality is *measured, not structural*: firm revenue
is household spend by construction, but household income is wages **plus draws**,
and firms withhold above `draw_amount`'s retained buffer — so what the window
shows is that at steady coffers there is no **aggregate** gap for a mechanic to
close. The gap that remains is distributional, and moving idle stock into the
wallets that spend it is what this milestone ships.
What there is: **30 residents, 21 job slots, and a technology that strictly
requires 22.5 workers** (0.75 per capita) to feed 30 people, so nine residents
receive **0 g of wages across 200 measured ticks** and burn savings at a mean
20.8 g/head/tick; the first-departure tick is *linear in the seed*,
`t = 6 + savings/28.05`, across nine seed values from 0 to 20,000. The whole
300-tick baseline is that savings burn with a closure cascade bolted to its end —
departures **lead** closures (first `Departed` t127, first `Closed` t140).

Of those ~40,000 ticks, **exactly one arm reaches a stationary state**: levy a
per-mille of every agent's gold each tick and pay the pot straight back out in
**equal shares** to every agent. At 20‰ the town is an exact fixed point — pop
30, 6 firms, 21 employed, 0 closures, 0 departures, 0 quits, every reported
metric bit-identical from t600 to t10000 *(measured on the probe arm; the record
does not state how that arm divided its pot, so this spec's lowest-`AgentId`
remainder rule is unmeasured and pack 2 re-measures the fixed point and its period
under it — see the `sim::mint_phase` contract and A3)* — and it is a *live* fixed
point, not a
rule-frozen corpse: 68–82 `Sold` events and 632–912 g of goods turnover per tick,
662–702 g of wages, 508 units produced, ~987 g/tick of levy (1.89% of the 52,148 g
supply), identical at t500, t2000 and t5000, with per-agent (gold, food) vectors
repeating exactly at lag 10 and **never** at lag 1/2/5 — L = 10 measured on the
probe arm; pack 2 measures and pins L under this spec's remainder rule (A3).
**What the gate buys is the stability outcome** — pop 30, 6 firms, 0 closures, 0
departures, 0 quits — not the bit-identity, which this spec does not inherit.
What it cures is the
bottom tail: agents holding less than one tick's spend go from 3 of 10 (null,
t300) to **0 of 30 at every horizon**, and the largest wallet falls from 27.8% to
~6.2% *of the 52,148 g supply*. What it does **not** cure is stated up front and
charged for in the gate: **94.0% of the supply still sits in agent wallets above
one tick's spend ceiling** at t300 and unchanged at t2000 — only 3.5 points below
the null's 97.5% at the same tick. *(The probe record does not state which spend
figure it used as the ceiling, and 94.0% does not reconcile exactly with the
49,520 g / 95.0% agent-held base used elsewhere in this spec — 49,520/52,148 is
94.96%. The unreconciled gap is recorded rather than smoothed; the 49,520 g base
is the one every derivation below relies on.)* **The cured town starves beside its own gold exactly as much; it
simply does not starve.**

This milestone ships that mechanic **and nothing else**. It is expressed as row
7's pre-authorised `burn` plus row 8's pre-authorised `mint` — an expression
measured *tick-for-tick identical* to a direct agent→agent transfer **placed at the
same point in phase 7, below the emigration decide**, at every rate and every
horizon to t10000, with `total_money(Gold)` never moving off 52,148 and the audit
green all 10,000 ticks. No new module, no `node.rs`, no new `Good`, no
new agent kind, no new persistent state and therefore no new single-writer
obligation: **the only datum crossing a phase boundary is one `Money` scalar in
`tick()`'s stack frame.**

Three things this design does deliberately differently from the naive form of
that mechanic, each because an adversarial pass refuted the naive form.

1. **The magnitude rests on the direct sweep alone.** 15‰ measured
   minimum-sufficient, 20‰ measured an exact fixed point, payroll shortfalls 395
   per 2000 ticks at 20 against 3,227 at 15. The tempting mechanism story — that
   20‰'s ~33 g/head share clears the 28.05 g basket while 15‰'s ~25 g does not —
   is recorded **as refuted, not as conjecture**: on a base of 49,520 g of
   agent-held gold, the share-clears-burn crossing is at **16.99‰** (16.96‰ on the
   constants-derived 28) — either way **two per-mille *above* the measured
   sufficiency threshold of 15** — so the story mis-predicts the one sweep point
   the record contains. Run on the other branch
   of the unresolved spend contradiction (36.0 g) it demands 21.81‰ and would
   declare the frozen 20 *insufficient*. Per-agent spend therefore enters this
   spec **only as an acceptance denominator**, never as a tuning input.
2. **The faucet gets a real refusal point.** `Accounts::mint` is infallible, its
   gold-reserve cap is deferred in its own doc comment, and the §8.3 audit
   **provably cannot detect a mint at all** — `mint` raises the credited balance
   and `total_minted` by the same amount, so the identity holds for *any* mint,
   matched or not. So the matching property is not left in prose plus one soak:
   `World::disburse` is **pot-scoped** (it refuses beyond the remaining pot), and
   `tick()` asserts `Δtotal_minted(Gold) == Δtotal_burned(Gold) == pot` beside the
   audit **on every tick of every scenario**. That assertion is this milestone's
   substitute for the deferred backing cap.
3. **The amendment is honestly priced.** Row 8's money-ops cell says "`mint` only"
   (satisfied), but its *purpose* cell says "new money from reserve", which a
   recycle disbursement is not. Amendment 20 rewrites that purpose as a **closed,
   named grant** — not the open class "matched re-issue of a same-tick sink",
   which any future phase-8 mechanic could claim by asserting its own matching. A
   row-8 mint purpose is the one grant that can create money and the one §8.3
   cannot police; it is priced as a real grant, not waved through as description.

- **Fits into:** the phase-7/phase-8 seam in `src/sim.rs` and the command layer in
  `src/world.rs`; nothing else grows a field. `sinks` (@src/sim.rs:1327) keeps its
  emigration decide/apply verbatim — the `cheapest_food` guard whose own comment
  already reserves this slot ("the guard scopes the DECIDE, not the phase, so
  future phase-7 mechanics (demurrage, imports) appended below stay
  unconditionally reached") — and gains the levy pass below it plus a `Money`
  return; `mint_phase` (@src/sim.rs:1452), today literally
  `fn mint_phase(_world: &mut World) {}`, wakes and gains `pot` and
  `&mut TickReport` (the Amendment-15 provision for stub phases waking); `tick`
  (@src/sim.rs:232) threads the one scalar and gains the matched-issue assertion
  immediately before `world.accounts.audit()`; `Event` gains one aggregate
  variant; the constant and the pure helper sit beside their phase (the
  `QUIT_ARREARS_BILLS` / `DRAW_BUFFER_BILLS` / `draw_amount` / `insolvent_now`
  placement precedent — this is household bookkeeping, not §8.6 market logic,
  stated here so the plan never re-litigates the split). `src/world.rs` gains two
  command-layer wrappers beside `pay` (@src/world.rs:228), today the **only**
  wrapper over the §8.2 chokepoint — there is none for `mint` or `burn` — plus one
  `WorldError` variant. `src/money.rs` changes no mover; `Accounts::burn`
  (@src/money.rs:218) loses its `#[allow(dead_code)] // the sinks phase (7) lands
  later`, a comment that has been waiting for exactly this. `game_loop.rs` gains
  one forced `match` arm and a derived `net = minted − burned` column.
  `worldgen.rs` changes no scenario — `town_world` and its genesis totals (gold
  52,148 / silver 300 / copper 600) are untouched — but three shipped acceptance
  artifacts are invalidated by the cure and are re-cut in the pack that lands it
  (see **Migration impact**). **Untouched by the recycle mechanic:** `market.rs`
  (neither leg reads or writes a price or wage — §8.6), `business.rs`, `agent.rs`,
  `goods.rs`, `metal.rs`, phases 1–6 in every respect, and `Intent` (both legs are
  direct passes, the `pay_wages`/profit-draw precedent). **Packs 1–2 touch none of
  them. The one exception in the container is pack 3's founding-template sweep**,
  which edits `FOUNDING_TEMPLATE`'s table comment (@src/market.rs:318–361) whatever
  the sweep concludes and — only if the gate answers open question 6 "yes" and the
  headcount re-freezes away from 2 — the headcount column itself, which phase 6
  reads in both the founding decide and the apply. Pack 3 is independently
  droppable; dropping it restores this list in full.
- **Invariants:** **§8.1** — `levy_amount` is
  `balance.times(permille).divided_by(1000)`, flooring, on the existing checked
  `Money` helpers; the share is `pot.divided_by(heads)`, flooring; no float
  anywhere. *The integer floor is also the design's progressivity:* at 20‰ every
  balance below 50 g levies exactly zero and still receives a full share, so the
  bottom tail this exists to protect pays nothing **by construction rather than by
  a guard**. **§8.2** — the only mutators reached are `Accounts::burn` and
  `Accounts::mint`, each behind exactly one new command-layer wrapper that
  validates before touching state (the `World::pay` convention: `Err` always means
  nothing changed). No new mover; no direct `Accounts` call from `sim.rs`.
  **§8.3** — untouched and still the final call in `tick`, and *strengthened*
  rather than relied on: because the audit is totals-only **and** provably blind
  to minting, the milestone adds a per-tick, every-scenario assertion beside it
  (Gold's two deltas equal `pot`; every other metal's are zero), plus the exact-pot
  property `sum(disbursed) == pot`, discharged by `mint_phase`'s
  threaded-pot-ends-at-`Money::ZERO` assertion and gated per tick by A8 — the
  command layer refuses an over-issue against the remaining pot it is handed, and
  an *equality* is not a property any per-call check can carry, which this spec
  does not claim it does. Dropping the
  remainder would deflate the supply by up to 29 g/tick — up to 290,000 g over
  10,000 ticks against 52,148 — **while still passing the audit**: the class of bug
  §8.3 structurally cannot see, and the same reason the firm-lifecycle spec proved
  liquidation completeness with per-account assertions. **§8.4** — honoured
  literally; what changes is what the two logs *mean*, and that is paid openly
  (Amendment 22 and open question 2) rather than hidden. **§8.5** —
  `levy_amount(b, p) <= b` for every `p <= 1000`, so the burn leg cannot
  overdraft; the payout leg mints and cannot; the pot-scoped `disburse` cannot
  over-issue. Every `Result` is bound and `.expect`ed with its justification,
  never `let _ =` and never `.is_err()` — the pack-2 discarded-receipt erratum
  (@src/sim.rs:1402) is the recorded precedent for what that costs. **§8.6** — a
  levy is not a price and not a wage; neither leg of the mechanic touches
  `market.rs`, and neither leg
  reads a posted price. **Determinism without RNG** — both passes iterate
  `world.agents` order, ascending `AgentId` by construction; the remainder
  tie-break is lowest `AgentId`; `Metal::ALL` order for the per-metal assertions.
  **Single-writer** — no new counter and no new persistent state, so no new
  obligation; the one cross-phase datum is a local in `tick()`. **Phase order** —
  inside phase 7 the emigration decide/apply runs FIRST and unchanged, so a
  departing agent is never levied (their gold reached External inside
  `remove_agent` before the pass) and phase 8's roster is provably the roster
  phase 7 levied, nothing running between them but `tick()`'s own control flow.
  **Standing obligation discharged, not assumed** — `CLOSE_INSOLVENT_TICKS`
  (@src/sim.rs:328) binds any pack that touches coffers; this one changes demand,
  therefore revenue, therefore coffers, and 395 payroll shortfalls per 2000 ticks
  were measured at this rate, so the healthy max streak is re-measured and the
  constant re-frozen or confirmed **with the number quoted**.
- **Out of scope (YAGNI) — each refusal carries the measurement that earned it:**
  - **Any new module.** No `node.rs`, no policy layer, no Treasury account, no
    reserved-id shift. §7's `node.rs` has never been created and nothing measured
    requires it: the winning arm works as a rate and a rebate, and a policy layer
    is a re-skin of the same two calls. Recorded so the successor who builds it
    finds the migration already scoped — `tick_with_rate` is the policy-as-data
    seam in miniature; if a second policy ever lands, the constant becomes a field
    and the pass moves.
  - **Any new `Good`, agent kind, `Business` field, `World` field, or persistent
    counter.**
  - **The literal staffed §2.1 Mint, gold-as-a-good, seigniorage, and the deferred
    gold-reserve/backing cap.** `Accounts::mint` stays infallible and this
    milestone adds **no** backing-ratio enforcement. The pot-scoped `disburse` plus
    the per-tick matched-issue assertion bound *this* faucet arithmetically; that
    is a balance-shaped guarantee, not a ratio-shaped one, and must not be read as
    having designed faucet policy. *(Also noted for whoever does: CLAUDE.md's
    roadmap line "a literal staffed Mint business that **consumes** precious-metal
    goods" contradicts §2.1 and hard invariant 4, both of which say gold is held
    in reserve, **not consumed**. That contradiction is flagged here and left to
    the Mint's own spec to rule.)*
  - **A minted stipend in any form.** It works, and it has no stable threshold —
    the minimum sufficient value climbs 23 → 24 → 25 → 26 → 27 as the horizon goes
    t2000/t3000/t5000/t10000, so any value frozen today is known-good only to its
    tested horizon. It mints **2.43M gold by t10000 against a 52,148 genesis
    supply (46×)**, its magnitude *is* a basket level so it inherits the spend
    contradiction whole, and it re-opens precisely the faucet the 07-19 pricing
    spec deleted as "the accepted broken faucet".
  - **Demurrage as a net sink, and any unrebated or flat-per-head levy.** 46
    configurations — rates 1/2/5/10/20/50/100‰, burned vs transferred to External,
    on agents / agents+coffers / thresholded at 10×–1000×, at 300 and 600 ticks —
    and **not one clears the bar**. Best sink arm is t300 pop 11 against the
    baseline's 10 and t600 pop **3 against 4**: a delay, not an arrest, costing
    20.8% of the supply, because the idle gold *is* the savings the jobless live
    on. Its flat-looking high-rate arms are **frozen corpses**: with no Food seller
    the destitution decide is guarded, nobody can be judged destitute, and
    population freezes *by rule*.
  - **Exports, imports, and any change at the External seam.** External's entire
    capital is **600 g**; every export setting from 1 unit/firm to half of all
    stock moves 623–796 g across 300 ticks while starving 241–300 of them — the
    seam is **unfunded, not underpowered** — and funding it is lethal: outside
    demand outbids residents, `sold_out` fires every tick and `adjust_price`
    ratchets with no ceiling (Food 179 g at t100, no Food seller by t200; the
    series diverges to 3.3 × 10¹⁰/unit at 40 units/firm). Imports as pure outflow
    are monotonically harmful (20 g/agent/tick extinguishes the town; a 1/20
    wallet-proportional drain kills it by t83). Under this milestone External's
    gold **does not move at all**, pinned by an acceptance criterion.
  - **Any change to phase 6** — the profit draw, `draw_amount`,
    `DRAW_BUFFER_BILLS`, the founding decide's *logic*, or
    `CLOSE_INSOLVENT_TICKS`'s *value* (its re-measure is in scope; changing it on
    any other basis is not). **Named exception, and the only one:** if open
    question 6 is answered "yes", pack 3's re-run sweep may change
    `FOUNDING_TEMPLATE`'s headcount, which changes the founder-eligibility bar in
    the decide and the `RoleSlot` in the apply. That is a founding-template
    re-freeze, not a re-design of phase 6, and it licenses nothing about the draw
    or its constants; the invitation at @src/market.rs:358–361 is borrowed for its
    **table and its sweep procedure only**. This must survive verbatim into the
    manifest: turning the draw off is the largest single
    t300 lever (pop 17 vs 10, closures 18 → 5) and **worthless** — pop 4 at t2000,
    pop 3 at t5000, 490 closures — because it *relocates* the one-way sink rather
    than removing it (firms hold **43.1%** of the supply at t1000 under draw-off
    against 1.0% under the null), and draw-off *plus* recycle is strictly worse
    than recycle alone at all ten rates 1–900‰, since an agent-wallet levy cannot
    reach a coffer. **The t300 number is not evidence for touching this phase.**
  - **`target_days` and every non-monetary re-tune.** Fourteen arms to t1000 —
    headcount to 30 slots, population cut to 21, `PRICE_FLOOR` 2 and 3, Food
    production 40→38, `UNEMPLOYED_SAVINGS` 0 and 10,000, wages −20%, five deeper
    retunes making 30 workers strictly required — **not one stable, eleven
    collapsing faster than baseline.** Full employment is reachable and
    unaffordable: 30/30 employed at t6 produces 1,511 g of arrears by t25 and 12
    employed by t75. Population 21 makes Food insolvent from tick 1.
  - **Population growth, new housing, new venues, and phase 6's untouched
    expand-capacity half.** Every stable configuration measured parks at exactly
    the seeded 30 residents with arrivals 0 and foundings 0 forever. Stability and
    growth are separate problems and only the first is solved here. The successor
    is named with a number, not a gesture — see **The successor, named**.
  - **A producer good / firm-to-firm demand in this milestone.** It was the
    panel's best *diagnosis* and its worst-evidenced *proposal*: zero of ~40,000
    measured ticks tested firm-to-firm demand, and its arithmetic does not close on
    the record — Food's margin at `PRICE_FLOOR` = 1 is roughly 20 g/tick against a
    280 g payroll, several times smaller than the input bill it would owe, after
    which the shipped fuse liquidates in `CLOSE_INSOLVENT_TICKS + 1` = 13
    arrears-ticks and a dead supplier halts *all* production — a failure mode
    faster than the null. It is the successor, gated on a **per-venue** headroom
    probe, not a town-wide one.
  - **Per-agent levy/disbursement events**, and any UI beyond one shell arm and one
    derived column. At 30 agents the per-agent form is 60 events/tick of noise and
    no criterion reads it.
  - **Non-gold metals in the recycle.** Silver and copper stay inert savings; the
    07-12 spec's reference-currency and exchange-rate questions stay open. Both are
    nonetheless covered by the new per-tick assertion (both deltas ZERO) — the
    per-metal completeness the totals-only audit cannot reach, and free.

### Amendments to the running series

*(continues the series at 20; 19 is executed. Two touch the 07-02 ⟨REF⟩ phase
table's **purpose** column only; two are outside the table and are numbered in
the same running series because that series has never been table-only — 14
widened `pay`, 15 made `TickReport` pure observation, 16 the grubstake, 17 the
settlement.)*

**Money-ops cells for rows 7 and 8 are unchanged.** The levy **is** a `burn`,
which row 7 pre-authorises without restricting the debited party and whose
future-mechanic cell literally names "demurrage"; the payout **is** a `mint`,
which is exactly and only what row 8 allows. But the claim that this makes the
amendment "free of capability" is refuted and is **not made here**: a row-8
*purpose* grant is the one grant that can create money, and §8.3 provably cannot
police it, whereas the declined alternative — widening row 7 for a conserved
agent→agent transfer — cannot create money at all, since `Accounts::transfer`
preserves `total_money` by construction. The grant this spec buys is on the
*worse* axis, and open question 1 puts that fork to the gate rather than burying
it.

- **Amendment 20** — row 8 (`mint_phase`), **purpose column only**: "new money
  from reserve" → "new money from reserve; and the phase-7 recycle levy's matched
  re-issue, which must equal that tick's levy exactly". Deliberately *not* worded
  "matched re-issue of a same-tick sink": that open form would let any future
  phase-8 mechanic mint on its own assertion that its issue is matched.
- **Amendment 21** — row 7 (`sinks`), **purpose column only**: "degradation,
  imports" → "degradation, the recycle levy, imports". The Amendment-18 precedent
  (a purpose-text honesty edit, money-ops deliberately untouched) is the shape.
- **Amendment 22** — supersedes the signed Contracts unit in the 07-19 pricing
  spec ("`mint_phase` becomes a stub … no money operation of any kind …
  Consequently `total_minted` changes only at worldgen and the §8.3 audit pins the
  supply at the seed forever"), quoted in full and superseded rather than left to
  diverge silently. The supersession also **corrects the retracted contract's
  reasoning, not merely its conclusion**: the audit never pinned the supply and
  cannot — `mint` moves the balance and `total_minted` together, so the identity
  holds for any mint. The faucet is closed today only because nothing calls `mint`
  at tick time. *The same misattribution is carried by @src/sim.rs:1448,
  @src/engine/worldgen.rs:370 and CLAUDE.md, and is corrected in the same pass.*
- **Amendment 23** — the 07-02 ⟨REF⟩ "Money entry/exit map", which today
  enumerates exactly two entries (the mint job; export receipts). Gains a third,
  named and closed: "the phase-7 recycle levy's matched re-issue in phase 8 —
  bounded by that tick's levy, so it changes `total_money` by zero". Without this
  the map is false the moment pack 2 lands.

**Not amended, and argued rather than assumed:** row 9 ("read-only / never gains
behavior") — the matched-issue check is a read-only assertion in `tick()` *beside*
the audit, not inside it, and the phase-9 function is untouched; if the gate rules
otherwise the amendment is cheap and nothing else moves. The ⟨REF⟩ "Each phase is
a plain `fn(&mut World)`" header line is likewise not amended: Amendment 15
already provided for phases taking `&mut TickReport`, and firm-lifecycle pack 1
already changed phase signatures under it; `pot` and the rate ride that provision,
and `sim::tick`'s own public signature is unchanged.
### The measured baseline this design must respect

Every figure here was produced by throwaway instrumentation on `town_world()` at
`ec1f818`, across roughly 40,000 simulated ticks, before a line of this design
was written — the discipline the last two packs established, applied to a
container premise for the first time. It **refuted that premise**, so the
refutations come first.

#### What the standing finding got wrong

CLAUDE.md has carried this since 2026-08-30:

> the residual is a **circulation** failure rather than a firm-count one: money
> is conserved, but `target_days` purchase caps mean a wallet above the cap is a
> sink that never returns, so the town starves beside its own gold.

The *conclusion* survives in a weakened form. The **mechanism is measurably
false**, and the remedy it implies is an accelerant.

- **No agent is ever blocked by the cap.** Two probes independently counted
  **0 of 7,074 agent-ticks** in which a solvent agent bought nothing because it
  was at cap, and 0 saturated agents at every sampled tick through t120 (the
  first nonzero reading is 4 agents at t220, when the town is already down to 17
  residents). Phase 5 consumes before phase 4 shops, so every agent re-enters the
  market below cap every tick and buys. The cap governs the **stockpile**, never
  the flow.
- **Relaxing the cap makes it worse at every setting.** `target_days` 7 →
  14/30/70/100/1000 gives t300 population **6 / 3 / 4 / 4 / 4** against the
  baseline's 10, and *raises* the largest wallet's share of the supply to
  35.3% / 64.6% / 51.0% / 99.3%. At ×50 the town holds zero businesses by t119.
  The cap is load-bearing in the good direction: it rations scarce stock toward
  the people who need it, and uncapping it lets rich wallets buy the shelves out
  and starve everyone else faster.
- **There is no aggregate deficit to close.** Over the healthy window t20–t120,
  household income averages **772.5 g/tick** against **772.3 g/tick** of
  purchases — balanced to 0.03% — and over 1000 ticks the household ledger closes
  to a **+0 g residual**. In a closed conserved economy firm revenue *is*
  household spend by construction. No mechanic can close a gap that is already
  zero.

#### What is actually wrong

**Bottom-tail solvency, not velocity.** Under the cure measured below, **94.0%
of the supply still sits in agent wallets above the per-tick spend ceiling** at
t300 and unchanged at t2000 — only 3.5 points below the null's 97.5% — and the
town holds 30 residents with zero closures. The cured town starves beside its own
gold exactly as much; it simply does not starve. The one variable that moves is
the bottom tail: **agents holding less than one tick's spend go from 3 of 10
(null, t300) to 0 of 30 at every horizon.**

**The cause is the production technology, not the money.** 30 residents, **21 job
slots**, and a technology that strictly requires only **22.5 workers** (7.5 per
good) to feed 30 people — 0.75 workers per capita. Nine residents therefore have
**no income source of any kind** and receive 0g of wages across 200 measured
ticks. They burn `UNEMPLOYED_SAVINGS` at a mean 20.8 g/head/tick, the fastest of the nine
at a measured **28.05 g/tick** over the 200 measured ticks — a *mean rate*, not a
tick amount, since money is integer — and within 0.2% of the **constants-derived
basket of 28** (Food 10×1 + Ent 5×2 + Lux 2×4 at the settled cheapest prices
1/2/4). **28 is the figure A4 gates on; 28.05 is the measured burn and is never
labelled with the constants expression.** First-departure tick is **linear in the
seed**: `t = 6 + savings/28.05` (fitted on the measured burn), measured across
nine values from 0 to 20,000. The entire
300-tick baseline is a savings-burn transient with a closure cascade bolted to its
end.

**The cascade triggers on effective demand, not headcount.** Food-sector revenue
dips below the sector's fixed 280 g payroll on the restock cycle from **t24**
onward with no consequence — 26 such ticks by t130, in a town still at 30
residents. What changes at **t128** is that the dips stop clearing: Longacre
Farm's coffer hits zero and its arrears streak runs unbroken t128 → t139
(`insolvent_ticks` 1 … 12), so phase 6 liquidates it at **t140** — the closure
tick `sim.rs`'s own pack-2 derivation predicted. Mid-streak at t130, 27 buyers out
of a tick-start population of 29 bought 267 g of food against that 280 g payroll;
the farm died holding **3,003 unsold food units at price 1**. *(This corrects the container ledger's "the first farm died at exactly
the tick population hit the derived break-even of 28": the farm died at t140 and
population fell below 28 only at t152, twelve ticks later. The break-even of 28 is
real, but it is 28 **paying customers**, not 28 residents.)*

**Spend is flat and wealth-independent; income is not capped.** At t100 the
richest agent (5,451g) and the poorest non-zero one (310g) each spent **exactly
280g** over the same ten ticks. Six survivors spent 10,694–10,819g each over 300
ticks — within 1.2% — across incomes from 31.9 to 84.5 g/tick. Gold accumulates in
whichever account has no spending rule proportional to its balance: owners
**+19.12** g/agent-tick, employed non-owners **+4.41**, the jobless **−20.13**.

#### Levers measured and refused

| Lever | Settings tried | Result |
|---|---|---|
| Demurrage as a **sink** | 46 configs — 1–100‰, burned vs →External, agents / agents+coffers / above 10×–1000× thresholds, 300 and 600 ticks | **None clears the bar.** Best (burn 1‰) is t300 pop 11 vs 10, and t600 pop 3 vs 4 — a delay costing 20.8% of the supply. The idle gold *is* the savings the jobless live on. |
| **Exports** at the shipped seam | 1 unit/firm → half of all stock | Inert: External holds **600g total**, moves 623–796g across 300 ticks in every setting, starves 241–300 of 300 ticks. The seam is not underpowered, it is **unfunded**. |
| Exports with deep pockets | External pre-seeded +100,000g | Helps, never stabilises (t300 pop 15, 6 firms). |
| Exports, mint-backed | 20 and 40 units/firm/tick | **Lethal.** Outside demand outbids residents, `sold_out` fires every tick, `adjust_price` ratchets with no ceiling: Food 179g at t100, no Food seller by t200; at 40 units the price series diverges to 3.3 × 10¹⁰/unit. |
| **Imports** as pure outflow (control) | 1/5/20 g/agent/tick; 1/100 and 1/20 wallet-proportional | Monotonically harmful, as a control should be. 20g extinguishes the town; 1/20 kills it by t83. |
| Profit draw **off** | — | Best single lever at t300 (pop 17 vs 10, closures 18→5) and **worthless**: pop 4 at t2000, pop 3 at t5000. It *relocates* the one-way sink — firms hold **43.1%** of the supply at t1000 against 1.0% under the null. Draw-off *plus* recycle is worse than recycle alone at all ten rates 1–900‰, because an agent-wallet levy cannot reach a coffer. |
| **Non-monetary re-tunes** | 14 arms to t1000: headcount → 30 slots, population → 21, `PRICE_FLOOR` 2 and 3, Food production 40→38, `UNEMPLOYED_SAVINGS` 0 and 10,000, wages −20%, five deeper retunes making 30 workers strictly required | **Not one is stable; eleven collapse faster than baseline.** Full employment is reachable and unaffordable: 30/30 employed at t6 produces 1,511g of arrears by t25 and 12 employed by t75. Population 21 makes Food insolvent from tick 1 (revenue 210 vs payroll 280). |

**Pinned, so it is never re-litigated:** the phase-6 profit draw is **not** the
disease and must not be touched by this container. Its t300 number is a trap — the
t2000 number reverses it.

#### The one arm that worked, and what it costs

| | null baseline | conserved recycle @ 20‰ |
|---|---|---|
| population t300 / t1000 | 10 / 4 | **30 / 30** |
| live firms t300 / t1000 | 3 / 4 | **6 / 6** |
| closures · departures · quits, to t300 | 18 · 20 · 49 | **0 · 0 · 0** |
| largest wallet, share of the 52,148 g supply | 27.8% (t300), 58.0% (t1000) | **~6.2%** |
| agents below one tick's spend | 3 of 10 (t300) | **0 of 30, every horizon** |
| supply above the per-tick spend ceiling | 97.5% | **94.0%** — barely moved |
| horizon at which it was confirmed | — | **bit-identical t600 → t10000** (probe arm; not inherited under this spec's remainder rule — see `mint_phase`, A3) |

Per tick in the cured steady state, identical at t500, t2000 and t5000: **68–82
`Sold` events**, 632–912 g of goods turnover, 662–702 g of wages, 62–138 g of
draws, 508 units produced, ~987 g/tick of levy. Per-agent (gold, food) vectors
repeat exactly at **lag 10** and never at lag 1/2/5 — a limit cycle, not a
freeze. Hunger is a **warm-up transient**: 68 events, all between t2 and t14,
zero from t15 to t600, every counter at 0 thereafter.

**And what it does not do, stated at the top rather than discovered later.**
The cure stabilises a town in which **nine adults live on a permanent transfer**
— they receive 0 g of wage-or-draw income and are fed entirely by the dividend.
It also puts the *just-shipped firm lifecycle to sleep*: `plan_founding`'s two
tiers require 0 or 1 sellers of a good, and the cured six-venue town holds
exactly **2 sellers of every good on all 600 measured ticks**, so it returns
`None` every tick while an eligible founder and a vacant house exist on 600 of
600; phase 1's immigration pull is dormant for a different reason (1 `Arrive`
decided, 0 applied in 600 ticks — the town's own nine unemployed take every
vacancy before `unfilled_ticks` reaches `VACANCY_PULL_TICKS`); and External's
gold has not moved since genesis. Under shock the lifecycle **does** fire and does
real work — with founding disabled the same shock leaves pop 6 and 0 firms
against pop 30 and 6 firms with it — but it **under-replaces**: the Food founding
template posts headcount 2 against the 4-headcount venue it replaces, so every
phoenix cycle permanently costs two jobs, drops Food output 320 → 240 units/tick
against 300 consumed, and installs ~6–8 hungry agent-ticks per tick that the
scarcity gate can never answer because at 2 sellers it is unreachable. Repeated
shocks ratchet the town monotonically to pop 24 / employed 15 and it never
regrows. Acceptance criteria A11 and A12 charge for all of this; pack 3 attacks
the under-replacement.

**n = 1, and there is no derivation rule.** Every figure above comes from one
scenario, `town_world`, at one wealth distribution (30 residents / 21 slots / 6
venues / 52,148 g / 3,400 g unemployed savings; the seeded full-staffing bill is
**676 g/tick** — 8×35 + 7×36 + 6×24 — and the 682 g figure quoted by one probe is
a post-tâtonnement reading once wages have moved). At a near-constant base a proportional rate *is* arithmetically a fixed
~33 g/head/tick transfer: it does not scale with prices, population or horizon.
What it does that a minted stipend cannot is **mint nothing net**, so it never has
to outrun a base it is itself inflating — that, not proportionality, is why the
stipend's sufficient value climbs 23 → 27 and this one does not. A new seed
requires re-running the sweep; the spec claims no scaling rule the evidence does
not contain.

### The successor, named with a number

Growth is a separate problem and this milestone does not touch it. The successor
is **producer-side demand and phase 6's untouched expand-capacity half**, and the
record sizes it — **on the trajectory this milestone actually ships**, which is
not the trajectory the tempting figure comes from. Cured, the profit stream is the
steady state's **62–138 g/tick of draws** (identical at t500, t2000 and t5000),
worth **2.2–4.9 livelihoods** at the 28.05 g measured burn: roughly a third of the
nine residents who hold no job. **On the pre-cure (null) trajectory** the same
stream was 221 g/tick (66,404 g of draws ÷ 300 ticks) to 278 g/tick (772.5 income
− 494.2 wages over the null's healthy window t20–t120 — two incommensurable
windows, deliberately *not* averaged), i.e. 7.9–9.9 livelihoods. **That parity is
what made the successor look self-funding, and it does not survive the cure**,
because cured wages run 662–702 g/tick against the null window's 494.2. So the
successor must find roughly two thirds of its wage bill somewhere other than the
existing draw stream: size-parity between un-needed labour and un-spendable profit
is **measured false on the shipped town**, not merely unmeasured. Where the dead
money ends up on the null trajectory (t20–t120): owners accrete at +19.12
g/agent-tick, employed non-owners at +4.41, the jobless at −20.13, while the
richest agent (5,451 g) and the poorest non-zero one (310 g) each spent **exactly
280 g** over the same ten ticks.

### Proposed pack sequence (the container manifest is written from this)

**Pack 1 — the wrappers, the helper, and the null seam (zero behavior change).**
`World::levy` and the pot-scoped `World::disburse` with the new
`WorldError::OverIssue { requested, remaining }` and its `Display`;
`sim::levy_amount` and `RECYCLE_PERMILLE`; `sim::tick_with_rate` introduced with
`tick` delegating to it, the rate threaded but **not yet read by any phase**;
`#[allow(dead_code)]` retired from `Accounts::burn`. Unit tests only, all outside
the sim — the narrow id rule in both directions, the zero no-ops, insufficient-funds
atomicity, `OverIssue` atomicity, and `levy_amount`'s property table including the
49/50 boundary. Nothing is wired into a phase, so no scenario behavior can move. Also plants the
container's 🚧 pointer in `docs/INVENTORY.md` (the 08-21 precedent: the pointer goes
in at container start, and is folded by the pack that closes the container).
*Gate:* `verify.sh` green, and the three town soaks plus every genesis assertion
**bit-identical** to the pre-pack build — event streams diffed, not merely green,
because a pack gated as "zero behavior" must prove it.

**Pack 2 — the recycle runs, and every artifact it invalidates is re-cut in the
same commit.** *Item 1 (the bisection point and the null proof):* wire both legs
at `RECYCLE_PERMILLE = 0` — `sinks` returns the pot, `mint_phase` spends it,
`tick_with_rate` threads it, `Event::Recycled` plus its one shell arm and the
derived `net` column, and the per-tick matched-issue assertion. Mechanism live,
magnitude zero; every existing test and all three soaks unchanged and green. *A
sink shipped without its matched issue is the measured-harmful arm and must never
be a releasable state* — hence both legs together — but rate 0 gives the successor
a clean bisection handle; it does **not** discharge A9, which item 2's freeze at
20 erases and item 3's twins make durable. *Item 2:* freeze the constant at 20
with the sweep justification recorded **and the share-vs-basket account recorded
beside it as refuted**, plus the n=1 note. *Item 3:* the invalidated acceptance
artifacts, re-cut **in this commit** with the invalidating measurement quoted for
each (see **Migration impact** — this is the item that keeps a self-invalidated
soak from being discovered red under gate pressure and quietly re-cut to fit).
**Each of the three town soaks is FIRST copied verbatim — pre-cure body, pre-cure
criteria — into a `#[cfg(test)]` rate-0 twin calling `tick_with_rate(world, 0)` in
place of `sim::tick`, and only then is the original re-cut.** Those twins are the
artifact A9 gates, and they are why the pre-cure trajectory stays executable in the
shipped tree rather than surviving only in git history. Duplicating the migration
soak's ~250 lines is the accepted price: A9 says *original form*, and parameterising
one body over the rate would make the criterion set rate-dependent — the
re-cut-to-fit failure this item exists to prevent. *Item 4:* Amendments 20–23 executed in the same edit, including the supersession
text and the counter-doc reinterpretation **with its written expiry**. *Item 5:*
discharge the `CLOSE_INSOLVENT_TICKS` standing obligation with the number quoted.
*Item 6 (recorded, not gated):* the ledger probe — per-tick pot, per-head share,
agent-held gold as a share of 52,148, and the count of residents receiving zero
wage-or-draw income over each rolling 50-tick window, expected **9**. *Item 7:*
the fixed-point/volume soak to t5000 carrying A1–A8 and A10, **and A9 on item 3's
rate-0 twins**. *Gate:* `verify.sh`
with real output quoted, plus a **per-soak verdict** — never the blanket claim
"the three town soaks pass unchanged". Expected shape, stated in advance so a
surprise is a finding: the 100-tick soak passes with criterion 2 re-measured (its
coffer-at-buffer bound is untouched — the levy takes only from agent wallets and
phase 6's draw still runs before phase 7); the 50-tick employment soak passes
unchanged; the 200-tick migration soak passes only in its re-cut form.

**Pack 3 — shock recovery, lifecycle reachability, and the founding-template
sweep re-run (independently shippable).** The criteria that cannot pass on a
corpse, plus the one defect the cure exposes. A new soak runs the cured town to
t500, force-closes one Food seller via `World::close_business`, and asserts A11
and A12. It carries the under-replacement finding rather than tolerating it — and
the fix is a **re-run sweep, never an asserted 2→4 correction**: `market.rs`'s own
table comment records that headcount 3 was *measured and failed* the then-signed
anti-churn criterion and that 4 was "not run", and it carries a standing
invitation to "start from this table and re-run the sweep against the corrected
criterion". This pack invokes that invitation on the **cured** town — the prior
sweep was run on the sick one — and quotes the table. Also folds
`docs/INVENTORY.md`'s pointer into a real entry and updates CLAUDE.md. *Gate:*
`verify.sh` green, the recovery window K measured then pinned, the sweep table
extended with its new column. **Independently droppable and honest about it:**
packs 1–2 stand without this one — in which case the INVENTORY fold and the
CLAUDE.md structure delta move to pack 2's gate, since the fold belongs to
whichever pack closes the container. And if the sweep finds no headcount satisfying
both the recovery clause and the anti-churn criterion, the recovery criterion is
**formally weakened at the gate with the measurement quoted** and the ratchet
recorded as a known live defect handed forward — never silently relaxed, and
never re-cut to fit the result.

## Contracts (the hard reference the plan is written from)

### World::levy  (world.rs, new)

Signature: `pub fn levy(&mut self, from: AgentId, metal: Metal, amount: Money) -> Result<(), WorldError>`
Given:  `from` is a **spawned agent** — `self.agent(from).is_some()`, deliberately
        narrower than `is_known_account` (@src/world.rs:216), which also admits
        `mint_id`, `external_id` and every live business id — validated FIRST,
        nothing touched before the check (the 07-03 command-layer property that
        `Err` always means nothing changed).
Then:   forwards unchanged to `Accounts::burn`, which debits the balance and logs
        `total_burned(metal)`. `Money::ZERO` is an `Ok` no-op creating no account
        entry (burn's own zero guard). This is the missing command-layer wrapper
        for the burn half of §8.2 — `World::pay` is today the only wrapper over the
        chokepoint and there is none for `burn` or `mint`. Landing it retires
        `#[allow(dead_code)] // the sinks phase (7) lands later` (@src/money.rs:218).
Error:  `WorldError::UnknownAgent(from)` for a reserved id, a live business id or a
        ghost id — nothing burned, books provably untouched.
        `WorldError::Money(MoneyError::InsufficientFunds)` forwarded unchanged,
        nothing applied (§8.5).
Refs:   §8.2, §8.5, @src/money.rs:218, @src/world.rs:216, @src/world.rs:228.

**Recorded rejection, so the plan does not "simplify" it back:** reusing
`is_known_account` was refused. Households-only is the design. **Arm-labelled, because the two arms differ by
~5×:** the split *agents 97.9% / firms 0.8% / External 1.3% / Mint 0* (denominator
`total_money(Gold)` = 52,148) is the **null** arm at **t300** — a collapsed
3-firm, 10-resident town — and is not the base this ships against. Under the cure
the levy base is the 49,520 g of agent-held gold used throughout this spec, which
leaves roughly **2,028 g ≈ 3.9% of the supply permanently outside the levy**: six
coffers at `DRAW_BUFFER_BILLS` × the 676 g full-staffing bill, plus External's
unmoved 600 g. **That ≈3.9% is derived from this spec's own base and phase order,
not separately measured** — pack 2 item 6's ledger probe records the firm share
beside agent-held gold and pins it, and a divergence is a finding. Coffers cannot
in fact sit at the null's 0.8% under the cure: phase 3 pays wages before phase 4
revenue arrives, so a coffer stock below its share of the ~676 g payroll would
post arrears every tick. Even at 3.9% a coffer levy has little to bite; it would perturb `draw_amount`'s retained buffer and the
`CLOSE_INSOLVENT_TICKS` fuse that reads `owed_total()` directly; and reaching into
coffers via draw-off was measured worthless by t2000. **This is stated as a
decision on partly unmeasured ground — no firm-side variant was measured — not as
mechanics** (open question 3).

### World::disburse  (world.rs, new)

Signature: `pub fn disburse(&mut self, to: AgentId, metal: Metal, amount: Money, pot: Money) -> Result<Money, WorldError>`
Given:  `to` is a **spawned agent** (the same narrow rule as `levy`: a recycle
        share never lands on a coffer, the Mint or External), validated FIRST; and
        `amount <= pot`, where `pot` is the *remaining unissued* pot.
Then:   credits `to` via `Accounts::mint`, logging `total_minted(metal)`, and
        returns `pot.minus(amount)` — the remaining pot, which the caller threads.
        `amount == Money::ZERO` returns `Ok(pot)` **without** calling `mint`:
        `Accounts::mint` has no zero guard and would insert a zero-balance entry,
        unlike `transfer`, so this wrapper restores the no-entry-on-zero property
        at the command layer.
Error:  `WorldError::UnknownAgent(to)`.
        `WorldError::OverIssue { requested: Money, remaining: Money }` (new
        variant) when `amount > pot` — **nothing minted**.
Refs:   §8.2, §8.4, §8.1, @src/money.rs:133–141, Amendment 20.

**Why pot-scoped, and why the obvious three-argument form is refused:**
`Accounts::mint` is infallible — it returns `()` and cannot refuse — with its
gold-reserve cap deferred in its own doc comment, and the §8.3 audit cannot detect
a mint at all, since `mint` raises the balance and `total_minted` together so the
identity holds for *any* mint, matched or not. An unscoped `disburse` would leave
"the faucet can never out-run the same tick's drain" as prose plus one soak
assertion on one scenario at one horizon — the exact class of unenforced property
this design elsewhere refuses. Pot-scoping gives the faucet **a refusal point the
trusted core structurally lacks** — `disburse` returns a `Result` where
`Accounts::mint` returns `()` — and localises the remaining pot in one function
that can say no. **It does not by itself make `sum(disbursed) <= pot` a
command-layer invariant, and is not claimed to:** `pot` is an argument, not `World`
state, so a caller re-passing the original pot on every call would be accepted every
time. The unconditional guard against exactly that bug is `tick`'s per-tick
`Δtotal_minted(Gold) == Δtotal_burned(Gold) == pot` assertion, which reads the §8.4
logs rather than the threaded value and therefore fires however the thread is
mis-written — on every tick of every scenario, which is what distinguishes it from
the "prose plus one soak" shape refused above. Pot-scoping is what makes that
failure *local and refusable at the call site* instead of only detectable at the end
of the tick. Holding the remaining pot as `World` state for the phase's duration —
which would make the bound structural — is **refused**: it buys stronger wording
with new mutable state in the command layer, a new single-writer obligation and a
cross-phase reset path, against this design's no-new-persistent-state claim.
`disburse` is also the single place a future gold-backing cap would be enforced.

### sim::RECYCLE_PERMILLE + sim::levy_amount  (sim.rs, new)

Signature: `pub(crate) const RECYCLE_PERMILLE: u64 = 20;` ·
           `pub(crate) fn levy_amount(balance: Money, permille: u64) -> Money`
Given:  a gold balance and a per-mille rate.
Then:   returns `balance.times(permille as u32).divided_by(1000)` — flooring
        integer arithmetic on the existing checked helpers, §8.1, no floats.
        Properties, each unit-pinned: `levy_amount(b, p) <= b` for every
        `p <= 1000` (the property phase 7 relies on, §8.5); `levy_amount(b, 0) ==
        Money::ZERO` for every `b` (the null-rate property A9 rests on); at 20‰,
        `levy_amount(49) == ZERO` and `levy_amount(50) == Money::new(1)` — the
        integer floor is a means test at 50 g, exempting the bottom tail **by
        construction rather than by a guard**. Pure, total and scalar-taking, with
        a unit-test home outside the sim (the `draw_amount` / `insolvent_now`
        precedent). It lives beside its phase and **not** in `market.rs` — §8.6
        reserves that module for pricing and wage logic and a levy is neither (the
        `QUIT_ARREARS_BILLS` / `DRAW_BUFFER_BILLS` placement precedent, stated so
        the plan never re-litigates it). `pub(crate)` so the soaks name the same
        constant rather than a re-spelled copy.
Error:  none — total; panics only through `Money`'s own checked ops.
Refs:   §8.1, §8.5, §8.6.

**FROZEN at 20 on the direct sweep alone:** 15‰ is the measured minimum sufficient
magnitude; 20‰ is an exact fixed point, bit-identical t600 → t10000; payroll
shortfalls 395 per 2000 ticks at 20 against 3,227 at 15 — 8.2× lower residual
stress.

**RECORDED AS REFUTED beside the constant, in these words, because it is the rule
a successor would re-tune with:** the account that "20‰'s ~33 g/head share clears
the 28.05 g basket while 15‰'s ~25 g does not" is **contradicted by the record,
not merely unmeasured**. On a base of **49,520 g of agent-held gold — the
constants-derived genesis wallet total (16 × `EMPLOYED_WALLET` 120 + 14 ×
`UNEMPLOYED_SAVINGS` 3,400), not a per-tick measurement of the cured arm** (that
series is item 6's probe) — 95.0% of 52,148, corroborated at that magnitude by the
measured ~987 g/tick pot at 20‰ and its ~33 g/head share — the share-clears-burn
crossing is at **16.99‰, two per-mille above the measured sufficiency threshold of
15**; the same derivation on the other spend branch (36.0 g) demands 21.81‰ and
would declare the frozen 20 insufficient. **Neither conclusion depends on the
base:** even at the arithmetic maximum, agents holding the whole 52,148 g supply,
the crossing is 28.05 × 30 / 52,148 = 16.14‰ (still above 15) and the 36.0-branch
figure is 20.71‰ (still above 20), so any error in the base leaves the refutation
standing. **No
mechanism for why the threshold sits between 15 and 17 is established** (most
plausibly a rationed spend well below the full basket, since Food sits at
`PRICE_FLOOR` = 1 for 85 of the first 100 ticks) — recorded as *unexplained*, not
explained. Therefore the per-agent spend figure is **not a tuning input at all**;
it enters only as an acceptance denominator, where the design figure is the
constants-derived **28** (Food 10×1 + Ent 5×2 + Lux 2×4) and the 36.0 g survivor
lifetime average is rejected as survivorship-biased across a declining town's
ratcheted prices.

### sim::sinks  (sim.rs, amended)

Signature: `fn sinks(world: &mut World, report: &mut TickReport, permille: u64) -> Money`
Given:  the world at phase-7 entry.
Then:   in this strict and load-bearing order — **(a)** the existing emigration
        decide/apply runs first and completely unchanged: the `cheapest_food`
        snapshot and its no-Food-seller guard, the `Intent::Depart` decide over
        `world.agents` order, `apply_sinks_intent`, the Amendment-17 settlement,
        the Amendment-19 forced liquidations and the per-metal sweep to External;
        **(b)** then the levy pass — a DIRECT pass with no intents (the
        `pay_wages` / profit-draw precedent: objective per-agent state, zero
        contention), over `world.agents` order, ascending `AgentId` by
        construction, no RNG and no `HashMap` iteration. For each surviving agent:
        `world.levy(agent, Metal::Gold, levy_amount(balance_of(agent, Gold),
        permille))`, accumulating the pot, which is **returned** — the sole
        cross-phase datum. Gold only (the sole trading metal; the phase-6 draw
        precedent). Households only, structurally, via `levy`'s id rule. Emits no
        event of its own: the recycle is narrated once, by phase 8. Levying
        strictly AFTER emigration means a departing agent is never levied — their
        gold reached External inside `remove_agent` before this pass — so there is
        no ghost payee and no coin taken from someone absent from phase 8's roster.
Error:  none returned. Each `World::levy` call is infallible by construction (the
        id came from `world.agents`; the amount came from that agent's own balance
        via `levy_amount`, which never exceeds it), so an `Err` is an
        `expect`-level bug and is written
        `.expect("a floor-fraction of the live balance of a live agent")` — never
        `let _ =` and never `.is_err()` (@src/sim.rs:1402, the recorded erratum).
Refs:   07-02 row 7 (Amendment 21), §8.2, §8.5, @src/sim.rs:1342.

**The ordering ruling, with its counter-case recorded rather than left as an
artifact of who typed it:** the alternative places redistribution BEFORE
destitution is adjudicated, on the principle that a policy meant to prevent
destitution should apply before the sim judges someone destitute. **That is not
available here** — the payout leg is a `mint` and only row 8 permits one — so the
consequence is stated plainly: the destitution decide reads **pre-dividend**
wallets, and an agent can be swept to External at tick t phase 7 while the share
that would have cleared the cheapest Food price arrives at t phase 8. Unobservable
in the cured steady state (0 departures at every horizon), therefore pinned by
A11's ordering clause rather than by argument. It is **not a measured divergence** from the
single-phase transfer expression — the arm measured tick-for-tick identical to
t10000 placed its redistribution at this same point, below the emigration decide,
so both expressions read pre-dividend wallets. What the split costs is the
**option**: a transfer expression *could* be placed above the decide and this one
structurally cannot, since only row 8 permits the payout leg. Open question 1
weighs that foreclosed option, not an observed difference.

### sim::mint_phase  (sim.rs, amended — the stub wakes)

Signature: `fn mint_phase(world: &mut World, pot: Money, report: &mut TickReport)`
Given:  `pot` — exactly what phase 7 burned this tick — and the world, with
        **nothing** having run in between except `tick()`'s own control flow, so
        the roster is provably the one phase 7 levied and no roster snapshot need
        be carried.
Then:   if `pot == Money::ZERO` or `world.agents.is_empty()`, mints nothing and
        emits nothing (the empty-roster guard also keeps `divided_by` away from a
        zero divisor, which panics — @src/money.rs:60). Otherwise
        `heads = world.agents.len()`, `share = pot.divided_by(heads as u64)`
        (flooring, §8.1), `remainder = pot − share × heads` (strictly `< heads`);
        every agent in `world.agents` order receives `share` through
        `World::disburse`, threading the returned remaining pot, and each of the
        FIRST `remainder` agents in that same order receives one extra unit; the
        thread must end at exactly `Money::ZERO`, and that is asserted. Then
        `sum(disbursed) == pot` **exactly**, so `total_money(metal)` is
        bit-identical across the tick and `total_minted − total_burned` is
        unchanged. The remainder is distributed rather than dropped because
        dropping it would silently deflate the supply by up to 29 g/tick — up to
        290,000 g over 10,000 ticks against a 52,148 supply — **while still
        passing the audit**, precisely the class of bug §8.3 cannot see. Emits
        exactly one `Event::Recycled { pot, heads, share }`. The remainder
        tie-break is lowest `AgentId` — deterministic, no RNG.
Error:  none returned; a `World::disburse` refusal on an id taken from
        `world.agents` within a threaded pot is an `expect`-level bug, written as
        such.
Refs:   07-02 row 8 (Amendment 20), Amendment 22 (this retracts the 07-19 stub
        contract), Amendment 23, §8.4, Amendment 15.

**NAMED AS UNMEASURED, and it changes what may be claimed:** the record does not
state how the winning probe divided its pot, so this lowest-id remainder rule is
this spec's own and may differ from the measured arm by up to 1 g/head/tick
against integer prices of 1–5 g and a 28 g basket. **This spec therefore does not
inherit the bit-identical t600 → t10000 claim for its own arm**: pack 2 *measures*
the fixed point and its exact period under this rule and pins those numbers,
recording any divergence in the ledger as a finding.

### sim::tick + sim::tick_with_rate  (sim.rs, amended)

Signature: `pub fn tick(world: &mut World) -> TickReport` ·
           `pub(crate) fn tick_with_rate(world: &mut World, permille: u64) -> TickReport`
Given:  a tick.
Then:   `tick` keeps its ⟨REF⟩ signature verbatim and delegates —
        `tick_with_rate(world, RECYCLE_PERMILLE)`. `tick_with_rate` runs the
        unchanged fixed 9-phase order, with phase 7 as
        `let pot = sinks(world, &mut report, permille);` and phase 8 as
        `mint_phase(world, pot, &mut report);` — the two calls adjacent, nothing
        between. Before phase 7 it records `total_minted(m)` and `total_burned(m)`
        for every `Metal::ALL` entry; after phase 8 and IMMEDIATELY BEFORE
        `world.accounts.audit()` it asserts, per metal: for Gold,
        `Δtotal_minted == Δtotal_burned == pot`; for every other metal, both
        deltas are `Money::ZERO`. Panics on failure, like the audit, never softened
        to a `Result`, and never placed **inside** phase 9 (row 9 stays read-only
        and behavior-free — no amendment is spent there).
Error:  panics by design.
Refs:   §8.3, §8.4, @src/sim.rs:232, Amendments 20/22/23.

**This assertion is the milestone's substitute for the deferred gold-backing cap,
and it must live here rather than in a soak** because the §8.3 audit provably
cannot detect a mint: `Accounts::mint` raises the credited balance and
`total_minted` by the same amount, so the identity holds for a matched re-issue,
an unmatched one, a dropped pot, or a share computed from the wrong denominator
alike. Today's faucet is closed because **nothing calls `mint` at tick time** —
not because the audit pins it, a misattribution carried by @src/sim.rs:1448,
@src/engine/worldgen.rs:370 and CLAUDE.md and corrected by Amendment 22.

**`tick_with_rate` is the null seam**, `pub(crate)` and test-reachable: it makes
"rate 0 reproduces the pre-cure trajectory bit-identically" an *executable*
criterion rather than a claim no build can hold alongside the cured default, and
it is the migration path if a successor ever turns the constant into policy data.
No new persistent state: the rate is a parameter, the pot a local.

### sim::Event::Recycled  (sim.rs, extended)

Signature: `Recycled { pot: Money, heads: u32, share: Money }`
Given:  a tick in which the recycle moved anything.
Then:   exactly one of these, emitted by `mint_phase`, carrying the pot burned,
        the number of payees and the per-head share; the remainder is implied by
        `pot − share × heads` and is deliberately not a field. A zero-pot tick
        emits nothing (the held-price and zero-draw precedents). **Aggregate rather
        than per-agent:** at 30 agents the per-agent form is 60 events/tick, no
        acceptance criterion reads it, and it would drown the shell's aggregated
        feed — a granularity ruling later mechanics will copy, so it is made
        deliberately. Data-only (Amendment 15); the amounts are what the command
        actually moved, never a re-derivation.
Error:  n/a (data).
Refs:   Amendment 15, @src/sim.rs:72.

**Compile-time forcing is asymmetric and is stated so the plan does not assume
help it does not get:** the shell's exhaustive `match event` IS forced to grow one
arm, but worldgen's soak `match event` arms end `_ => {}` and force nothing, so
every soak tally over this variant is **hand-written** — the pack-2 zero-closures
and pack-3 zero-`Founded` precedents.

### Mechanical / obvious (plan owns these — no contract needed)

- The `WorldError::OverIssue { requested, remaining }` `Display` arm and its
  placement in the enum — follows the existing variants' style.
- The shell's one new `match event` arm, aggregated beside the existing draws
  line, and the derived `net = minted − burned` column in the per-metal money
  summary (@src/engine/game_loop.rs:137). One line each.
- Retiring `#[allow(dead_code)]` at @src/money.rs:218 and re-wording the
  `total_minted` / `total_burned` doc comments to the agreed "gross lifetime
  credits/debits through the §8.4 logged paths" **plus the expiry note**. Text
  only; no signature moves.
- The accumulate-and-return plumbing inside the levy pass and the pot-threading
  loop inside `mint_phase` (including the first-`remainder`-agents extra unit) —
  obvious once `levy_amount`, `levy` and the pot-scoped `disburse` are fixed.
- The ledger-probe instrumentation for pack 2 item 6. A test-local harness that
  records and prints; it asserts nothing, so it needs no contract — but it must
  print **before** any assertion (the `PACK3 SOAK:` println precedent), so a red
  run still says what it measured.
- The mechanical re-pins inside pack 2 item 3's re-cut tests once their criteria
  are decided: literal expected values, `use` lines, and the hand-written event
  tallies. **The decisions in those re-cuts are pack-2 gate material, not
  mechanical, and are diffed line by line at the gate.**

## Migration impact (breaks existing shipped code)

The cure does not merely add behavior; it **falsifies the premise of shipped,
signed acceptance artifacts**. They are listed here, with the measurement that
invalidates each, so pack 2 re-cuts them deliberately in the commit that lands the
mechanic rather than discovering them red under gate pressure — the one way a
regression gets laundered. **The list is complete over the three town soaks and all
seven `tick()` call sites in `src/sim.rs`;** anything outside those two surfaces is
fixture-frozen and cannot see the levy.

1. **`town_soak_population_moves_both_directions`** (@src/engine/worldgen.rs:775)
   runs `town_world()` through 200 real `sim::tick` calls, so it runs the **cured**
   town, not a fixture. Under the cure it fails on five assertions and *panics* on
   a sixth:
   - `assert!(!departed_ids.is_empty(), "nobody left in {LAST} ticks")` (:896) —
     0 departures measured at every horizon. **This spec contains, in A5/A4 and
     here, an acceptance criterion that is the exact logical negation of a shipped
     assertion; saying so out loud is the point of this section.**
   - `first_answer_after_departure.is_some()` (:898) — no departure, no answer.
   - `assert!(dipped, …)` (:902) — population never falls below the seed count.
   - `total_births >= 3` (:930) — `plan_founding` returns `None` on 600 of 600
     ticks (2 sellers of every good, always).
   - `chain_reoccupied.expect("no freed house was ever founded into")` (:961) —
     zero closures means no house is ever freed: this **panics**, not even a clean
     assert.
   - `closed.len() >= 3` (:1000) — zero closures.

   Its entire decay-driven premise is structurally gone. It is converted to
   **shock-driven** and diffed line by line against its original criteria at the
   gate, each surviving criterion re-mapped to its shock analogue and each retired
   criterion accompanied by the measurement that retired it.
2. **`mint_phase_creates_no_money`** (@src/sim.rs:1836) no longer compiles (a
   one-argument call to a three-argument fn) and no longer means what it asserts.
   Re-cut to assert the **matched-issue** property instead of a closed faucet.
3. **The `total_minted(Gold) == Money::new(105)` assertion** inside
   `minimal_economy_feeds_the_worker_and_the_idle_leaves_town`
   (@src/sim.rs:3078) is taken **after ten real ticks**, not at t0, and its comment
   ("the audit pins it there every tick") carries the misattribution Amendment 22
   corrects. It moves the moment any agent in that fixture crosses the 50 g levy
   floor — pack 2 measures whether it does and records the tick. Either way the
   assertion's *meaning* is retracted and it is restated as `total_money(Gold) ==
   105` plus `minted − burned == 105`.
4. **The 100-tick soak's criterion 2 price clauses** (@src/engine/worldgen.rs:556:
   not floor-pinned all span, not monotonically rising) are **re-measured** under
   the cure and re-pinned or re-argued. Raising the bottom tail's purchasing power
   against a ceiling-less `adjust_price` is exactly the pressure that clause tests.
5. **`Accounts::burn`** loses `#[allow(dead_code)]` — a non-test caller finally
   exists. Its unit tests stand unchanged.
6. **The two multi-tick founding fixtures in `sim.rs`** —
   `a_single_seller_closure_with_live_demand_founds_within_a_window`
   (@src/sim.rs:2477) and `a_demand_death_founds_nothing` (@src/sim.rs:2573) — each
   drive 12 real `sim::tick` calls, so like item 1 they run the **cured** sim
   rather than a frozen fixture. With item 3's `minimal_economy_…` these are the
   only `sim.rs` tests that tick a *funded* world: of the seven `tick()` call sites
   in the file the remaining four are empty-world or single-tick with every wallet
   under the 50 g floor, so they take no levy and no share. Both of these hold
   agents far above the floor, so both are levied and both receive phase-8 shares.
   **Named as unmeasured: pack 2 measures both, quotes the measurement, and re-cuts
   only what moves.**
   - `a_demand_death_founds_nothing` is the sharper of the two, and its **premise**
     is at risk, not merely its pins. Its negative arm constructs "no demand" by
     leaving the buyers' purses empty — but `mint_phase` pays an equal share to
     *every* agent in `world.agents`, penniless ones included, so under the cure an
     empty purse is no longer a proxy for absent demand. Dividends alone could
     clear the one-unit shelf, accumulate `sold_out_ticks`, and fire the `Founded`
     the test forbids. If it moves it is re-cut the way item 1 is — demand death
     re-expressed against the dividend floor — with the retiring measurement
     quoted, never re-tuned to fit.
   - Its founder is seeded at **exactly** the founding bar
     (`stake.plus(FOUNDER_RESERVE)`), so the control arm rides on the sign of
     `share − levy_amount(410, 20)`. Pack 2 records that sign rather than assuming
     it either way.
   - `…founds_within_a_window`'s four-tick band is a timing pin on price ratchets
     and sell-out streaks, both of which move with bottom-tail purchasing power.
     Its closure leg is preset via `insolvent_ticks` and is not at risk; the
     founding leg is.
7. **`FOUNDING_TEMPLATE`'s headcount-2 freeze** (@src/market.rs:341–372). Its
   recorded justification is "the criterion is signed, so 2 ships" — and that
   criterion is the 200-tick migration soak's anti-churn clause, which item 1
   re-cuts, while the whole sweep was run on the sick town. **The freeze's basis,
   not its value, is what the cure invalidates.** Pack 3 re-runs the sweep on the
   cured town and extends the table; if pack 3 is dropped, the freeze stands with
   this note recorded against it.
8. **Unaffected, verified rather than assumed:** `town_world` itself and its
   genesis totals (@src/engine/worldgen.rs:377); the 50-tick employment soak
   (`NEAR_FULL` = 21); the 100-tick soak's coffer-at-buffer bound, since the levy
   takes only from agent wallets and phase 6's draw still runs before phase 7.
   Each is asserted per-soak at pack 2's gate, never as a blanket claim.

## Acceptance tests (written from the contracts above)

**A0 — the disallow clause, written where a future pack-writer trips over it.** No
criterion of the form "pop 30 / 6 firms / 21 employed / 0 closures / 0 departures"
may stand alone or carry a gate. The measured frozen corpses pass it identically:
with no Food seller the destitution decide is guarded (@src/sim.rs:1342), nobody
can be judged destitute, and population freezes **by rule**. Those numbers ship as
*supporting observations*, read only after A2 and A3 are green, and **the gate
order is part of the criterion**.

**A1 — conservation and matched issue, every tick to t5000, every metal.**
`total_money(Gold) == Money::new(52_148)` after every tick;
`total_minted(Gold) − total_burned(Gold) == 52_148` after every tick; and, per
tick, `Δtotal_minted(Gold) == Δtotal_burned(Gold) == Event::Recycled.pot`. For
Silver and Copper both deltas are `Money::ZERO` every tick — trivially true
because nothing mints or burns them after genesis, asserted anyway because it is
the per-metal completeness the totals-only audit cannot reach, and it is free.
Enforced by the assertion **inside `tick`**, so it runs on every scenario that
calls `tick` or `tick_with_rate` — not only in the soak, but equally not in
fixtures that drive a phase fn directly. *Denominator note: 52,148 is
`total_money(Gold)`, the whole supply including External (600 at genesis) and Mint
(0); every share figure in this spec names its denominator in the same sentence.*

**A2 — transaction volume at three horizons**, over every rolling 20-tick window
centred on **t500, t2000 and t5000**: at least 55 `Event::Sold` per tick, at least
500 g of goods turnover per tick (Σ price × units over `Sold`), at least 600 g of
`WagePaid` per tick, and at least 450 units in `Event::Produced` per tick. Floors
sit at roughly 70–90% of the measured steady state (68–82 Sold, 632–912 g
turnover, 662–702 g wages, 508 units) so the criterion has margin without being
vacuous. **A value that holds only at the shortest horizon is a failure, not a
pass** — the minted stipend's climbing threshold (23 → 27 across t2000–t10000) is
exactly the trap a t1000 gate walks into, and the winning arm's own evidence runs
to t10000. A rule-frozen system scores zero on every clause here.

**A3 — live limit cycle, measured under *this* remainder rule and then pinned.**
Pack 2 measures the exact recurrence lag L of the per-agent
`(gold balance, Food inventory)` vector (L = 10 was measured under the probe's
arm, with no recurrence at lag 1/2/5) and pins it: the vector at t600+L equals
that at t600 EXACTLY, **and** the vector at t601 DIFFERS from t600. The pairing is
the point — recurrence alone is satisfied by a corpse; recurrence plus lag-1
difference proves the town is moving through a cycle.

**A4 — bottom tail, as a stock gate over every tick and an income flow recorded.**
Gate: at **every** tick from t100 to t5000 — not at sampled instants, since a
decaying trajectory can look solvent on a sampled tick — zero living agents hold
less than **28** gold, the constants-derived full basket at settled prices. Null
contrast: 3 of 10 at t300; the cure measured 0 of 30 at every horizon.
**Recorded, NOT gated:** the count of residents receiving zero gold of
wage-or-draw income over each rolling 50-tick window, t50 → t2000 — expected
**9**. The stock gate is near-self-fulfilling under a mechanic that credits every
wallet every tick; the flow number is what honestly describes what shipped, and
the ledger carries it so a successor cannot read "bottom tail 0 of 30" as
"everybody has a livelihood".

**A5 — hunger is a warm-up transient only.** Zero `Event::WentHungry` from t15
through t5000, and every `Agent.hunger` counter zero at t600 and t5000. Measured:
68 events, all between t2 and t14, zero thereafter. Any nonzero hunger after t14
fails the pack.

**A6 — progressivity is pinned as a mechanism, not inferred from an outcome.**
Unit-pinned: at 20‰ an agent holding 49 g levies `Money::ZERO` and still receives
a full share; an agent holding 5,000 g levies 100. Soak-pinned: over every tick of
t480..=t500 and t1980..=t2000, that tick's per-head share strictly exceeds the
levy paid by the poorest **levied** agent. A flat household levy was the worst arm
tested; proportional-with-equal-rebate is net progressive (net = share − rate ×
balance, negative above the mean wallet), and "households only, proportional" is
this spec's least-measured decision — so the mechanism is tested, not asserted.

**A7 — price tripwire, which no volume floor can catch.** `adjust_price` has no
ceiling, and boosted demand was measured to send Food to 179 g by t100 and the
series to 3.3 × 10¹⁰/unit under an inelastic outside buyer. Because A2's floors
are *lower* bounds, a slow price ratchet would raise turnover and pass every other
criterion. Therefore: at every tick to t5000, the cheapest posted price of each
good is at most **4×** its t20 settled level; no seller posts a sell-out streak
longer than 20 consecutive ticks; and the 100-tick soak's re-measured criterion 2
still holds under the cure.

**A8 — no orphans, no drift, and the seams pinned.** Per tick to t5000:
`sum(disbursed) == pot` exactly (the threaded pot must end at `Money::ZERO`);
`balance_of(external_id, Gold)` and `balance_of(mint_id, Gold)` unchanged from
genesis (600 and 0) at every tick — External was measured untouched since genesis
and this milestone must not move it; and every removed agent's balance is
`Money::ZERO` on every metal (the existing per-account no-orphan pattern, which
the totals-only audit cannot make).

**A9 — null reproducibility, as an executable test rather than a claim.** Over the
**three rate-0 twins** pack 2 item 3 cuts from the pre-cure soak bodies *before* it
re-cuts the originals: every pre-cure assertion passes bit-identically — including
the six the cure inverts (@src/engine/worldgen.rs:896, :898, :902, :930, :961,
:1000), which is what makes this a real null and not a tautology — zero
`Event::Recycled` is emitted, and `total_minted` / `total_burned` never move from
genesis on any metal. The pre-cure soak *bodies* cannot be the vehicle: they call
`sim::tick`, so once the constant is frozen at 20 they run the cured town by
construction. **Pack 2's gate carries A9 explicitly**; pack 1 does not discharge
it, since pack 1's bit-identical claim is measured against the pre-pack build at a
rate no phase yet reads. Without the `tick_with_rate`
seam this is a criterion no single build can hold alongside the cured default
(open question 4).

**A10 — the insolvency-fuse obligation discharged** (@src/sim.rs:328). The maximum
consecutive `Business.insolvent_ticks` observed over t1..=t2000 under the cure is
recorded in the pack ledger and is strictly below `CLOSE_INSOLVENT_TICKS`; the
constant is then re-frozen or explicitly confirmed at 12 **with that number
quoted**. Not optional and not assumable — 395 payroll shortfalls per 2000 ticks
were measured at this rate, so arrears do occur. The 100-tick soak's zero-closure
criterion is the tripwire.

**A11 — shock recovery (pack 3), including the ordering clause the split
expression owes.** Run the cured town to t500, force-close one Food seller, and by
t500+K (K measured, then pinned): the town's **headcount** is back to its
pre-shock 30 — headcount, not just firm count, because the phoenix venue
under-replaces by 2 jobs and installs permanent hunger; the cheapest posted Food
price is back inside the [min, max] band observed over t400..=t500; A2's volume
floors hold again for every tick in t500+K..=t700; **A7's tripwire held at every
tick throughout the recovery** (a ratchet that eventually returns must not pass
through an excursion the gate never sees); and zero `WentHungry` fires after
t500+K. **Plus the ordering clause:** no `Event::Departed` fires for an agent whose
post-dividend balance that tick would have cleared the cheapest posted Food price.
If one does, the phase-7-levy/phase-8-payout ordering is recorded in the ledger as
a known asymmetry of the burn/mint split against a single-phase transfer
expression.

**A12 — the lifecycle is not entombed, charged rather than merely named.** Under a
forced single-seller shock, `market::plan_founding` must return `Some` within
`FOUND_SIGNAL_TICKS + 2` ticks and an `Event::Founded` must land. **This asserts
reachability only, and the spec says so in the same sentence:** the cured
six-venue town holds exactly 2 sellers of every good on all 600 measured ticks, so
`plan_founding`'s two tiers — which require 0 or 1 sellers — return `None` every
tick while an eligible founder and a vacant house exist on 600 of 600; phase 1's
pull is likewise dormant (1 `Arrive` decided, 0 applied in 600 ticks). This
criterion distinguishes "quiet because healthy" from "dead by rule". It does not
claim founding fires in steady state, and it does not pretend the dormancy is free.

**A13 — command-layer id and pot rules (pack 1, no soak needed).** `World::levy`
and `World::disburse` each return `WorldError::UnknownAgent` for the Mint id, the
External id, a live business id and a ghost id, with the books provably untouched;
both are `Ok` no-ops at `Money::ZERO` and neither creates an account entry; `levy`
forwards `MoneyError::InsufficientFunds` atomically; `disburse` returns
`WorldError::OverIssue` beyond the remaining pot with nothing minted and
`total_minted` unmoved.

--- APPROVAL GATE — do not write the plan or any code above this line without sign-off ---

## Open questions

Each carries a recommendation, the cost of the alternative, and exactly what it
blocks. None is deferred into the plan.

1. **The amendment fork — the one real decision.** *Recommend:* buy Amendment 20
   as a **closed row-8 purpose grant**, keeping the burn+mint split (this spec's
   shape). *Cost of the alternative* — widen row 7's money-ops for a conserved
   agent→agent redistribution transfer: it is **honest bookkeeping**
   (`total_minted(Gold)` stays 52,148 and `total_burned(Gold)` stays zero forever,
   no reinterpretation, no counter inflation), it does **not** wake phase 8 so
   Amendments 22 and 23 evaporate along with the tick-level matched-issue
   assertion, and it makes **available** the pre-destitution ordering the `sinks`
   contract forecloses — a single-phase pass can sit above the emigration decide.
   Note that the measured-identical arm did **not** sit there, so exercising that
   option is an unmeasured behavior change and a second decision, not a free
   benefit of the route. Against it: row 7 permanently gains a standing
   transfer power every future phase-7 mechanic inherits. **The honest
   counterweight, stated because it cuts against my own recommendation:** the
   grant this spec buys is on the *worse* axis — a transfer cannot create money,
   while a mint purpose can, and §8.3 provably cannot police a mint. The two
   expressions were measured **tick-for-tick identical** at the same ordering
   position to t10000, so — that foreclosed option aside — this is a question of
   what the phase table should be allowed to say. *Blocks:*
   both wrapper contracts (on the transfer route `disburse` disappears and `levy`
   collapses into `World::pay`), the `sim::mint_phase` contract entirely — taking
   `Event::Recycled`'s only emitter with it — the `sinks` contract's return-a-pot
   shape, A8's `sum(disbursed) == pot` clause, A13's whole `disburse`/`OverIssue`
   half, Migration items 2 and 5 (a transfer route calls neither `burn` nor
   `mint`), pack 2 item 4, and roughly half of pack 1's test list. **And note the
   alternative's amendment is a different one:** Amendment 21 as drafted is a row-7
   *purpose* edit, but on the transfer route it becomes a row-7 **money-ops**
   widening, which the Amendments preamble's "money-ops cells for rows 7 and 8 are
   unchanged" no longer covers. **Decide before pack 1 opens.**
2. **Reinterpret the two §8.4 counters now, or add a second counter pair now?**
   *Recommend:* reinterpret `total_minted` / `total_burned` as "gross lifetime
   credits/debits through the §8.4 logged paths", add the derived
   `net = minted − burned` column to the one non-test consumer
   (@src/engine/game_loop.rs:137), and write the **expiry** into the manifest —
   this reinterpretation expires the moment a caller needs a backing ratio against
   a gold reserve, at which point gross-mint-for-reserve must be told apart from
   recycle-mint. *Cost of the alternative* (a `recycle_minted`/`recycle_burned`
   pair now): new state in the trusted core with its own single-writer obligation,
   bought today to fix a display string — but paid while the counters still hold
   only genesis values rather than after millions of recycle units. If the owner
   believes the real §2.1 Mint is near, pay early. *Blocks:* `money.rs`'s public
   surface and the shell column in pack 2; nothing else.
3. **Households only, proportional — the least-measured decision in the design.**
   *Recommend:* keep it. Firms hold ≈2,028 g ≈ **3.9%** of the 52,148 g supply
   under the **cure** — `DRAW_BUFFER_BILLS` × the 676 g full-staffing bill, derived
   from this spec's 49,520 g levy base and pinned by pack 2 item 6, not separately
   measured; the 0.8% figure is the **null** arm at t300, a collapsed town — so a
   coffer levy reaches ~40 g/tick against the ~987 g/tick household pot, and it
   reaches it out of precisely the buffer `draw_amount` retains and the closure
   fuse reads. It would also perturb both, and reaching into coffers via draw-off
   was measured worthless by t2000. *Cost of the alternative* (a
   firm-side or mixed base): possibly a lower sufficient rate and less owner
   accretion — but **no firm-side variant was measured**, so it would ship on
   unmeasured ground, and the evidence about *flat* levies is not evidence about
   proportional ones. *Blocks:* the narrow id rule in both wrapper contracts;
   changing it later is a signature-level change, not a tuning change.
4. **Does `tick_with_rate` exist?** *Recommend:* yes. One `pub(crate)` fn and one
   delegation, `sim::tick`'s ⟨REF⟩ signature verbatim, no persistent state — and
   it is the **only** way A9 can be an executable test rather than a claim, since
   as a bare `const` "rate 0 reproduces the baseline" and "the cured town is the
   default" cannot both be green in one build. It is also the migration path if a
   successor turns the constant into policy data. *Cost of the alternative*
   (const only): A9 is demoted to a one-off recorded measurement and the baseline
   stops being reproducible from a shipped build. *Blocks:* A9's form and one item
   in pack 2.
5. **Is the dormancy of the just-shipped firm lifecycle and migration pull
   acceptable as the price of stability?** *Recommend:* yes, with A12 charging for
   it and the spec saying plainly that in the shipped scenario `plan_founding`
   returns `None` on 600 of 600 ticks, phase 1's pull decides 1 `Arrive` and
   applies 0, and External's gold has not moved since genesis. *Cost of the
   alternative* (a second shipped scenario keeping both live — e.g. a five-venue
   seed where some good has one seller): a whole new worldgen scenario with its own
   pinned totals, soaks and tuning, none of it measured, and the rate is n=1 to
   `town_world` with no derivation rule for a second seed. *Blocks:* pack 3's
   scope, and whether a fourth soak exists.
6. **Is the founding-template headcount question in scope for this milestone?**
   *Recommend:* yes, as pack 3's **re-run sweep** against the corrected criterion,
   invoking the standing invitation in `market.rs`'s own table comment
   (@src/market.rs:358–361) — **never** as an asserted 2→4 correction, because that
   table records headcount 3 as *measured and failing* the then-signed anti-churn
   criterion and 4 as "not run". *Cost of the alternative* (hand it to a
   firm-lifecycle follow-up): A11's headcount clause must be weakened at the gate
   with the measurement quoted, and the monotonic ratchet — pop 24 / employed 15
   under repeated shocks, never regrowing — stays live in shipped code as a named
   defect. *Blocks:* A11's headcount clause and pack 3's contents; packs 1–2 are
   unaffected either way.
7. **Is this milestone titled and gated as STABILITY, explicitly?** *Recommend:*
   yes — and record in the manifest's goal that every stable configuration
   measured parks at exactly the seeded 30 residents with arrivals 0 and foundings
   0 forever, so **a flat 30 is the designed outcome, not a growth failure
   discovered later**. Name the successor with its number in the same paragraph.
   *Cost of not doing it:* a successor reads the flat population as a regression
   and re-opens the refuted levers from CLAUDE.md's still-standing circulation
   paragraph. *Blocks:* nothing technical; it is the framing the whole gate is read
   through.
8. **Does the t5000 soak run in the default `cargo test`, or behind `#[ignore]`?**
   *Recommend:* default — a 30-agent town for 5,000 ticks is cheap in this
   zero-dependency crate, and A2's three-horizon rule is the direct answer to the
   climbing-threshold trap; a criterion that only runs when someone remembers to
   run it is not a gate. *Cost of the alternative:* `./scripts/verify.sh` stops
   being the acceptance line for the milestone's central criterion, contradicting
   the repo's own gate policy. *Blocks:* pack 2's gate wording and item 7. **Measure
   the actual wall-clock cost in pack 1 and decide with the number.**

## Corrections owed to CLAUDE.md (part of pack 2's definition of merged)

These are not optional tidying: CLAUDE.md's standing finding is the premise this
whole milestone was commissioned on, and the measurement refuted it. Leaving it
standing would mean the next reader re-derives a refuted remedy.

1. **Delete the `target_days`-as-sink mechanism, do not soften it.** Replace with
   bottom-tail solvency on top of the technology fact.
2. **Record the implied remedy as refuted**, so nobody re-derives it: `target_days`
   7 → 14/30/70/100/1000 gives t300 pop 6/3/4/4/4 against the baseline's 10 and
   *raises* the largest wallet to 35.3%/64.6%/51.0%/99.3% of the 52,148 g supply.
3. **Rewrite "phase 7's demurrage/imports and phase 8's mint are the standing TODO
   stubs that address it"** — false on all three counts as independent mechanics.
   What addresses it is the conserved recycle, and phases 7 and 8 are its home **as
   a matched pair**, not as the independent sinks and faucet that sentence
   promises.
4. **Correct "the tick-time mint faucet is closed … the audit pins it there"** —
   wrong in its *reason*, independently of this milestone. The audit provably
   cannot detect a mint; the faucet is closed only because nothing calls `mint` at
   tick time. The identical misattribution at @src/sim.rs:1448 and
   @src/engine/worldgen.rs:370 is corrected in the same pass. After pack 2 the
   sentence also changes in fact.
5. **Keep the decline narrative, but name the right successor** — producer-side
   demand and phase 6's expand-capacity half, with the 221–278 g/tick profit stream
   and its 7.9–9.9 livelihoods against nine incomeless residents.
6. **Add the refutation of the aggregate-deficit framing**, because it is the trap
   the next reader falls into: household income 772.5 g/tick against 772.3 g/tick
   of purchases over t20–t120, a +0 g residual over 1000 ticks.
7. **Fix the roadmap's Money-creation line** — "a literal staffed Mint business
   that *consumes* precious-metal goods" contradicts §2.1 and CLAUDE.md's own hard
   invariant 4, both of which hold that gold is *held in reserve, not consumed*.
8. **Add the n=1 caveat wherever a measured constant is quoted.** Every figure in
   the ~40,000-tick record comes from one scenario at one wealth distribution;
   `RECYCLE_PERMILLE`, like `DRAW_BUFFER_BILLS`, `CLOSE_INSOLVENT_TICKS` and
   `GRUBSTAKE`, is frozen against that seed and nothing else.
9. **Update the structure section** for what this milestone changes — `burn` gains
   a live caller, `pay` stops being the only command-layer wrapper, phase 7 returns
   a pot, phase 8 is no longer a stub, and "imports and exports are **refused with
   measurements**, not pending".
