# Town colony sim — Pack 4: The town breathes

**Status:** IN PROGRESS 2026-08-21 — drafted on the owner's "start pack 4".
**Container:** [2026-08-21-town-colony-sim.md](2026-08-21-town-colony-sim.md)
**Spec contracts executed here:** `World::remove_agent` (settle-then-sweep,
gate ruling 2 / Amendment 17 ACTIVE), `World::immigrate`,
`Intent::{Arrive, Depart}`, `Agent.hunger` + consume rule (gate ruling 3),
Amendment 16 (phase 1's money-op row gains the grubstake transfer) and
Amendment 17 (phase 7's row gains the settlement transfer) — the only
money-op widenings in the container. Closing this pack closes the
container: the INVENTORY 🚧 pointer folds into real entries.

Observable (verbatim from the container): population moves both directions
over a 200-tick soak — "petra left town (took 12g 3s 5c)", "Mara arrived
seeking work" — arrivals take vacant residences and apply next tick;
immigration visibly stalls when External drains or no residence stands
empty; audit green throughout.

## State on arrival (measured 2026-08-21)

| What | Measured |
|---|---|
| Test suite | 136 passed, 0 failed (pack-3 close `verify.sh` at 401528c) |
| Phases 6/7/8 | TODO stubs; phase 7's only sanctioned money ops after Am. 17: burn, transfer→External, business→leaver settlement |
| The demand fuse (pack-3 close review, 300-tick probe) | The frozen equilibrium is ~30% dis-saving-financed: coffers absorb ~90g/tick (no profit distribution until phase 6), the 9 permanently unemployed dis-save ~25g/tick each from 4000g — **hunger reaches them from ~t150, first quit ~t200**. This pack's 200-tick soak window is built to catch exactly that: the destitute emigrate, the demand shock bites venue payrolls, quits open slots, and the pull rule answers with grubstaked arrivals |
| Vacancy geography | The 9 permanent unemployed live 3 in "3 Orchard Row" (with 5 employed) and 6 in "4 Orchard Row" (alone) — their departure empties 4 Orchard Row entirely, a third landing pad beside the two seeded spare cottages |
| External | 600g settlement fund, seeded pack 2, untouched since |
| `House.owners` | Stored ownership lists; every shipped house seeds `owners: vec![]`, so the strip is exercised only by tests until worldgen assigns owners |

## Decisions (this pack's, within the spec's contracts)

- **`Agent.hunger: u8`, single writer consume** (gate ruling 3 — throwaway
  stopgap): a tick where Food inventory < one tick's `consumption_rate`
  saturating-increments it (same tick as the `WentHungry` event); a
  fully-fed tick resets it to 0. Read by phase 7's Depart rule and the
  shell (inspect gains a hunger line; fed agents show nothing).
- **Depart rule (phase 7 decide, pure over the phase-start snapshot)**:
  `hunger ≥ H && gold < the cheapest posted Food price` (destitution — they
  could not buy food even if they wanted to), agents in `world.agents`
  order. H = `DEPART_HUNGER_TICKS`, first guess 5, soak-tuned. No Food
  seller in the world → nobody departs by this rule (no price to be below;
  unreachable in shipped worlds, documented).
- **Depart apply = `remove_agent`, and its event carries the NAME**:
  after removal the id resolves to nothing, so
  `Event::Departed { name: String, took: Vec<(Metal, Money)> }` snapshots
  what the shell needs (`took` = the per-metal sweep amounts, settlement
  included — zero metals still listed, D3 visible-zeros precedent).
  `Event::Settled { business, agent, amount }` precedes it when arrears
  settle (Amendment 17 is a money-op row change; it narrates). A written-off
  remainder is silent bookkeeping, consistent with `owed_to` being
  bookkeeping-only — the preceding `PayrollShort` events already told that
  story.
- **`remove_agent` order of operations**: validate (a real spawned agent —
  not reserved, not a business id; `Err(WorldError::UnknownAgent)` = nothing
  changed), then per business in houses order settle
  `min(gold coffer, owed_to[leaver])` via `pay` and remove the entry
  (remainder written off), then sweep every `Metal::ALL` balance to
  External via `pay`, then clear `home`/`workplace`/`employed_role`, strip
  the leaver from every `House.owners`, and remove the `Agent` from
  `world.agents`. After validation no internal `pay` can fail — every
  amount is `min`-bounded by a live balance — so the command is atomic by
  construction (documented, not `unwrap`-hoped: each transfer's error arm
  is unreachable and says why).
- **Arrive rule (phase 1 decide), at most ONE arrival per tick**: fires
  when some (business, role) slot has been open `K` consecutive ticks
  (K = `VACANCY_PULL_TICKS`, first guess 3, soak-tuned), AND a vacant
  residence exists (zero occupants, hosts no business — v1's whole rule),
  AND External's gold covers `GRUBSTAKE` at snapshot. Target home:
  the lowest-`HouseId` vacant residence. One per tick keeps the pass
  deterministic and the External drain legible; the queue of ticks does
  the rest.
- **The K-counter lives on `RoleSlot`** (`unfilled_ticks: u32`): phase 1's
  write-back is its single writer — incremented when the role still has
  `open_slots > 0` post-matching, reset to 0 when fully staffed. Sim
  state, not presentation; worldgen seeds 0 (the boot-time openings age
  from tick 1). Recorded here per the spec's lift-back-if-shared note —
  it stays single-writer/single-reader inside phase 1's file.
- **Immigrant names: fixed table + counter on `World`**:
  `World.arrivals: u32`, incremented by `immigrate` itself (the arrival
  command is the natural single writer); the Arrive decide names the
  newcomer `IMMIGRANT_NAMES[arrivals % LEN]`, suffixed with a generation
  number on wraparound ("Mara 2") — deterministic, no RNG. External's
  600g / grubstake caps real arrivals far below LEN anyway.
- **`World::immigrate(name, home)`**: validates the house exists
  (`UnknownHouse`) and is a vacant residence — zero occupants AND hosts no
  business — rejected otherwise with the single new variant
  `WorldError::HouseNotVacant(HouseId)` (one case, per the spec's
  lift-back note). Builds on the untouched 3-arg `spawn_agent` (07-13
  pin): next id, housed at `home`, unemployed, empty inventory, zero
  balances, hunger 0. Moves no money; bumps `arrivals`.
- **Arrive apply**: re-checks the home is STILL vacant (live), calls
  `immigrate`, then attempts the full `GRUBSTAKE` External→agent gold
  transfer via `pay` — a refused stake (External drained since snapshot)
  leaves a penniless-but-valid newcomer, never a partial or a
  conservation break (§8.5; pinned by test). `Event::Arrived { agent,
  name, home }` — "Mara arrived seeking work". Arrivals join the
  applicant pool next tick (they were not in the phase-start snapshot).
  GRUBSTAKE first guess 100 (≈4 ticks of living costs — enough to reach
  the next hiring pass fed), soak-tuned.
- **Phase-1 apply order**: quits, hires, arrivals, then the wage
  write-back (which also ages/resets `unfilled_ticks`). Event order
  follows.
- **Both new intents die cleanly on stale facts**: Depart re-checks the
  agent still exists (and re-reads hunger/destitution live? No — decide
  decided; apply only re-checks existence, mirroring the goods template's
  "intents don't outlive facts" minimalism: hunger/wallet cannot improve
  between phase-7 snapshot and apply since no phase runs between).
  Arrive re-checks vacancy (an earlier arrival this tick cannot exist —
  one per tick — but the rule is cheap and future-proof).
- **The soak union grows to four**: pack-2 100-tick (unchanged criteria +
  zero quits — the fuse must not bite before t100), pack-3 50-tick
  employment, pack-4 200-tick breathing, and `n_ticks_run_clean`. The
  200-tick soak asserts: population drops below 30 (the destitute leave),
  at least one `Arrived` (it also rises — both directions), every
  departure's sweep leaves no orphan balance (per-account, spot-checked
  via total agent count × savings arithmetic is NOT enough — assert
  `balance_of` zero for a departed id captured from the event stream),
  audit green every tick (implicit in `tick`), and External's gold never
  exceeds its seed + swept amounts (sanity). Tuning levers: H, K,
  GRUBSTAKE, `SETTLEMENT_FUND`, and if needed `UNEMPLOYED_SAVINGS` (which
  moves the fuse — but it must stay ≥ ~2600 so nobody departs before the
  100-tick soak's window closes).
- **INVENTORY fold is the container's DONE** (item 6): the 🚧 pointer
  becomes real entries, and the stale pre-container prose (the 4-agent
  scenario, "labor allocation not simulated", the 2026-08-15 inspect-wipe
  note that pack 1 fixed) is rewritten to what ships.

## Items

- [ ] **1. Hunger.** `Agent.hunger: u8` (spawn/immigrate start 0);
  consume's single-writer rule (increment beside the existing
  `WentHungry` emission, reset on a fed tick); shell inspect shows it.
  Tests: saturating increment, reset-on-fed, starts-at-zero. Done:
  `./scripts/check.sh` clean; `cargo test sim::` green. Touches:
  src/agent.rs, src/sim.rs, src/engine/game_loop.rs.
- [ ] **2. `World::remove_agent`.** Settle-then-sweep per the decision
  above; `#[allow(dead_code)]` until item 4 wires phase 7 (recorded).
  Tests: `remove_agent_sweeps_every_metal_no_orphans` (per-account:
  leaver zero on every metal, External credited exactly the pre-sweep
  balances), settlement `min(coffer, owed)` with write-off,
  reserved/business/unknown ids rejected with nothing changed,
  owners-strip, home/workplace/role cleared, derived occupancy updates.
  Done: `./scripts/check.sh` clean; `cargo test world::` + `money::`
  green. Touches: src/world.rs.
- [ ] **3. `World::immigrate`.** Vacancy-validated wrapper over the
  untouched `spawn_agent`; `HouseNotVacant`; `arrivals` counter.
  `#[allow(dead_code)]` until item 4 (recorded). Tests:
  `immigrate_is_money_free` (zero balances, all metals), occupied and
  business-bearing houses rejected + unknown house, newcomer housed and
  unemployed, `spawn_agent` call sites untouched (compile-time fact,
  noted not tested). Done: `./scripts/check.sh` clean;
  `cargo test world::` green. Touches: src/world.rs.
- [ ] **4. The town breathes.** `Intent::{Depart, Arrive}` +
  `Event::{Settled, Departed, Arrived}`; phase 7's decide→apply
  (destitution rule → settle→sweep→remove); phase 1 gains the pull rule
  (K-counter on `RoleSlot`, name table, capped grubstake — Amendment 16
  executed); exhaustive-match arms ripple (both applies, the shell);
  constants H/K/GRUBSTAKE first guesses. Tests: emission tests for all
  three events; `stake_is_capped_and_failure_leaves_a_valid_newcomer`;
  `immigration_halts_on_drained_external_and_on_zero_vacancy` (both
  designed bounds, constructed worlds); departed agent's slot ages into
  a pull; hire-next-tick for an arrival; phase-1 order pin extended.
  Done: `./scripts/check.sh` clean; `cargo test sim::`, `world::`,
  `market::`, `money::` output quoted in the ledger. Touches:
  src/sim.rs, src/business.rs, src/world.rs, src/role.rs (if needed),
  src/engine/game_loop.rs.
- [ ] **5. The 200-tick soak + tuning.** As decided above; constants
  iterated until the FOUR-soak union holds, then frozen; deviations (if
  any) recorded and flagged. Done: full suite green; final constants +
  the breathing timeline (first departure, first arrival, final pop)
  quoted in the ledger. Touches: src/engine/worldgen.rs, src/sim.rs
  (constants).
- [ ] **6. Container close.** `./scripts/verify.sh` green; 3-lens
  review; INVENTORY 🚧 folded into real entries (and its stale prose
  corrected); CLAUDE.md code-state updated (phases 6/8 the remaining
  stubs, migration live); pack + container ledgers closed quoting real
  output; PR body updated; container Status → DONE. Done: `VERIFY OK`
  quoted with count. Touches: docs/INVENTORY.md, docs/manifests/*,
  CLAUDE.md.

## Not in this pack unless you say so

- No firm founding/closure/ownership — the next milestone (gate ruling 1);
  phase 6 stays a stub and coffers stay one-way sinks (the fuse is
  recorded, not fixed — migration is the relief valve, profit
  distribution is the cure and it is out of scope).
- No burn/demurrage, no imports — phase 7 gains ONLY the departure path;
  phase 8 stays inert.
- No housing market, rent, or house capacity; ownership plays no part in
  vacancy (v1 rule stands).
- No hunger consequences beyond the Depart rule — no health, no death.
- No wage/price mechanics changes — pack 3's market is frozen; this pack
  only adds the `unfilled_ticks` bookkeeping beside the write-back.
- No new soak criteria for packs 2–3's tests beyond what their unions
  already assert.

## Ledger

- **2026-08-21** — **drafted and started.** Go given ("start pack 4").
  Items 1–6 as above; baseline on arrival at 401528c:
  `VERIFY OK — fmt, clippy, build, tests all clean.` 136 passed, 0 failed.
- **2026-08-21** — **item 1 landed (2f091d2).** `Agent.hunger` behind the
  existing `WentHungry` emission, single writer consume; shell inspect
  shows it. `CHECK OK`; `sim::` 40 passed.
- **2026-08-21** — **item 2 landed (6984460).** `remove_agent`:
  settle-then-write-off (all three coffer cases pinned), per-metal
  no-orphan sweep asserted per-account, reserved/business/unknown ids
  refused with nothing changed, owners-strip + derived-link updates.
  `CHECK OK`; `world::` 28 / `money::` 18 passed.
- **2026-08-21** — **item 3 landed (e98bece).** `immigrate`:
  vacancy-gated (`HouseNotVacant` — one variant covers occupied and
  business-bearing), money-free on every metal, wraps the untouched
  constructor, bumps the `arrivals` name counter. `CHECK OK`;
  `world::` 30 passed.
- **2026-08-21** — **manifest verification (first lens, contract
  fidelity) applied mid-implementation:** the Arrive apply now re-checks
  live labor demand (any open slot) so the boot cascade's race dies like
  every stale intent — measured before the fix: "Mara" arrived t4 into
  the still-hiring boom, found the slots taken by lower-id locals, spent
  her 100g stake and left penniless t22; after: no boot arrival, the
  first arrival is the real one (~t182). `Event::Departed` gains the
  `agent` id (names are not enforced unique; the soak harvests ids).
  The reviewer's fuse-timeline arithmetic confirmed: at savings 4000
  the arrival chain lands past the window — savings 3400 (first
  departure ~t127 > the 100-tick criteria span; whole chain < 200) and
  the conservation re-pin moves to gold 52148. Also recorded: the
  Depart rule reads each Food seller's live posted price at phase-7
  start, stock ignored (a sold-out seller's price still counts); the
  spec's `immigrate_is_money_free_and_stake_is_capped` acceptance test
  is split across items 3/4 on the module boundary; `Departed.took` is
  pinned to `Metal::ALL` order by its emission tests; `Settled` renders
  impersonally ("paid out Ng of back wages to a leaver") — the Departed
  line immediately after carries the name; a leaver's earlier same-tick
  events (their last `WentHungry`) render "(unknown agent)" in the
  post-tick frame — accepted and recorded, revisit if the close review
  objects.
- **2026-08-21** — **manifest verification (second lens, mechanics)
  reconciled — its blocker is structurally prevented, its hardenings
  taken:** the double-pull it flagged (one aged vacancy pulling two
  immigrants) cannot occur under the shipped apply order — the first
  immigrant's TakeJob applies BEFORE the next tick's Arrive, so when
  the last open slot fills, the second arrival dies on the live
  labor-demand re-check; with several genuine vacancies, multiple
  arrivals are the correct behavior (reasoned here, exercised by the
  200-tick churn). Taken: the 200-tick soak now requires an arrival
  AFTER the first departure (the pull answering the shock — a boot
  transient cannot satisfy it); the shell drops a leaver's history
  buffer (leak); the phase-7 price-reading note (live posted price, one
  write-back ahead, stock ignored) moved into the code. On its
  destitution-vacuity concern (post-spend wallets make `gold <
  cheapest` near-universal for the working poor): the rule is the
  spec's sentence verbatim, and the hunger clause is the real gate —
  an employed worker who eats resets to 0 daily; measured across all
  four soaks, no spurious eviction of the employed. The wallet clause
  bites exactly for the moneyed-but-starving (supply shortage), who
  rightly stay. Recorded, no change. A zero-amount settlement emits no
  `Settled` (held-price precedent, guarded and tested).
- **2026-08-21** — **items 4+5 landed (e32c340), one commit — the
  breathing chain measured end to end:** boot cascade t1–4 (no
  spurious arrival); first destitute departure t127, the nine
  unemployed leave through ~t173 with silver/copper visibly swept; the
  demand shock guts The Brass Bell — quits at ~144g arrears from t174,
  wages ratchet 39→36→33, settlements pay out on churn departures
  (117g, 132g — Amendment 17 narrated); the K-aged vacancy pulls
  "Mara" at t182, hired at t183; External drains per stake. Population
  30 → 24 and rising ticks observed. All four soaks green:
  `cargo test` 151 passed, 0 failed. Constants frozen: H=5, K=3,
  GRUBSTAKE=100, savings 3400.
