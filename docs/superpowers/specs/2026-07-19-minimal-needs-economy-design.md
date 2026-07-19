# Minimal Needs Economy (first playable loop) — Design Spec

## Design (decisions — approve before reading on)

Gives the skeleton its first real behavior: agents have needs (Food,
Entertainment, Luxury) satisfied from personal inventories; businesses each
produce one good into a stock at a fixed price; agents spend their wage on
goods by priority with diminishing returns. Employment is pre-assigned at
worldgen — no job choice (that is the later labor-market spec). Prices never
adjust. The money supply grows forever: the mint phase tops up each staffed
business by its wage bill, so wages are effectively minted each tick — an
accepted "broken" economy for this milestone.

- **Fits into:** new `src/goods.rs` (Good enum + per-good constants), new
  `src/market.rs` (pure shopping algorithm), `src/business.rs` (product /
  price / stock fields), `src/agent.rs` (inventory), `src/world.rs`
  (`employee_of` derived query), `src/sim.rs` (phases 2, 3, 4, 5, 8 +
  `Intent::Buy`), `src/main.rs` / `src/engine/game_loop.rs` (worldgen
  scenario + per-tick summary).
- **Invariants in play:** §8.1 integer money; §8.2 single chokepoint (all
  movement via `World::pay`; mint only in phase 8); §8.3 audit last; §8.4
  mint-only creation; §8.5 no overdraft (unaffordable buys / unfunded wages
  skip cleanly); §8.6 pricing logic lives in `market.rs` (price *data* sits
  on `Business`).
- **Out of scope (YAGNI):** job choice / labor market (phase 1 stays TODO),
  price or wage adjustment, multi-employee businesses, input goods /
  production chains, sinks & demurrage, gold backing on the mint, saving the
  unemployed from starving.

### Tick behavior (phase order unchanged, audit still last)

| Phase | Behavior added |
|---|---|
| 2 produce | staffed business: `stock += production_rate(good)`; unstaffed: nothing |
| 3 pay_wages | staffed business pays its role wage to its worker via `World::pay` (transfer only); unfunded wage skips cleanly |
| 4 goods_market | decide→apply template: pure shopping plan per agent → `Intent::Buy` applied via `World::pay` |
| 5 consume | every agent: `inventory[good] -= consumption_rate(good)` saturating at 0, per good |
| 8 mint | per staffed business: `mint(business.id, wage bill)` — funds next tick's wages |

Tick 1 ordering note: pay_wages runs before the first mint, so worldgen
seeds each business account with one wage bill (via `mint`, logged as
created money) so wages never skip on tick 1.

### Per-good constants (data on `Good`, tunable later)

| Good | consumption/tick | shopping weight | target (days) | production/tick | unit price | notes |
|---|---|---|---|---|---|---|
| Food | 10 | 100 | 7 | 40 | 1 | highest priority |
| Entertainment | 5 | 30 | 7 | 20 | 2 | |
| Luxury | 2 | 10 | 7 | 8 | 5 | |

Price is a `Business` field set at worldgen (constants above are the
scenario's chosen values, not properties of the good). Wage for every role
in the scenario: 35/tick — daily full consumption costs 30, so employed
agents slowly accumulate.

### Worldgen scenario (mechanical, plan owns exact wiring)

Three business houses — farm (Food), theater (Entertainment), jeweler
(Luxury) — each with one single-headcount role slot at wage 35. Three
agents employed one per business (`workplace` + `employed_role` set), one
unemployed agent. All four agents housed. Each agent starts with a small
minted wallet (~35) and a starting inventory of one day's consumption; each
business starts with one wage bill minted and zero stock.

## Contracts (the hard reference the plan is written from)

### Unit: Good  (goods.rs)

Signature: `enum Good { Food, Entertainment, Luxury }` + `Good::ALL:
[Good; 3]` + `fn consumption_rate(self) -> u32`, `fn weight(self) -> u32`,
`fn target_days(self) -> u32`, `fn production_rate(self) -> u32`
Given:  any variant
Then:   returns the constants table above; `ALL` lists every variant once
        (same closed-enum pattern as `Role` / `Metal`)
Error:  none — total functions on a closed enum
Refs:   @src/goods.rs, @src/role.rs (pattern)

### Unit: plan_purchases  (market.rs)

Signature: `fn plan_purchases(wallet: Money, inventory: &HashMap<Good, u32>,
offers: &[Offer]) -> Vec<Purchase>` where `Offer { business: AgentId, good:
Good, price: Money, stock: u32 }` and `Purchase { business: AgentId, good:
Good, units: u32 }`
Given:  an agent's wallet snapshot, current inventory, and every business's
        offer (built by the caller from `World::businesses()`)
Then:   greedy loop — score each good `weight / (held + planned + 1)`; buy
        1 unit of the highest-scoring good that is (a) affordable within
        remaining budget, (b) in remaining offer stock, (c) below its
        `target_days × consumption_rate` cap; repeat until no good
        qualifies. Same-good offers: cheapest first, then input order.
        Returns coalesced per-(business, good) purchases. Pure — no world
        access, deterministic.
Error:  none — an empty plan is the valid "can't afford anything" result
Refs:   @src/market.rs, §8.6 pricing stays in market.rs

### Unit: Intent::Buy apply  (sim.rs)

Signature: `Intent::Buy { buyer: AgentId, business: AgentId, good: Good,
units: u32 }` handled by `apply_goods_intent(world, intent)`
Given:  a planned purchase (possibly stale — stock may have been bought by
        an earlier buyer this phase)
Then:   caps units to the business's current stock, pays `units × price`
        via `World::pay(buyer, business, …)`, then moves exactly the paid
        units from business stock to buyer inventory. Zero capped units =
        no-op. Money and goods move together or not at all.
Error:  `pay` refusal (§8.5) = skip cleanly, stock untouched — wanting is
        unconstrained, paying is not
Refs:   @src/sim.rs (worked template), §8.2, §8.5

### Unit: World::employee_of  (world.rs)

Signature: `fn employee_of(&self, house: HouseId) -> Option<AgentId>`
Given:  any house id
Then:   the first agent (in `agents` order) whose `workplace` is that house
        — derived from agent fields per the link rule, never stored; `None`
        for unstaffed or unknown houses
Error:  none
Refs:   @src/world.rs (mirrors `occupants_of`)

### Unit: mint_phase  (sim.rs)

Signature: existing `fn mint_phase(world: &mut World)` stub gains behavior
Given:  end-of-tick world
Then:   for each business whose house has an employee: `mint(business.id,
        wage bill)` where wage bill = sum over its role slots of
        `wage × headcount`. Unstaffed businesses get nothing. Only `mint`
        is used — the phase's only permitted money op.
Error:  none
Refs:   @src/sim.rs, §8.4 mint is the only creation

### Mechanical / obvious (plan owns these — no contract needed)

`produce` / `pay_wages` / `consume` phase bodies (single loops per the tick
table); `Business` / `Agent` field additions; worldgen scenario wiring in
`main.rs`; per-tick summary lines and extended agent inspect in the shell;
building `Vec<Offer>` from `businesses()`.

--- APPROVAL GATE — do not write the plan or any code above this line without sign-off ---

## Open questions

None.
