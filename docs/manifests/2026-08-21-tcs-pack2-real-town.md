# Town colony sim — Pack 2: A real town

**Status:** IN PROGRESS — drafted 2026-08-21, go given same day ("start
pack 2"). Baseline on arrival: `VERIFY OK — fmt, clippy, build, tests all
clean.` 114 passed (pack 1's close gate).
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

- **`town_world` scenario shape**: 28 agents in 14 houses — 4 occupied
  residences (7 each), 2 zero-occupant spare residences (pack 4's landing
  pads), 8 business premises. Businesses over `Good::ALL`: 3 Food farms
  (the competing sellers), 3 Entertainment venues, 2 Luxury ateliers, all
  `headcount > 1`. 24 agents seeded employed (workplace + employed_role
  set directly, template precedent), 4 seeded unemployed.
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

- [ ] **1. `employees_of` replaces `employee_of`.** New
  `pub fn employees_of(&self, house: HouseId) -> Vec<AgentId>` (derived
  scan, ascending, unknown house empty); `employee_of` deleted; produce
  scales stock gain by staff count; pay_wages accrues and pays every
  slotted employee (per-accrual payable re-read keeps the shared-coffer
  semantics); inspect's workers line lists all. Tests: ordering pin,
  multi-worker produce/payroll, updated callers. Done: `./scripts/check.sh`
  clean; `cargo test world::`, `sim::`, `market::`, `money::` green.
  Touches: src/world.rs, src/sim.rs, src/engine/game_loop.rs.
- [ ] **2. Worldgen split + `town_world`.** `engine/worldgen.rs` with
  `template_world` (moved verbatim) and `town_world` per the contract:
  deterministic, seedless, no RNG; the shape decided above; every business
  seeded one full-staffing `wage_bill()`; every agent wallet + one day's
  goods + inert silver/copper; External seeded with the pack-4 settlement
  fund. Structure tests (counts, ≥2 Food sellers, spares vacant,
  unemployed count). Done: `./scripts/check.sh` clean; `cargo test
  worldgen` green. Touches: src/engine/mod.rs, src/engine/worldgen.rs,
  src/engine/game_loop.rs.
- [ ] **3. The conservation re-pin (one deliberate item).**
  `town_world_seeds_the_decided_metals` pins the new per-metal totals;
  audit green on the fresh world. Done: `./scripts/check.sh` clean;
  `cargo test money::` and `cargo test market::` output quoted in the
  ledger. Touches: src/engine/worldgen.rs.
- [ ] **4. Ship it.** The binary boots `town_world` (`template_world`
  stays the fixture); the aggregated feed lands. Done: `./scripts/check.sh`
  clean; frame confirmed readable by eye at 28 agents. Touches:
  src/engine/game_loop.rs.
- [ ] **5. 100-tick soak + tuning.** Headless soak test from `town_world`:
  audit green every tick (tick panics otherwise); from tick 10 — every
  agent ≥1 Food purchase in every rolling 5-tick window; per Good the
  cheapest posted price neither floor-pinned all span nor monotonically
  rising; at least one price moves in both directions. Constants iterated
  until green, then frozen. Done: soak test green in `cargo test`; final
  constants recorded in the ledger. Touches: src/engine/worldgen.rs.
- [ ] **6. Pack close.** `./scripts/verify.sh` green; review pass; ledger
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
