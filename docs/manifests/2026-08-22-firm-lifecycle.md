# Firm lifecycle (container)

**Status:** IN FLIGHT — gate signed 2026-08-22; **pack 1 DONE**
2026-08-22 (`VERIFY OK`, 160 passed); **pack 2 IN PROGRESS** from
2026-08-30 (the owner's go). Pack 3 planned, on the owner's go.
**Branch:** `claude/town-colony-sim-p1s06q` (PR #2)
**Origin:** [2026-08-22-firm-lifecycle-design.md](../superpowers/specs/2026-08-22-firm-lifecycle-design.md)
(approved 2026-08-22; drafted from a judged three-angle proposal panel —
market-signal purist / bookkeeping minimalist / colony-story first, scored
under repo-fidelity, economy-soundness, and incrementality lenses — then
adversarially verified against shipped code before the gate; both passes
recorded in PR #2).

## Goal

Firms stop being immortal coffers. Every business gets a required living
owner; phase 6 wakes with the three lifecycle transitions: the **profit
draw** (gold surplus above a retained buffer, business→owner, every tick —
the recorded cure for the dis-saving fuse, expected partial: consumption
caps turn draws into founding capital, not demand), **closure** (persistent
arrears liquidate a venue: creditors settled `min(coffer, owed)` ascending,
staff laid off, every metal of the residual to the owner, the freed house a
landing pad), and **founding** (`Intent::Found`: fewer than two sellers
plus a live scarcity price — or a dead sector — and the first capitalized
unemployed resident stakes three wage bills into a new venue and
self-hires). An emigrating owner's firms force-liquidate inside
`remove_agent` (Amendment 19 — the milestone's one money-op widening); the
vacancy pull gains an arrears exclusion, resolving the recorded
deadbeat-recruitment handoff. End state at `cargo run`: profit lines in
the feed, a demand-gutted venue visibly dying instead of zombie-ing, and
the full phoenix cycle — closure → scarcity → founding → restaffing —
playing out inside a soak, audit green throughout.

Not in scope (spec, Out of scope): ownership transfer/sale/inheritance,
shares, banking/credit, `House.owners` rules, voluntary job-switching,
multi-role founded firms, premises construction, demand-composition
reshaping, the wage-ratchet overshoot, the Mint business, non-gold
pricing, terrain, physical goods movement.

## Decisions (fixed by the spec and the gate rulings)

- **Gate rulings (2026-08-22):** (1) founder eligibility is
  unemployed-only; (2) Amendment 18 is spent on the row-6 purpose-text
  edit (the direction-unrestricted "transfer only" reading is pinned in
  the spec's Invariants); (3) three packs, forced liquidation in pack 2.
- **Amendments 18–19** ride this container: 18 (row 6 purpose text, lands
  pack 1 with the draw), 19 (row 7 money-ops gains the forced-liquidation
  transfers, lands pack 2 with `remove_agent`'s step 0).
- **`Business.owner` is required and living** — enforced by construction
  once pack 2 lands; the pack-1 interim carries the specified draw-skip
  tolerance (dangling owner ⇒ skip cleanly, no event, test-pinned,
  retired by pack 2).
- **The `ClosureReceipt` is the event-measurement mechanism**: settlement
  and residual can share the owner's wallet inside one atomic command, so
  events are emitted from per-step receipts, never around-the-command
  deltas; `apply_sinks_intent`'s A17 creditors snapshot excludes
  leaver-owned firms once step 0 exists.
- **The pack-2 re-cut is named up front**: any quit now guarantees that
  venue's closure (frozen ex-worker arrears are unpayable outside
  settlement), and with the Arrive exclusion live, the shipped 200-tick
  arrival criterion cannot hold on pack-2 code — its departure-side
  criteria stay, the arrival assertion moves to pack 3's full-cycle soak,
  where a founded firm is the pull-eligible vacancy.
- **Pack 1 re-measures before anything freezes**: the draw re-times the
  fuse and the 21/30 ceiling; every pack-2/3 criterion is written against
  pack 1's re-pinned numbers, never the inherited t127 / 21-of-30.
- **Determinism without RNG** carries over: houses-order passes,
  ascending-`AgentId` settlement and founder choice, `Good::ALL` /
  `Metal::ALL` iteration, sorted `owed_to` keys — never `HashMap` order.

## Packs

### Pack 1 — Owners and the draw

Item manifest: [2026-08-22-fl-pack1-owners-draw.md](2026-08-22-fl-pack1-owners-draw.md)
(DONE 2026-08-22; 155 → 160 tests; the re-measured baseline and the
pack-2 trigger-design handoff live in its ledger).

`Business.owner` + the widened, owner-validating `create_business` (29
forced call sites — worldgen ×2, world.rs ×15, sim.rs ×12; the spec's
estimate was exact); the worldgen reorder seeding each venue's
first worker as owner-operator (alice/ed/ivan/karl/marco/otto) with the
id-pin migration and the per-metal totals asserted unchanged; the phase-6
direct draw pass (`draw_amount`, `DRAW_BUFFER_BILLS`, the dangling-owner
skip) with `Event::ProfitDrawn`; Amendment 18 executed; shell owner
display + business count; the named re-measure item re-pinning the fuse
timeline and `NEAR_FULL` under the draw.

Observable: "Greenrow Farm paid alice 62g profit" in the feed; coffers
visibly plateau at the buffer; owners named in roster and inspect.

### Pack 2 — Firms die

Item manifest: [2026-08-30-fl-pack2-firms-die.md](2026-08-30-fl-pack2-firms-die.md)
(IN PROGRESS from 2026-08-30; the closure trigger settled by a fresh
200-tick probe + a judged four-angle panel — the signed predicate stands,
`CLOSE_INSOLVENT_TICKS` retunes 6 → 12, one erratum).

`Business.insolvent_ticks` (single writer: phase 6's write-back over the
live set); `World::close_business` returning the `ClosureReceipt`
(settle-ascending → write-off incl. zero entries → layoffs → `Metal::ALL`
residual to owner → detach); the phase-6 closure pass before draws; the
Arrive decide+apply arrears exclusion (the handoff resolved by rule);
forced liquidation in `remove_agent` + Amendment 19 + the dedicated
owner-emigration fixture; `Closed`/`LaidOff` + `Settled` doc widened; the
100-tick zero-closure pin and the NAMED re-cut of the 200-tick migration
soak.

Observable: on a stress fixture the whole death narrates — quits, then
"The Brass Bell closed — karl pockets 12g" — and the freed address
inspects as a vacant residence.

### Pack 3 — Firms are born

Item manifest: written on your go.

`market::plan_founding` + the founding template (FOUND_PRICE/wage/
headcount per good) + `FOUND_SIGNAL`; `Intent::Found` (one per tick,
unemployed-only, phase-start snapshot, kill-only re-checks);
`World::found_business` + the stake; the founder self-hire;
`Event::Founded`; the both-directions shock soak, the anti-churn tuning
target with its two named levers, the restored arrival chain, the
full-cycle criterion; CLAUDE.md update + INVENTORY fold = container DONE.

Observable: after a venue dies, "mira founded a food stall at
5 Weir Cottage (staked 420g)", then Hired events restaffing it; zero
foundings on the tuned equilibrium town.

## Open questions (carried)

None. All three spec questions were ruled at the 2026-08-22 signing (see
the spec's SIGNED paragraph). Carried *re-records*, not questions: the
wage-ratchet overshoot stays untouched; owner-wallet pooling is the
recorded seam for phase 6's "expand capacity" half; `House.owners` stays
rule-inert; the 21/30 ceiling's composition cause stays the owner's-choice
follow-up whatever pack 1 re-measures.

## Ledger

- **2026-08-22** — **container drafted from the approved spec.** Baseline:
  the town-colony container closed at `VERIFY OK — fmt, clippy, build,
  tests all clean.` 155 passed (head 7794676). Spec provenance: three-angle
  proposal panel judged under three lenses (the minimalist spine won, with
  the colony-story command shapes and the purist's closure trigger
  grafted; the judges killed the sell-through founding signal — it reads
  the ent/lux deliberate-scarcity regime as an entry invitation — and the
  close-beats-pull timing race). Adversarial verification (three critics
  vs. shipped code) found and fixed pre-gate: the closure-receipt
  event-measurement mechanism (around-the-command deltas cannot attribute
  flows sharing a wallet), the guaranteed-closure-after-any-quit corollary
  and the pack-2 migration-soak re-cut it forces, the pack-1
  dangling-owner window (now a specified, test-pinned tolerance), the
  draw-buffer depth mismeasure (worldgen's 3 bills bought ~4.5–6 boot
  ticks, not 3 full-staffing ticks), the slow-bleed closure ordering (now
  a stated design decision), the founding template's missing price datum,
  and the stake-failure branch that would have minted a closure-proof
  empty firm.
- **2026-08-22** — **gate signed; container moves to PLANNED.** All three
  open questions ruled per the owner's direction (recommendations
  adopted): unemployed-only founders; Amendment 18 spent on the
  purpose-text edit; three packs. Owner's "start pack 1" is the go on
  pack 1's items — execution authorized for pack 1 only.
- **2026-08-22** — **pack 1 closes** (commits a44385e..close): the owner
  lands everywhere (widened `create_business`, 29 forced sites [read 27
  until the 2026-08-22 re-measure below], the
  worldgen reorder with owner-operators pinned, per-metal totals
  unchanged), phase 6 wakes with the draw (`draw_amount`, the
  dangling-owner interim skip, `Event::ProfitDrawn`, Amendment 18
  executed), the shell knows owners, and the re-measure sets the new
  baseline: **the sink is dead** (coffer ≤ 3 bills + owed, now soak
  criteria 5–6), **the fuse unmoved** (first departure t127, `NEAR_FULL`
  21 — the cure is capital, not demand: owner income pools behind
  `target_days` caps, the recorded expand-capacity seam), **quit churn
  earlier and broader** (t174→t134, 2→3 venues, 11→26 quits — the
  cushion the draw removes), and the **pack-2 trigger datum** (post-shock
  insolvency streaks 57–73 ticks at still-operating venues; a bare
  6-tick persistence trigger's head start over the quit line is −1 to
  +1 ticks — at one venue the quit fires first — so
  `CLOSE_INSOLVENT_TICKS = 6` cannot ship as-is; the trigger goes
  magnitude-aware or sits above 73, pack 2's manifest rules it).
  3-lens close review: zero blockers; two ledger-honesty MAJORs and six
  MINORs, all applied. Close gate quoted in the pack manifest:
  `VERIFY OK` 160 passed. Next: pack 2 item manifest, on your go.
- **2026-08-22** — **doc-accuracy review** (owner-requested, between
  packs: CLAUDE.md, this container, the pack-1 manifest, INVENTORY —
  five parallel reviewers over ~295 claims, every finding adversarially
  verified; 1 killed as a misreading, 11 confirmed: 1 MAJOR, 8 MINOR,
  2 NIT; zero touched a §8 invariant or money logic, and the headline
  claims re-verified from scratch — `VERIFY OK`, 160 passed). All
  applied. The two that were code-vs-decision, not prose:
  - **`roster` had drifted from D3** (multi-metal pack 2 decided the
    compact `g/s/c` form "everywhere a single balance prints today
    (roster lines, inspect view)"; roster shipped gold-only, and
    CLAUDE.md's "every balance renders as compact `g/s/c`" was
    therefore false). Reconciled toward the decision — roster now calls
    `compact_balances`, so silver and copper stop being invisible in
    the one view that lists every agent.
  - **The draw's "draws nothing, ever" gloss overstated its own
    formula** — `draw_amount` *nets* arrears into the buffer rather
    than gating on them, so a venue carrying arrears does draw once its
    coffer clears bills + owed (phase-4 revenue makes that reachable
    after phase 3 carried the debt). The formula is the contract and is
    unchanged — deliberately, since it is measured-in and pack 1 is
    closed; the gloss was corrected in the spec (as an erratum), in
    sim.rs, and in INVENTORY.

  Prose corrections: this container's Status header (stale at PLANNED /
  "pack 1 authorized" against its own DONE pack section and ledger);
  the "0–3 ticks" head-start range in BOTH ledgers (the tick values
  printed beside it give −1..+1, with one venue's quit firing before
  the crossing — pack 2 designs its trigger from this, so the ordering
  matters); INVENTORY's frame order (the money summary renders above
  the feed, not below) and its unconditional "every business belongs to
  a named resident" (the pack-1 interim tolerates a dangling owner
  until forced liquidation lands); CLAUDE.md's "stubbed node modifier
  layer" (there is no stub — nothing in code, and the parent spec's
  "hook exists" is intent) and its two owner-machine references
  (unreachable from cloud/remote sessions — the shipped manifests are
  now named as the fallback templates).

  Eyeballed, not just compiled — the roster change is shell behavior no
  test covers, so it was read in `cargo run` at town scale:
  `alice — labourer at Greenrow Farm · owns Greenrow Farm · g:120 s:10
  c:20 · food 10 · entertainment 5 · luxury 2`. Gate after the edits:
  `VERIFY OK — fmt, clippy, build, tests all clean.` 160 passed.

  **Post-fix audit — and a regression it caught in this very entry.**
  A second four-lens adversarial pass over the applied diff confirmed 6
  defects (15 killed), one of them MAJOR *and self-inflicted*: the first
  pass "corrected" the container's accurate "~29 call sites" to 27 and
  stamped it "the measured 27". The count is **29** (worldgen ×2,
  world.rs ×15, sim.rs ×12; `a44385e` removes 29 `create_business(`
  call lines, none of them the definition) — the 27 came from the
  pack-1 ledger, which had it wrong, and this review propagated the
  error into two more files including a signed spec before re-measuring
  from git caught it. All three sites now read 29, and the pack-1
  ledger carries a dated in-place correction. The other five (a
  misquotation inside a correction note, a lazy-continuation markdown
  bug that reparented this paragraph under the sub-bullet above, three
  unwrapped lines, and the missing eyeball record now supplied) are
  fixed here. Lesson recorded: a doc-accuracy pass is exactly as
  fallible as the docs it audits — the numbers it "corrects" need the
  same primary-source measurement as the ones it flags.
