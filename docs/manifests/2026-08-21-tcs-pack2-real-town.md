# Town colony sim — Pack 2: A real town

**Status:** DONE 2026-08-21 — all items landed, tuned, and reviewed; close
gate: `VERIFY OK — fmt, clippy, build, tests all clean.` 119 passed (114 on
arrival, +5). Two measured deviations from the drafted shape — recorded in
the ledger, flagged to the owner in the PR.
**Container:** [2026-08-21-town-colony-sim.md](2026-08-21-town-colony-sim.md)
**Spec contracts executed here:** `World::employees_of` (replaces
`employee_of`), `town_world`; the spec's pinned soak exit criteria become a
test.

Observable (verbatim from the container): `cargo run` opens on a genuine
town — 25–35 named agents, competing Food sellers' prices visibly diverging
in the feed, multi-worker payrolls, unemployed agents flagged in `roster`,
100-tick soak green.

## State on arrival (measured 2026-08-21)

| What | Measured |
|---|---|
| Test suite | 114 passed, 0 failed (pack-1 close `verify.sh`) |
| `employee_of` | world.rs:304, `Option<AgentId>` first-match; callers: produce (sim.rs), pay_wages (sim.rs), inspect workers line (game_loop.rs), own tests |
| Worldgen | `template_world` inline in game_loop.rs: 4 agents, 4 houses, 3 single-worker businesses; totals pinned at gold 245 / silver 40 / copper 80 |
| Goods table | consumption 10/5/2 · production per staffer 40/20/8 · target 7 days (07-19 pinned — tuning happens in worldgen, not here) |

## Decisions (this pack's, within the spec's contracts)

- **`town_world` scenario shape** *(AMENDED during tuning — the drafted
  28/14/8-business/24-employed shape was measured infeasible; see the
  ledger's deviation entries)*: 30 agents in 12 houses — 4 occupied
  residences (8/8/8/6), 2 zero-occupant spare residences (pack 4's
  landing pads), 6 business premises. Businesses over `Good::ALL`: 2
  sellers per good, all `headcount > 1`. 16 agents seeded employed
  (workplace + employed_role set directly, template precedent), 14 seeded
  unemployed — pack 3's hiring pool.
- **The unemployed live off savings until pack 3 hires them**: their
  wallets are seeded deep enough to keep buying Food through the 100-tick
  soak (the spec's every-agent liveness criterion applies to them too;
  income for them arrives with the labor market, not this pack).
- **Worldgen owns all tuning**: wallets, wages, prices, staffing, and the
  External settlement fund are `town_world` constants iterated until the
  pinned soak criteria hold. The 07-19 goods table is not touched unless
  the soak cannot pass without it (would be recorded as a deviation).
- **Worldgen splits into `engine/worldgen.rs`** (the spec left this to the
  plan): `template_world` moves there unchanged as the test fixture;
  `town_world` lands beside it; game_loop keeps presentation only.
- **Feed aggregates the routine at town scale** (the anticipated
  readability pass, shell-side only — events stay granular): WagePaid and
  Sold render as one line per business ("Greenrow Farm paid 3 workers
  105g", "Greenrow Farm sold 87 food to 12 buyers @ 2g"); PayrollShort,
  PriceMoved, and WentHungry stay individual lines. The last-3-events
  inspect buffer keeps granular per-agent lines.
- **`employees_of` order**: ascending `AgentId` by construction —
  `world.agents` is append-ordered by the ascending id counter — stated in
  the doc comment and pinned by a test rather than re-sorted.

## Items

- [x] **1. `employees_of` replaces `employee_of`.** New
  `pub fn employees_of(&self, house: HouseId) -> Vec<AgentId>` (derived
  scan, ascending, unknown house empty); `employee_of` deleted; produce
  scales stock gain by staff count; pay_wages accrues and pays every
  slotted employee (per-accrual payable re-read keeps the shared-coffer
  semantics); inspect's workers line lists all. Tests: ordering pin,
  multi-worker produce/payroll, updated callers. Done: `./scripts/check.sh`
  clean; `cargo test world::`, `sim::`, `market::`, `money::` green.
  Touches: src/world.rs, src/sim.rs, src/engine/game_loop.rs.
- [x] **2. Worldgen split + `town_world`.** `engine/worldgen.rs` with
  `template_world` (moved verbatim) and `town_world` per the contract:
  deterministic, seedless, no RNG; the shape decided above; every business
  seeded one full-staffing `wage_bill()`; every agent wallet + one day's
  goods + inert silver/copper; External seeded with the pack-4 settlement
  fund. Structure tests (counts, ≥2 Food sellers, spares vacant,
  unemployed count). Done: `./scripts/check.sh` clean; `cargo test
  worldgen` green. Touches: src/engine/mod.rs, src/engine/worldgen.rs,
  src/engine/game_loop.rs.
- [x] **3. The conservation re-pin (one deliberate item).**
  `town_world_seeds_the_decided_metals` pins the new per-metal totals;
  audit green on the fresh world. Done: `./scripts/check.sh` clean;
  `cargo test money::` and `cargo test market::` output quoted in the
  ledger. Touches: src/engine/worldgen.rs.
- [x] **4. Ship it.** The binary boots `town_world` (`template_world`
  stays the fixture); the aggregated feed lands. Done: `./scripts/check.sh`
  clean; frame confirmed readable by eye at 28 agents. Touches:
  src/engine/game_loop.rs.
- [x] **5. 100-tick soak + tuning.** Headless soak test from `town_world`:
  audit green every tick (tick panics otherwise); from tick 10 — every
  agent ≥1 Food purchase in every rolling 5-tick window; per Good the
  cheapest posted price neither floor-pinned all span nor monotonically
  rising; at least one price moves in both directions. Constants iterated
  until green, then frozen. Done: soak test green in `cargo test`; final
  constants recorded in the ledger. Touches: src/engine/worldgen.rs.
- [x] **6. Pack close.** `./scripts/verify.sh` green; review pass; ledger
  quoting real output and the new count; container entry updated. Done:
  `VERIFY OK — fmt, clippy, build, tests all clean.` quoted with count.
  Touches: docs/manifests/*.

## Not in this pack unless you say so

- No hiring, quitting, or wage adjustment — phase 1 stays a stub until
  pack 3; the seeded unemployed stay unemployed all pack.
- No `Event` variants added or changed — pack 1's six stand.
- No goods-table changes (07-19 pinned) unless the soak forces it, and
  then only as a recorded deviation.
- No migration, no `immigrate`, no hunger counter — pack 4.
- No INVENTORY body rewrite — the 🚧 pointer stands until container DONE.

## Ledger

- **2026-08-21** — **drafted and started.** Go given ("start pack 2").
  Items 1–6 as above; baseline 114 tests green.
- **2026-08-21** — **item 1 landed (b0286b1).** `employees_of` replaces
  `employee_of`; produce scales by staff; payroll pays every slotted
  employee ascending. `CHECK OK`; 116 passed; `world::`/`sim::`/
  `market::`/`money::` green within the full run.
- **2026-08-21** — **item 2 landed (54aa9b9), with one deviation pulled
  forward:** `run()` boots `town_world` in the same commit — an
  unconsumed worldgen is dead code under the clippy `-D warnings` gate
  (same forcing as pack 1's item 1/2 pairing). `template_world` became a
  `#[cfg(test)]` fixture. `CHECK OK`; 117 passed.
- **2026-08-21** — **item 4 landed (0f808ea).** Aggregated feed
  ("Longacre Farm paid 3 workers 210g"), per-agent ledger left the frame
  (roster carries it). Confirmed readable by eye at town scale.
- **2026-08-21** — **items 5+3 landed (208c9cf) — the tuning saga,
  measured, and two DEVIATIONS from the drafted shape:**
  1. *Six businesses, not the spec's 7–9.* Three same-good sellers cannot
     all stay solvent under the shipped market mechanics: at the floor
     the houses-order tie-break routes all demand to the first seller
     forever (the floor can't be undercut; loser-lowers re-undercuts
     before a third's turn recurs) — traced live: Stonefield Farm at
     floor with 1360 unsold food by t20, its workers' wallets at zero by
     t10. Two sellers per good is the maximum the mechanics admit; the
     spec's business count and its pinned soak criteria are jointly
     unsatisfiable at three. Flagged to the owner (PR) as a spec
     correction candidate rather than silently absorbed.
  2. *16 employed / 14 unemployed, not 24/4.* Exact supply=demand
     clearing lets rotation sell-outs ratchet prices until they tax every
     wage to zero (traced: whole payrolls at 0g by t40); and an ent/lux
     staffer's output value at the floor (20g/8g per tick) is below any
     livable wage, so parity staffing bankrupts those venues (arrears
     5,700g by t75, traced). Frozen regime: Food runs mild surplus with
     rotation (universal liveness); ent/lux run deliberate scarcity,
     priced above the floor where revenue covers payroll. The larger
     unemployed pool becomes pack 3's hiring stock.
  Frozen constants: pop 30, staff 4+4/2+2/2+2, `EMPLOYED_WALLET` 120,
  `UNEMPLOYED_SAVINGS` 4000, `WAGE_BILLS_SEEDED` 3, shelves 2 ticks deep,
  `SETTLEMENT_FUND` 600. Item-3 re-pin: gold 60272 / silver 300 / copper
  600 (`town_world_seeds_the_decided_metals`). Item-gate output, quoted:
  `money::` "18 passed; 0 failed", `market::` "12 passed; 0 failed".
- **2026-08-21** — **3-lens review (contract / invariants /
  economy-quality): no blockers; applied (72a4085):** criterion 3
  tightened to one-price-both-directions (the aggregate form was a silent
  weakening); criterion 2's series re-sampled to the prices in force
  during the evaluated span; four doc corrections. Reviewers confirmed:
  employees_of semantics exact, shared-coffer payroll preserved, re-pin
  arithmetic independently verified, no §8 surface touched.
- **2026-08-21** — **pack closes.** `VERIFY OK — fmt, clippy, build,
  tests all clean.` 119 passed, 0 failed.
