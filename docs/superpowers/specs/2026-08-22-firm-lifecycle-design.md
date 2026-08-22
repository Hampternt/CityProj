# Firm Lifecycle — Design Spec

**Status:** DRAFT — awaiting the approval gate ·
**Parent:** [2026-07-02-money-gameloop-skeleton-design.md](2026-07-02-money-gameloop-skeleton-design.md)
(grandparent: [2026-06-20-economy-sim-design.md](2026-06-20-economy-sim-design.md));
successor to [2026-08-21-town-colony-sim-design.md](2026-08-21-town-colony-sim-design.md)
per its gate ruling 1 ("firm lifecycle is the immediately-following milestone").
**Shape:** container — three packs, one level deep; the container manifest
(`docs/manifests/`) is written from this spec's Contracts after sign-off.
**Branch:** `claude/town-colony-sim-p1s06q`

## Design (decisions — approve before reading on)

Firms stop being immortal coffers: every business gets a required living
**owner**, phase 6 wakes, and the three lifecycle transitions land — the
**profit draw** (each tick a business transfers its gold surplus above a
retained buffer to its owner — the recorded cure for the dis-saving fuse,
whose coffers today absorb ~90g/tick as one-way sinks), **closure** (a
business whose wage arrears persist unpaid for a run of ticks liquidates:
back wages settled at `min(coffer, owed)` per creditor, staff laid off,
every metal of the residual swept to the owner, the emptied house becoming
an immigration landing pad by the existing vacancy rule), and **founding**
(`Intent::Found` on the decide→apply template: when a good has fewer than
two sellers and the survivor's price signals live scarcity — or no seller
stands at all — the first capitalized unemployed resident stakes three wage
bills of their own gold into a new venue on a vacant house and self-hires
as owner-operator). An emigrating owner's firms are force-liquidated inside
`remove_agent` before the Amendment-17 settlement and sweep — the one place
this milestone's money ops exceed a standing phase row (Amendment 19). The
phase-1 vacancy pull gains an arrears exclusion, resolving the recorded
deadbeat-recruitment handoff by rule rather than timing. Everything stays
deterministic without RNG (ascending-`AgentId`, `Role::ALL`/`Metal::ALL`/
houses-order iteration), every money movement rides `World::pay`, and the
audit pins the per-metal totals throughout — the draw, the stake, and both
liquidation paths are transfers, never mint or burn. **The draw re-times
every frozen measurement** (fuse timeline, the 21/30 employment ceiling),
so pack 1 deliberately re-measures and re-pins before any later criterion
freezes — and the spec expects the cure to be partial: `target_days`
consumption caps mean draws largely pool as founding capital rather than
demand, which is precisely what the founding loop spends.

- **Fits into:** the phase-6 `invest` TODO stub and the `Intent`
  decide→apply template in `src/sim.rs` (@src/sim.rs:836); `Business` in
  `src/business.rs` (gains `owner`, `insolvent_ticks`); the command layer
  in `src/world.rs` (`create_business` widens, `found_business` /
  `close_business` join `immigrate`/`remove_agent`, `remove_agent` gains
  forced liquidation); `market.rs` gains the founding-choice logic
  (`plan_founding` — entry choice ranks market signals, §8.6) while the
  draw/closure threshold constants live beside their fns in `sim.rs` (the
  `QUIT_ARREARS_BILLS` precedent — retention depth and insolvency age are
  firm bookkeeping, not pricing; stated here so the plan never re-litigates
  the split); phase 1's Arrive decide (@src/sim.rs:311) gains the arrears
  exclusion; `worldgen.rs` seeds owners (one deliberate reorder + re-pin
  item); the shell narrates the new events.
- **Invariants:** §8.1 (every new amount is `Money`; all threshold
  arithmetic integer); §8.2 (draw, stake, settlements, and both residual
  sweeps move only via `World::pay` → `Accounts::transfer` — no new movers;
  `found_business` is money-free like `immigrate`, the stake is a separate
  capped pay in the apply); §8.3 (per-metal audit unconditionally last
  across every draw, closure, and founding; liquidation completeness is
  proven by per-account no-orphan assertions on the dead business id — and,
  in the forced path, on the removed agent too — never by the totals-only
  audit, which cannot see a conservation-legal orphan); §8.4 untouched (no
  mint or burn anywhere in this container; the faucet stays closed); §8.5
  (the stake is refused whole if the founder's live wallet cannot cover it;
  the draw and every settlement are min-bounded by live balances); §8.6
  (founding choice logic in `market.rs`; pricing/wage write-backs
  untouched). **Pinned reading, so it is never re-litigated:** the 07-02
  phase table's row 6 money-op entry "`transfer` only" is
  direction-unrestricted — rows 3 and 4 carry the identical wording over
  fixed-direction flows without naming directions — so the draw
  (business→owner), the founding stake (founder→business), and steady-state
  closure's settlements and residual (business→workers, business→owner) all
  ride the existing allowance. Only phase 7's row is direction-named, which
  is why the forced-liquidation path needs Amendment 19 and nothing else
  needs any money-op amendment.
- **Out of scope (YAGNI):** ownership transfer, sale, or inheritance
  between living agents (owner departure = liquidation, full stop); shares,
  partnerships, multiple owners per firm (one `AgentId`, though one agent
  may own several firms); banking/credit/debt (the stake is cash on hand);
  engaging `House.owners` in any rule (it stays real-estate bookkeeping,
  re-recorded for a housing-market milestone — `Business.owner` is a
  distinct fact and the two may disagree); owner salary-vs-dividend
  distinctions (the draw is the whole return); voluntary job-switching and
  employee happiness (roadmap, unchanged); multi-role or multi-good founded
  firms (single `Labourer` slot, mirroring every shipped venue); premises
  construction (founders occupy existing vacant houses; building volumes
  stay roadmap); demand-composition reshaping or new Goods (the 21/30
  ceiling is re-measured here, its composition cause stays the recorded
  owner's-choice follow-up); the wage-ratchet overshoot (untouched,
  re-recorded); speculative third-seller entry (the count gate refuses it
  until the market layer changes the measured two-seat carrying capacity);
  the Mint business, non-gold pricing, terrain, physical goods movement,
  everything in `docs/ideas.md`.

### Amendments to the running series

*(continues the series at 18; 15–17 are executed. 18 is a purpose-text
honesty edit — the money-op column of row 6 is deliberately untouched, per
the pinned reading above; subject to open question 2. 19 is the milestone's
one money-op widening.)*

18. *(subject to open question 2)* 07-02 ⟨REF⟩ phase table, row 6
    (`invest`), **purpose column only**: "expand capacity / take profit" →
    "take profit / found & liquidate firms (capacity expansion still TODO)".
    The money-ops column is untouched — "`transfer` only" already covers
    every phase-6 movement this milestone adds. Numbered because the table
    is the standing contract and three mechanics now hang on that row.
19. 07-02 ⟨REF⟩ phase table, row 7 (`sinks`), money-ops column gains:
    "forced-liquidation transfers when the departing agent owns a business —
    business→creditor arrears settlements and business→departing-owner
    per-metal residual — immediately preceding that owner's Amendment-17
    settlement and sweep to External". Required: phase 7's row is
    direction-named, and the forced path pays third parties (laid-off
    creditors) and pays the leaver amounts that are not arrears (liquidation
    proceeds), neither of which the row covers today.

### The measured baseline this design must respect (from the container's ledgers)

- **Two solvent sellers per good is the market's carrying capacity** — a
  third starves before payroll (floor can't be undercut, ties favor earlier
  houses). Founding therefore never *creates* a third seller: the count
  gate is structural, and the equilibrium town founds nothing (pinned as a
  soak criterion, not hoped).
- **The dis-saving fuse:** coffers absorb ~90g/tick while the 9 permanently
  unemployed dis-save ~25g/tick each. The draw is the recorded cure, but
  `target_days` purchase caps mean owner income mostly pools rather than
  becoming demand — so pack 1 *re-measures* the fuse and the employment
  ceiling and treats large movement as a finding, never a target. Owner-
  wallet pooling is re-recorded as the seam for phase 6's still-TODO
  "expand capacity" half.
- **The deadbeat-pull handoff** (pack-4 ledger): the vacancy pull recruits
  immigrants into arrears-carrying venues. Resolved here by rule — the
  Arrive decide's slot-age scan skips businesses with `owed_total() >
  ZERO` — not by racing closure against the pull: `unfilled_ticks` ages
  independently of arrears (boot openings age from tick 1), so no closure
  threshold can win that race.
- **Post-quit arrears freeze:** quits fire strictly above 3×wage per
  worker, so a dying venue's `owed_total` plateaus near ~3 wage bills and
  then freezes when the last worker walks (no employees, no accrual). A
  closure trigger set as an arrears *level* above that plateau is
  unreachable; the trigger below is an arrears *persistence* counter
  instead.

### Proposed pack sequence (the container manifest is written from this)

Signing the gate approves this sequence as direction; the cut stays
re-cuttable at manifest level without spec re-approval, per standing
practice. Cure first — the draw re-times every measurement the later packs
pin against; death before birth — a mis-founded firm must die by working
closure rules, not zombie.

**Pack 1 — Owners and the draw.** `Business.owner` (required, living);
`create_business` widened with the validated owner param; the one
deliberate worldgen item (staff spawned before their venue so the owner id
exists, owners seeded as each venue's first seeded worker — alice, ed,
ivan, karl, marco, otto — id-pinned tests migrated, per-metal totals
asserted unchanged at gold 52148 / silver 300 / copper 600, which the
reorder cannot move); the phase-6 direct draw pass (`draw_amount` +
`DRAW_BUFFER_BILLS` beside it in sim.rs) with `Event::ProfitDrawn`; shell:
owner on roster/inspect, business count in the header. Ends with the named
re-measure item: all four soaks re-run, the fuse timeline and `NEAR_FULL`
re-pinned, the new numbers quoted in the ledger as the baseline every later
criterion cites. *Observable:* "Greenrow Farm paid alice 62g profit" in the
feed; coffers visibly plateau at the buffer instead of climbing forever.

**Pack 2 — Firms die.** `Business.insolvent_ticks` (single writer: phase
6's write-back); `World::close_business` (settle-ascending →
write-off-including-zero-entries → layoffs → per-metal residual to owner →
detach, atomic); the phase-6 closure pass ordered before draws; the Arrive
decide's arrears exclusion (the handoff resolved); forced liquidation
inside `remove_agent` with Amendment 19 executed and the dedicated
owner-emigration fixture (the shipped town's owners are solvent — the path
is soak-invisible and must be proven on a fixture or it ships untested);
`Event::Closed`/`Event::LaidOff`, `Settled` reused with its doc widened.
*Observable:* on a stress fixture the whole death narrates — quits, then
"The Brass Bell closed — karl pockets 12g" — and the freed address inspects
as a vacant residence; on the tuned town nothing closes (pinned).

**Pack 3 — Firms are born.** `market::plan_founding` + the per-good
founding template and `FOUND_SIGNAL` constants; `Intent::Found` on the
decide→apply template (one per tick, phase-start snapshot, kill-only live
re-checks); `World::found_business` (money-free gated wrapper; the stake is
a separate pay in the apply); the founder self-hires at apply;
`Event::Founded`; the full-cycle soak (closure → the survivor's price
ratchets → founding answers → the labor market staffs it); CLAUDE.md
structure update + the INVENTORY fold — the container's definition of DONE.
*Observable:* after a venue dies, "mira founded a food stall at
5 Weir Cottage (staked 420g)" followed over the next ticks by Hired events
restaffing it; on the tuned equilibrium town, zero foundings (pinned).

## Contracts (the hard reference the plan is written from)

### Business.owner  (business.rs, new field)

`pub owner: AgentId`
Required, never `Option`: every business always names a **living spawned
agent** (never a reserved or business id). The invariant holds by
construction — every creation path takes a validated owner
(`create_business`), and `remove_agent` force-liquidates a leaver's firms
(see below), so no path strips an owner and leaves the field dangling. An
agent may own several businesses. Strictly distinct from `House.owners`
(real-estate bookkeeping, engaged by no rule, untouched here — the shop
house's owners list and its business's owner may disagree). Economically
load-bearing: an ownerless firm is an undrainable sink — the exact
~90g/tick pathology the fuse measurement recorded. Written only at
creation; no transfer mechanic exists (YAGNI).
Refs: @src/business.rs:36, @src/housing.rs:23 (`House.owners`, distinct).

### World::create_business  (world.rs, widened)

`pub fn create_business(&mut self, house: HouseId, owner: AgentId, product: Good, price: Money, roles: HashMap<Role, RoleSlot>) -> Result<AgentId, WorldError>`
Given a real spawned agent as `owner` — validated FIRST, per the
agent-checked-first convention (@src/world.rs:209) — then the existing
house checks (`UnknownHouse`, `BusinessAlreadyExists`) unchanged. Reserved
ids, business ids, and ghosts refuse with `WorldError::UnknownAgent(owner)`.
`Err` means nothing changed. The 07-13 "do not widen the constructor"
precedent is distinguished, not violated: `immigrate` added a behavioral
gate over a constructor whose untouched call sites stayed valid, so a new
name preserved both; here `owner` is a required field of the type — an
un-widened path would keep manufacturing invalid (ownerless) values, a
standing invariant hole no wrapper can fix. Every call site changes
(compile-time forcing, ~17 sites: worldgen ×2, tests).
Refs: @src/world.rs:278, the Business.owner invariant above.

### World::found_business  (world.rs, new)

`pub fn found_business(&mut self, founder: AgentId, house: HouseId, product: Good, price: Money, roles: HashMap<Role, RoleSlot>) -> Result<AgentId, WorldError>`
The tick-time founding command — the `immigrate` wrapper precedent applied
to firms. Validates the founder is a spawned agent and the house is **fully
vacant** (zero occupants AND hosts no business — the `immigrate` predicate
verbatim, `HouseNotVacant` on refusal), then forwards to the widened
`create_business(owner = founder)`. **Money-free**, like `immigrate`
(07-03's refusal of money-moving constructors stands): the capital stake is
a separate `World::pay(founder, new_id, Gold, capital)` in the Found apply,
so a failed stake can never half-found and §8.5 caps it against the live
wallet. `Err` means nothing changed.
Refs: @src/world.rs:411 (`immigrate`, the wrapper precedent), §8.2, §8.5.

### draw_amount + the phase-6 draw pass  (sim.rs, new)

`fn draw_amount(coffer: Money, wage_bill: Money, owed_total: Money) -> Money`
Pure: `coffer − (wage_bill × DRAW_BUFFER_BILLS + owed_total)`, clamped at
`Money::ZERO`. `DRAW_BUFFER_BILLS = 3` provisional — matching
`WAGE_BILLS_SEEDED`, the measured payroll-drought depth worldgen already
encodes — a constant beside its fn (the `QUIT_ARREARS_BILLS` placement
precedent: retention depth is firm bookkeeping, not §8.6 market logic;
stated so the plan never re-litigates it). The `+ owed_total` term is the
pack-3 net-of-arrears erratum applied as formula: **a business owing back
wages draws nothing, ever** — arrears outrank the owner. The pass: every
tick, phase 6, businesses in houses order, `Metal::Gold` only (the sole
trading metal; closure's `Metal::ALL` sweep is the completeness backstop),
one `World::pay(business, owner, Gold, draw)` per positive draw plus
`Event::ProfitDrawn` — a **direct pass**, no intents (the `pay_wages`
precedent: objective per-business state, zero contention). Ordering inside
phase 6: closures → Found apply → draws → counter write-back, so a closing
business never draws and a just-founded firm sits exactly at buffer (draw
zero by construction). Rides row 6's existing "`transfer` only" — no
money-op amendment (pinned reading, Invariants).
Refs: @src/sim.rs:836 (`invest` stub), @src/sim.rs:192
(`QUIT_ARREARS_BILLS` placement), the 08-21 spec's net-of-arrears erratum.

### Business.insolvent_ticks + the closure trigger  (business.rs + sim.rs, new)

`pub insolvent_ticks: u32` (field on `Business`)
Consecutive ticks ending with `owed_total() > ZERO`, measured at phase 6
(post-payroll, post-revenue). SINGLE WRITER: phase 6's write-back —
incremented while owed, reset to 0 when clear; worldgen seeds 0 (the
`RoleSlot.unfilled_ticks` discipline). Closure fires when the phase-start
snapshot shows `insolvent_ticks >= CLOSE_INSOLVENT_TICKS` (6 provisional,
in sim.rs beside the fn) — an arrears-**persistence** trigger, deliberately
not an arrears level: the quit rule caps per-worker debt near 3 bills and
post-quit arrears freeze, so persistence is the one signal that stays
reachable, and the threshold sits beyond the quit horizon so worker churn
(the cheaper correction) always fires first. The tuned town must never trip
it: `insolvent_ticks` pinned 0 for all six seeded firms through the
100-tick soak (the zero-quits precedent applied to firms).
Refs: @src/business.rs:20 (`unfilled_ticks`, the counter precedent),
@src/sim.rs:192, the post-quit-freeze baseline note above.

### World::close_business  (world.rs, new)

`pub fn close_business(&mut self, house: HouseId) -> Result<(), WorldError>`
Given a house that exists and hosts a business —
`Err(WorldError::UnknownHouse)` / a new no-business variant otherwise
(exact variant plan-owned), and `Err` means nothing changed. Then, in
order: **(1) creditor settlement** — every `owed_to` entry (current AND
former workers), keys explicitly sorted ascending `AgentId` (the ledger is
a `HashMap`; the no-RNG guarantee is only as good as pinned iteration),
paid `min(remaining gold coffer, owed)` via `World::pay`, remainders
written off and every entry removed, zero-amount entries included (the
pack-4 dangling-entry bug precedent); **(2) layoffs** — `vacate_workplace`
for every `employees_of(house)` (already ascending), clearing `workplace` +
`employed_role` together; laid-off workers re-enter the applicant pool next
tick; **(3) residual sweep** — every `Metal::ALL` balance of the business
account to the **owner's** wallet via `pay` (liquidation proceeds belong to
a living resident and stay in-node; External is the seam for money
*leaving* the node, which this is not); completeness proven per-account by
no-orphan assertions on the dead id — the totals-only audit cannot see a
conservation-legal orphan; **(4) detach** — `house.business = None`, last,
so `is_known_account` retires the id (any future pay to it refuses) and the
house — zero occupants, hosting nothing — satisfies the immigration vacancy
rule verbatim: closure manufactures landing pads by design. Atomic by
construction after validation: every amount min-bounded by a live balance,
both ids known. Steady-state caller: phase 6's closure pass (all transfers
under row 6). Forced caller: `remove_agent` (Amendment 19).
Refs: @src/world.rs:352 (`remove_agent`, the settle-then-sweep shape
mirrored), §8.2, §8.3 per-account proof obligation, @src/world.rs:176
(`is_known_account`).

### remove_agent gains forced liquidation  (world.rs, amended)

`remove_agent`'s order of operations gains a step 0, after validation and
before the existing Amendment-17 settlement: **for each house in houses
order whose business's `owner` is the leaver, run the `close_business`
procedure** — that firm's creditors settled (the leaver included, if their
own firm owes them), staff laid off, per-metal residual into the *leaver's*
wallet. The existing sequence then runs unchanged: A17 settlement of other
businesses' debts to the leaver, the per-metal sweep (now carrying the
liquidation proceeds) to External, owners strip, removal. Still atomic
post-validation. This is the one path whose money ops exceed a standing
row — Amendment 19 names them. No-orphan proofs must cover BOTH dead ids
(each closed business account and the removed agent) per `Metal::ALL`. The
path is unreachable in the shipped town (owners are seeded employed and
solvent) — it is proven on a dedicated fixture, or it ships untested.
Refs: @src/world.rs:352, Amendment 19, §8.3.

### The Arrive decide's arrears exclusion  (sim.rs, amended)

Phase 1's pull rule (@src/sim.rs:311) gains one conjunct: the slot-age scan
counts only businesses with `owed_total() == Money::ZERO` — an aged slot at
an arrears-carrying venue cannot justify an arrival. The recorded pack-4
handoff resolved in the same grain as the resident deadbeat exclusion
(`plan_application`'s `owed_by`), extended to people who cannot yet know
the employer. Rule, not race: `unfilled_ticks` ages independently of
arrears, so no closure timing can protect an immigrant from a venue whose
slot aged before its insolvency began.
Refs: @src/sim.rs:311, @src/market.rs:178 (the deadbeat exclusion),
the pack-4 ledger handoff.

### market::plan_founding + the founding gates  (market.rs, new)

`pub struct SellerSnapshot { pub good: Good, pub sellers: u32, pub cheapest_price: Option<Money> }`
`pub struct Prospectus { pub good: Good, pub price: Money }`
`pub fn plan_founding(snapshots: &[SellerSnapshot]) -> Option<Prospectus>`
Pure entry choice (§8.6: reading market signals to decide entry IS market
logic — the ranking mirror of `plan_application`). Scans in `Good::ALL`
order and returns the first good satisfying the founding gates:
`sellers < 2` (the measured carrying capacity — founding never creates the
third seller the container proved starves) AND either `sellers == 0`
(unconditional refound — existential: with no Food seller the shipped
Depart decide has no posted price and nobody can even emigrate; a dead
sector must be recoverable) or `cheapest_price >= FOUND_SIGNAL(good)` (the
scarcity gate: a lone survivor facing live demand sells out and ratchets
up toward the signal; dead demand leaves its price falling to floor and
nobody refounds into it — the anti-churn discriminator). `FOUND_SIGNAL`
per-good constants alongside (provisional Food 4 / Entertainment 5 /
Luxury 8 ≈ 2× seed prices; soak-tuned then frozen). Prospectus price: the
survivor's live cheapest posted price (enter AT market; tâtonnement takes
it from there), or the good's seed price when no seller stands. Wage and
headcount come from the per-good founding template (constants alongside:
the seeded-solvent wages 35/36/24 — never the top posted wage, which would
seed a wage war through highest-wage matching — and headcounts 4/3/3;
single `Labourer` slot). `None` when no good qualifies.
Refs: @src/market.rs:178 (`plan_application`, the ranking mirror), §8.6,
the two-seller and demand-shock measurements.

### Intent::Found  (sim.rs, extended)

`Found { founder: AgentId, home: HouseId, good: Good, price: Money }`
Emitted only by phase 6's pure decide over the phase-start snapshot: at
most ONE per tick (the one-arrival precedent — legible wallet drains,
deterministic pass). Decide: `plan_founding` over the seller snapshots
names the good; the founder is the first **unemployed** agent in
`world.agents` order (ascending id) whose gold wallet covers
`capital + FOUNDER_RESERVE` where `capital = template wage × headcount ×
FOUND_CAPITAL_BILLS` (3, the `WAGE_BILLS_SEEDED` precedent — a founded firm
is funded exactly like a worldgen firm; `FOUNDER_RESERVE` provisional 200,
nobody founds themselves destitute). Unemployed-only is the eligibility
rule (subject to open question 1): founding converts a dis-saver into an
earner — the one channel in this milestone that actually relieves the fuse.
Premises: the lowest-id fully-vacant house (the `immigrate` predicate).
Apply, kill-only live re-checks mirroring stale Buys: founder still exists,
still unemployed, wallet still covers; house still fully vacant; the good's
live seller count still `< 2` — then `found_business`, then the stake
`pay(founder, new_id, Gold, capital)` (cannot fail after the re-check;
§8.5 backstop: an `Err` on either drops the intent with nothing half-done —
the command validates before touching state, and the stake precedes nothing
else), then the founder **self-hires** via
`assign_workplace(founder, house, Labourer)` — without it the new firm's
open, affordable slot ratchets its wage one step per tick (the measured
fact-4 overshoot) while standing as an aged-vacancy magnet; with it the
firm produces next tick. The remaining slots go to the labor market from
the next tick's snapshot. Every `match Intent` stays exhaustive.
Refs: @src/sim.rs:19, the goods/labor decide→apply templates,
@src/sim.rs:305 (the one-arrival precedent), §8.2, §8.5.

### Events  (sim.rs, extended)

`ProfitDrawn { business, owner, amount }` — zero draws emit nothing (the
held-price precedent). `Closed { business, house, owner, proceeds: Vec<(Metal, Money)> }`
— carries the house because the business id resolves to nothing afterward
(the `Departed` precedent); proceeds list every `Metal::ALL` entry in that
order, zeros included (D3 visible-zeros). `LaidOff { agent, business }` —
distinct from `Quit` (pushed, not walked; no volition, no arrears
implication); the money story is told by `Settled`, which is REUSED for
closure's creditor settlements with its doc widened from "phase 7" to "a
business settles arrears outside payroll" — covering former-worker
creditors, who are settled but not laid off. `Founded { business, founder,
house, good, price, capital }`. All amounts measured as balance deltas
around the command (the `apply_sinks_intent` precedent — events report what
moved, never a re-derivation). A phase-7 forced liquidation narrates in
causal order before the `Departed` line. Exact intra-closure event order is
plan-owned; determinism (phase order, then pinned iteration order) is not.
Refs: @src/sim.rs:59 (`Event`), :146 (`Departed`, the Vec-with-zeros and
name-carrying precedent), :880 (balance-delta measurement).

### Worldgen seeds owners  (engine/worldgen.rs, amended)

Each of the six shipped venues' `owner` is its FIRST seeded worker — alice
(Greenrow Farm), ed (Longacre Farm), ivan (Gilt Curtain Theater), karl
(The Brass Bell), marco (Karat & Co), otto (Silverthread Atelier) — the
owner-operator pattern, deterministic by spawn order. WHY these six: they
are employed and solvent, so the destitution push never reaches them in the
shipped soaks (no spurious forced liquidation), and the 9 permanently
unemployed stay non-owners — the emigration pool and its dis-saving fuse
survive as the migration story. The forced reorder (a venue's staff must
spawn before `create_business` so the owner id exists) shifts every
interleaved `AgentId` downstream: ONE deliberate item reorders the loop,
migrates every id-pinned test and event expectation, and re-pins — the
pack-2 conservation-re-pin precedent, never scattered. Per-metal totals are
order-independent: gold 52148 / silver 300 / copper 600, asserted
unchanged.
Refs: @src/engine/worldgen.rs:137 (`town_world`), :178 (the businesses
table and fill order).

### Mechanical / obvious (plan owns these — no contract needed)

Event-emission plumbing and the `invest` signature gaining the report param
(Amendment 15 already provides for stub phases waking); all shell
presentation (owner on roster/inspect, business count in the header,
distress on business inspect, feed lines and their aggregation); the exact
`WorldError` variant for a business-less house; the phase-6 pass's internal
decomposition (decide list shapes, the write-back loop); the seller
snapshot builder in sim.rs (world plumbing feeding `plan_founding`);
`businesses_mut` or per-house mutation, whichever the write-back needs; all
tuning constants' exact values (`DRAW_BUFFER_BILLS`,
`CLOSE_INSOLVENT_TICKS`, `FOUND_SIGNAL` table, `FOUND_CAPITAL_BILLS`,
`FOUNDER_RESERVE`, founding template wages/headcounts) — provisional here,
soak-tuned then frozen per the `adjust_price` pattern; the stress fixtures
(a doomed over-waged venue; a destitute owner) and soak harnesses;
the id-migration mechanics of the worldgen reorder.

## Migration impact (breaks existing shipped code)

- `src/world.rs` — `create_business` signature widens (+`owner`,
  validated first): ~17 call sites across worldgen and tests, all
  compile-time forced. `remove_agent` gains the forced-liquidation step
  (behavior change: an owner's departure now moves business money — pinned
  by the fixture tests).
- `src/business.rs` — `Business` gains `owner` and `insolvent_ticks`:
  every literal constructor in tests gains two fields.
- `src/engine/worldgen.rs` — the reorder item: staff spawned before
  venues, owners seeded; every downstream `AgentId` shifts; id-pinned
  tests and event expectations migrate in the same item; per-metal pins
  asserted unchanged.
- `src/sim.rs` — `invest` gains behavior and the report param; phase 1's
  Arrive decide gains the arrears conjunct (the 200-tick migration soak's
  arrival timing may shift — re-pinned in pack 2 if so); `Intent::Found`
  and four `Event` variants force every exhaustive match (trace, shell).
- `src/engine/game_loop.rs` — render arms for the new variants; owner
  display; business count.
- Soak criteria — pack 1's re-measure re-pins the fuse timeline and
  `NEAR_FULL`; every pack-2/3 criterion is written against the re-measured
  numbers, never the inherited 21/30 or t127.

## Acceptance tests (written from the contracts above)

- `draw_amount_clamps_and_respects_arrears` — the formula unit suite:
  surplus paid, at-buffer zero, arrears eat the surplus (net-of-arrears
  pinned), integer exactness.
- `draw_pass_pays_owner_and_pins_coffer_at_buffer` — phase-6 pass on a
  fixture: transfer lands, `ProfitDrawn` emitted, zero-draw silent, audit
  green.
- `create_business_validates_owner_first` — ghosts, reserved ids, business
  ids refuse with `UnknownAgent` before house checks; nothing changed on
  `Err`.
- `close_business_settles_ascending_writes_off_and_sweeps_no_orphans` —
  multi-creditor coffer-short fixture: ascending order pinned, remainders
  written off, zero-amount entries stripped, every `Metal::ALL` balance of
  the dead id zero PER-ACCOUNT, proceeds = owner wallet deltas, house
  passes the `immigrate` vacancy predicate afterward, audit green.
- `insolvent_ticks_single_writer_and_healthy_town_control` — the counter
  moves only in phase 6's write-back; all six seeded firms hold 0 through
  the 100-tick soak.
- `closure_fires_on_persistence_after_quits` — the doomed-venue fixture:
  workers quit first (the cheaper correction), the counter ages on frozen
  post-quit arrears, closure fires at the threshold — the reachability
  proof the level-gate design fails.
- `owner_emigration_forces_liquidation_no_orphans_on_either_id` — the
  dedicated fixture: leaver's firm settled/laid-off/liquidated, proceeds
  ride the sweep, External's delta exact, both dead ids zero per
  `Metal::ALL`, causal event order, audit green.
- `arrive_pull_skips_arrears_carrying_venues` — an aged slot at an owing
  venue pulls nobody; the same slot clean pulls (the handoff pin).
- `plan_founding_gates` — count gate (2 sellers → `None`), scarcity gate
  (survivor below signal → `None`, at/above → prospectus at the live
  price), zero sellers → unconditional at seed price, `Good::ALL` order
  pinned iteration-order-free.
- `found_apply_rechecks_and_self_hires` — stale intents die on each
  re-check (house taken, seller count recovered, wallet drained); a landed
  founding stakes exactly `capital`, the founder is employed at their own
  firm, the firm produces next tick; failed-stake impossibility pinned at
  the §8.5 boundary.
- `one_founding_per_tick_ascending_founder` — two qualifying founders, one
  fires, deterministic winner.
- Soaks: pack 1's four re-runs with re-pinned numbers; pack 2's
  stress-fixture death + tuned-town zero-closures; pack 3's both-directions
  shock soak — a fixtured single-seller closure with live demand founds
  within a pinned window, a fixtured demand-death founds NOTHING — plus
  zero foundings on the tuned equilibrium town, the anti-churn cycle cap
  (no good exceeds one found→close cycle per 100 ticks), the vacant-house
  competition watch (immigration still completes), and the full-cycle chain
  criterion: `Closed` → that house re-used (`Founded` or `Arrived` there) →
  `Hired`, population moving both directions, audit green every tick of
  every soak.

--- APPROVAL GATE — do not write the plan or any code above this line without sign-off ---

## Open questions

1. **Founder eligibility: unemployed-only, or any capitalized agent?**
   Unemployed-only (recommended, and written into the Intent::Found
   contract above): it is the one mechanism in this milestone that
   genuinely relieves the dis-saving fuse — a dis-saver becomes an earner —
   it keeps the decide deterministic and simple, and it tells the colony
   story (idle capital meets idle labor). The alternative — any agent whose
   wallet covers the stake — lets draw-rich owners found second ventures
   (the empire texture) but quietly selects the 3400g dis-savers anyway for
   the first ~100 ticks and entangles employed agents' wallets with the
   founding decide. Blocks pack 3 only.
2. **Amendment 18's ceremony.** The row-6 money-op column needs nothing —
   that reading is pinned in the Invariants — but the purpose text
   ("expand capacity / take profit") no longer describes the row honestly
   once three mechanics hang on it. Recommended: spend the number on the
   purpose-text edit (the table is the standing contract; silent divergence
   is the thing this repo refuses). Alternative: leave the table verbatim
   and carry the pinned reading in this spec alone — zero amendments beyond
   19. Blocks nothing; decides only what the 07-02 doc looks like.
3. **Pack cut: three packs (forced liquidation inside pack 2, with its
   fixture), or four (owner-departure isolated as its own closing pack)?**
   Three (recommended): the always-a-living-owner invariant is enforced the
   moment closure machinery exists — a four-pack cut leaves the draw paying
   a dangling id if an owner emigrates during the interim packs, an
   unspecified path under three soak-bearing packs. The four-pack
   alternative isolates the milestone's only amendment and its fixture at
   the cost of that window. Blocks the manifest's shape only.
