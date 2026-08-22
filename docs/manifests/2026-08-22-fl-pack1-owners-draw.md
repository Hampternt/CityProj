# Firm lifecycle — Pack 1: Owners and the draw

**Status:** IN PROGRESS 2026-08-22.
**Container:** [2026-08-22-firm-lifecycle.md](2026-08-22-firm-lifecycle.md)
**Spec contracts executed here:** `Business.owner`, the widened
`World::create_business`, `draw_amount` + the phase-6 draw pass (with the
pack-1 dangling-owner skip rule), `Event::ProfitDrawn`, the worldgen
owner-seeding item, Amendment 18 (row-6 purpose text). `close_business`,
`found_business`, `insolvent_ticks`, `Intent::Found`, and Amendment 19 are
packs 2–3.

Observable (verbatim from the container): "Greenrow Farm paid alice 62g
profit" in the feed; coffers visibly plateau at the buffer; owners named
in roster and inspect.

## State on arrival (measured 2026-08-22)

| What | Measured |
|---|---|
| Test suite | 155 passed, 0 failed (container close `verify.sh` at 7794676) |
| Phase 6 | `invest(_world)` TODO stub, takes no report |
| `Business` | no `owner`, no counter fields; `owed_to` arrears ledger live |
| `create_business` | 4-param, no owner; 29 call sites beside the definition (worldgen ×2, world.rs tests ×15, sim.rs tests ×12 — several inside shared fixture helpers) |
| `House.owners` | every shipped house seeds `vec![]`; rule-inert (stays so) |
| Worldgen order | per venue: `add_house` → `create_business` → mint bills → stock → spawn staff — the business id PRECEDES its staff ids; the reorder swaps that interleaving, shifting every downstream id |
| The fuse (pack-4 measurements, pre-draw) | coffers absorb ~90g/tick as one-way sinks; 9 unemployed dis-save ~25g/tick each from 3400g; first departure t127; Brass Bell gutted from t174; Mara arrives t182; savings band [3000, 3800] |
| Employment | `NEAR_FULL` frozen at the measured 21/30 (pack-3 deviation record) |
| Per-metal pins | gold 52148 / silver 300 / copper 600 — order-independent, must survive the reorder unchanged |

## Decisions (this pack's, within the spec's contracts)

- **Item 1 is deliberately atomic**: the signature widening, the worldgen
  reorder, and the id-pin migration cannot land separately — the compiler
  forces every call site the moment `owner` exists, and worldgen cannot
  pass an owner id that hasn't spawned. One compile-forced item (the
  pack-3 items-2+3 precedent for the recorded forcing), never scattered.
- **The reorder's exact shape**: per venue — `add_house` → spawn that
  venue's `seeded_staff` (same `NAMES`/`next_name` sequence, same
  `residences[next_name / 8]` homes: the agent-spawn ORDER is unchanged,
  only the business-id interleaving moves) → `create_business(owner =
  first spawned worker)` → mint bills → seed stock. Unemployed spawn after
  all venues, as today. Owners land alice/ed/ivan/karl/marco/otto by
  construction.
- **Draw pass shape** (spec contract restated for the diff): direct pass,
  no intents, businesses in houses order, gold only;
  `draw_amount(coffer, wage_bill(), owed_total())` with
  `DRAW_BUFFER_BILLS = 3` first guess (independent constant — the
  re-measure item may raise it; the spec forbids the "worldgen already
  proved 3" justification); positive draw ⇒ `World::pay(business, owner,
  Gold, draw)` + `Event::ProfitDrawn { business, owner, amount }`; zero
  draw silent. Dangling-owner skip: `world.agent(owner).is_none()` ⇒
  skip, no transfer, no event — pinned by test, retired by pack 2.
- **Amendment 18 executes with the draw** (the first mechanic to hang on
  row 6): the 07-02 table's row-6 purpose cell edits to "take profit /
  found & liquidate firms (capacity expansion still TODO)" and the 07-02
  header's amended-by line gains this spec's Am. 18 (19 follows in
  pack 2).
- **Shell**: `ProfitDrawn` renders "Greenrow Farm paid alice 62g profit"
  and stars the owner in last-3 history; `roster` marks owners
  (`alice — Labourer at Greenrow Farm — owns Greenrow Farm`
  presentation-level, plan-owned); business inspect names the owner; the
  header gains a business count. Exhaustive-match forcing covers the
  render arm and the trace module.
- **The re-measure item's procedure** (the container's baseline-setting
  step): re-run all four soaks under the draw. Record per venue: draw
  totals, post-phase-6 coffer ceiling (must be ≤ 3 bills + owed_total
  from tick 20 — the sink-is-dead criterion), `PayrollShort` flicker
  count, and the longest revenue drought at actual staffing (the datum
  that sizes `DRAW_BUFFER_BILLS` and, in pack 2, `CLOSE_INSOLVENT_TICKS`).
  Re-pin: the fuse timeline (first departure tick), the arrival tick if
  moved, `NEAR_FULL` if moved. Red criteria retune in this order:
  `UNEMPLOYED_SAVINGS` within a re-measured band, then
  `DRAW_BUFFER_BILLS`, then criterion re-pin with the deviation recorded.
  All numbers quoted in the ledger — they are the baseline packs 2–3
  cite.

## Items

- [ ] **1. The owner lands everywhere (atomic).** `Business.owner:
  AgentId` (doc: required, living once pack 2 lands, distinct from
  `House.owners`); `create_business` widened — `owner` validated FIRST
  (spawned agents only; reserved/business/ghost ids ⇒
  `WorldError::UnknownAgent(owner)`), house checks unchanged, `Err`
  changes nothing; the worldgen reorder per the decision above with
  owners seeded; every call site migrated (tests spawn owners; fixture
  helpers gain one); id-pinned tests and event expectations re-pinned;
  per-metal totals asserted unchanged (gold 52148 / silver 300 / copper
  600). New pins: owner-validation-first suite,
  `town_world_seeds_owner_operators` (the six named owners, each employed
  at their venue). Done: `./scripts/check.sh` clean; `cargo test world::`
  + `engine::` green, quoted. Touches: src/business.rs, src/world.rs,
  src/engine/worldgen.rs, src/sim.rs (test fixtures).
- [ ] **2. Phase 6 wakes: the draw.** `draw_amount` + `DRAW_BUFFER_BILLS`
  beside it in sim.rs; `invest(world, report)` gains the direct pass per
  the decision (houses order, gold only, dangling-owner skip);
  `Event::ProfitDrawn` + render arm + trace arm (compile-forced);
  Amendment 18 executed in the 07-02 spec (row-6 purpose cell + header
  line). Tests: `draw_amount_clamps_and_respects_arrears` (surplus /
  at-buffer / arrears-eat-surplus / integer exactness),
  `draw_pass_pays_owner_and_pins_coffer_at_buffer`,
  `draw_skips_a_dangling_owner_cleanly`, emission test (zero draw
  silent), audit green through draws. Done: `./scripts/check.sh` clean;
  `cargo test sim:: money::` quoted. Touches: src/sim.rs,
  src/engine/game_loop.rs, docs/superpowers/specs/2026-07-02-*.md.
- [ ] **3. The shell knows owners.** Roster owner mark, business-inspect
  owner line, header business count; ProfitDrawn stars the owner in
  last-3 history. Done: `./scripts/check.sh` clean; behavior eyeballed at
  `cargo run` and described in the ledger. Touches:
  src/engine/game_loop.rs.
- [ ] **4. The re-measure (+ the buffer freeze).** Per the decision:
  four soaks re-run under the draw; the sink-is-dead coffer criterion
  added to the 100-tick soak; drought/flicker instrumentation via
  TickReport harvesting in the soak harness; fuse timeline +
  `NEAR_FULL` re-pinned; `DRAW_BUFFER_BILLS` frozen (raised if the
  measured drought demands); retune order as decided; every number
  quoted in the ledger as the packs-2/3 baseline. Done: all four soaks
  green in `cargo test`; ledger carries the new baseline. Touches:
  src/engine/worldgen.rs, src/sim.rs (constants only, if retuned).
- [ ] **5. Pack close.** `./scripts/verify.sh` green; 3-lens close review
  (spec-contract fidelity / §8 invariants / economy quality); ledger
  quotes real output + new test count; container entry updated; CLAUDE.md
  code-state section (phase 6 no longer a stub; Business gains owner);
  PR body updated. Done: `VERIFY OK` quoted. Touches: docs/manifests/*,
  CLAUDE.md.

## Not in this pack unless you say so

- No `insolvent_ticks`, no `close_business`, no closure pass — pack 2
  (the dangling-owner draw-skip is this pack's ONLY nod to departure).
- No Arrive exclusion — pack 2 (the pull still recruits into deadbeats
  here, as shipped; the re-measured migration soak tolerates it).
- No `Intent::Found`, no `found_business`, no founding template — pack 3.
- No Amendment 19 — pack 2 executes it with `remove_agent`'s step 0.
- No `House.owners` behavior of any kind.
- No profit-driven owner behavior (spending, founding) — draws pool, and
  that pooling is measured, not patched (the recorded expand-capacity
  seam).

## Ledger

- **2026-08-22** — pack drafted from the signed spec; items 1–5 cut per
  the decisions above. Arrival state measured (table above): 155 tests at
  the container-close head; the spec + container manifest + this manifest
  are the only diffs since.
