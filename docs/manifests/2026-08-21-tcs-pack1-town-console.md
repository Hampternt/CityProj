# Town colony sim — Pack 1: Town console

**Status:** IN PROGRESS — drafted 2026-08-21, go given same day ("start
pack 1"). Baseline on arrival: `VERIFY OK — fmt, clippy, build, tests all
clean.` 105 passed.
**Container:** [2026-08-21-town-colony-sim.md](2026-08-21-town-colony-sim.md)
**Spec contracts executed here:** `sim::tick` (modified), `Event`/`TickReport`
(new), Amendment 15.

Observable (verbatim from the container): on the unchanged 4-agent scenario,
each Enter prints a narrated feed instead of a static ledger dump —
"Greenrow Farm produced 40 food", "Greenrow Farm paid alice 35g", "bob
bought 5 entertainment @ 2g" — under a header showing tick, population,
employed count, and per-metal totals; `roster` lists every agent, and
inspecting a name shows its last 3 events.

## State on arrival (measured 2026-08-21)

| What | Measured |
|---|---|
| Test suite | 105 passed, 0 failed (`verify.sh`) |
| `sim::tick` | `pub fn tick(world: &mut World)` at sim.rs:39; phases 2/3/4/5 live, 1/6/7/8 stubs |
| `Intent` | one variant (`Buy`), decide→apply worked template in `goods_market` |
| Shell | `render` (money/houses/agents dump), `inspect` (agents only, waits for Enter), commands: Enter/q/map/name |
| Worldgen | `template_world`: 4 agents, 3 businesses, gold 245 / silver 40 / copper 80 pinned by `template_world_seeds_the_decided_metals` |

## Decisions (this pack's, within the spec's contracts)

- **Event variant fields** (spec: plan-owned): `Produced { business, good,
  units }`, `WagePaid { business, worker, amount }`, `PayrollShort
  { business, worker, remaining }` (what stays owed after this tick's
  payment), `Sold { business, buyer, good, units, price }` (per applied
  intent, at the transacted price), `PriceMoved { business, good, from, to }`
  (emitted only when the write-back changes the price — a held price is not
  an event), `WentHungry { agent }` (Food inventory below one tick's
  `consumption_rate` at consume time — the pack-4 counter's exact trigger,
  event-only here).
- **Threading** (spec: mechanical): live phases gain a `&mut TickReport`
  param; stub phases keep `fn(&mut World)` until they gain behavior — the
  amendment's "phase fns thread the report" applies as each phase goes live.
- **Event order** is pinned by construction: phase order, then each phase's
  existing iteration order (businesses in houses order, intents in agents
  order) — no sorting pass.
- **Feed rendering names**: a business renders as its house address, an
  agent as its name; gold amounts render `35g` in the feed (the compact
  form's gold column; feed events are all-gold this milestone).
- **Last-3-events buffer lives in the shell** (presentation state, not
  world state): per-agent `Vec<String>` capped at 3, fed from each tick's
  report; an agent's events are WagePaid/PayrollShort (as worker), Sold (as
  buyer), WentHungry.
- **`roster` and business-inspect resolution in the shell**: `roster` is a
  keyword (shadows an agent literally named "roster", same caveat as `map`);
  an inspect name that matches no agent is tried as a house address
  (case-insensitive) and shows that house's business.

## Items

- [ ] **1. `Event` + `TickReport`; `tick` returns the report (Amendment
  15).** New types in sim.rs; `pub fn tick(world: &mut World) ->
  TickReport`; live phases take `&mut TickReport` (emitting nothing yet);
  audit stays unconditionally last and emits nothing; shell caller updated.
  Done: `./scripts/check.sh` clean; `cargo test sim::` green including
  `tick_runs_audit_last` unchanged. Touches: src/sim.rs,
  src/engine/game_loop.rs.
- [ ] **2. Emissions from the live phases + their tests.** produce →
  `Produced`; pay_wages → `WagePaid`/`PayrollShort`; goods apply → `Sold`;
  price write-back → `PriceMoved` (only on change); consume → `WentHungry`.
  Unit tests per phase on fixture worlds + `tick_report_is_pure_observation`
  (identical fixtures, report dropped on one, state digests equal). Done:
  `./scripts/check.sh` clean; `cargo test sim::` and `cargo test market::`
  green (tick-order/market-adjacent item rule). Touches: src/sim.rs.
- [ ] **3. Shell frame rework: header + feed.** Header line gains
  population and employed/unemployed counts; the last tick's rendered feed
  prints above the ledger view; tick 0 shows a quiet placeholder. Done:
  `./scripts/check.sh` clean; behavior confirmed by eye in `cargo run`
  (presentation — no unit tests, per the shell's existing convention).
  Touches: src/engine/game_loop.rs.
- [ ] **4. `roster` + inspect extensions.** `roster` command (one line per
  agent: job, employer, gold, pantry); inspect resolves businesses by house
  address; agent inspect shows the last 3 events from the shell buffer.
  Done: `./scripts/check.sh` clean; confirmed by eye in `cargo run`.
  Touches: src/engine/game_loop.rs.
- [ ] **5. Pack close.** `./scripts/verify.sh` green; ledger entry quoting
  real output and the new test count; container manifest pack entry updated.
  Done: `VERIFY OK — fmt, clippy, build, tests all clean.` quoted with
  count. Touches: docs/manifests/*.

## Not in this pack unless you say so

- No worldgen changes — `template_world` stays the shipped scenario until
  pack 2.
- No `employees_of` — phases keep `employee_of`; the rename is pack 2's
  first item.
- No stored hunger counter — `WentHungry` is event-only; `Agent.hunger` is
  pack 4.
- No event variants beyond the six above — hired/quit/wage-moved are pack
  3, arrived/departed pack 4.
- No INVENTORY body rewrite — the 🚧 pointer stands until container DONE.

## Ledger

- **2026-08-21** — **drafted and started.** Go given with the gate signing
  ("start pack 1"). Items 1–5 as above; baseline 105 tests green.
