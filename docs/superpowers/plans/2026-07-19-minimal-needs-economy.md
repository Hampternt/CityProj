# Minimal Needs Economy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** First real behavior in the tick: agents with needs buy goods from
single-product businesses, wages are paid, and the mint tops up wage bills —
the spec's accepted "broken" (ever-growing) money supply.

**Architecture:** New `goods.rs` (closed `Good` enum + constants) and
`market.rs` (pure greedy shopping planner) feed the existing decide→apply
template in `sim.rs`. Phases 2/3/4/5/8 gain bodies; all money moves through
`World::pay` except phase 8 and worldgen seeding, which use
`Accounts::mint`. Spec: `docs/superpowers/specs/2026-07-19-minimal-needs-economy-design.md`.

**Tech Stack:** Rust edition 2024, std only (no new dependencies), `cargo test`.

## Global Constraints

- §8.1 integer money: all money is `Money(u64)`; never a float anywhere.
- §8.2 single chokepoint: phases move money only via `World::pay`
  (which forwards to `Accounts::transfer`); phase 8 and worldgen use
  `Accounts::mint` only. Nothing else mutates balances.
- §8.3 the audit stays unconditionally last in `tick`; never weaken it.
- §8.4 mint is the only creation; nothing in this plan burns.
- §8.5 no overdraft: unaffordable buys and unfunded wages skip cleanly
  (`Err` = nothing changed).
- §8.6 pricing/shopping *logic* lives in `market.rs`; price *data* is a
  `Business` field.
- Phase order is fixed; behavior lands inside existing phase functions,
  never around them.
- Constants (verbatim from spec): Food consumption 10, weight 100, target
  7 days, production 40, price 1. Entertainment: 5 / 30 / 7 / 20 / 2.
  Luxury: 2 / 10 / 7 / 8 / 5. Wage for every scenario role: 35/tick.
- Contract names are verbatim from the spec: `Good`, `Good::ALL`,
  `consumption_rate`, `weight`, `target_days`, `production_rate`,
  `plan_purchases`, `Offer`, `Purchase`, `Intent::Buy`,
  `apply_goods_intent`, `World::employee_of`, `mint_phase`. Do not rename.
- Commit style: lowercase `feat:` / `test:` / `docs:` subjects, matching
  `git log`.

## File Structure

- Create `src/goods.rs` — `Good` enum + per-good constant table (data only).
- Create `src/market.rs` — `Offer`, `Purchase`, `plan_purchases` (pure
  shopping algorithm, §8.6).
- Modify `src/money.rs` — make `Money::plus`/`Money::minus` pub, add
  `Money::times` (checked, value arithmetic only — `Accounts` untouched).
- Modify `src/agent.rs` — `Agent.inventory: HashMap<Good, u32>`.
- Modify `src/business.rs` — `product`/`price`/`stock` fields +
  `Business::wage_bill()`.
- Modify `src/world.rs` — `create_business` gains `product`/`price`
  params; new `World::employee_of`.
- Modify `src/sim.rs` — bodies for phases 2, 3, 4, 5, 8; `Intent::Buy`;
  integration test.
- Modify `src/engine/game_loop.rs` — worldgen scenario, per-tick summary,
  extended inspect.
- Modify `src/main.rs` — `mod goods; mod market;` + module-map doc bullets.
- Modify `CLAUDE.md` — "Current code state" section (final task).

Tests are co-located `#[cfg(test)] mod tests` per existing convention.

---

### Task 1: `Good` enum with the constants table

**Files:**
- Create: `src/goods.rs`
- Modify: `src/main.rs:37-44` (mod declarations)
- Test: `src/goods.rs` (co-located)

**Interfaces:**
- Consumes: nothing.
- Produces: `pub enum Good { Food, Entertainment, Luxury }` (`Copy + Eq +
  Hash`), `Good::ALL: [Good; 3]`, `fn consumption_rate(self) -> u32`,
  `fn weight(self) -> u32`, `fn target_days(self) -> u32`,
  `fn production_rate(self) -> u32`, `impl Display` (lowercase). Every
  later task uses these exact names.

- [ ] **Step 1: Write the failing tests**

Create `src/goods.rs` with the test module only (types come in step 3 —
the file won't compile until then, which is the expected failure):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_lists_every_variant_exactly_once() {
        let mut seen = std::collections::HashSet::new();
        for good in Good::ALL {
            assert!(seen.insert(good), "duplicate in Good::ALL: {good:?}");
        }
        assert_eq!(seen.len(), 3);
    }

    #[test]
    fn constants_match_the_spec_table() {
        // 07-19 spec, "Per-good constants" table — change spec first, then this
        let table = [
            (Good::Food, 10, 100, 7, 40),
            (Good::Entertainment, 5, 30, 7, 20),
            (Good::Luxury, 2, 10, 7, 8),
        ];
        for (good, consumption, weight, days, production) in table {
            assert_eq!(good.consumption_rate(), consumption);
            assert_eq!(good.weight(), weight);
            assert_eq!(good.target_days(), days);
            assert_eq!(good.production_rate(), production);
        }
    }

    #[test]
    fn display_is_lowercase_for_the_shell() {
        assert_eq!(Good::Food.to_string(), "food");
        assert_eq!(Good::Entertainment.to_string(), "entertainment");
        assert_eq!(Good::Luxury.to_string(), "luxury");
    }
}
```

Add to `src/main.rs` after `mod engine;`:

```rust
mod goods;
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test goods`
Expected: compile error — `Good` not found.

- [ ] **Step 3: Write the implementation**

Prepend to `src/goods.rs` (above the test module):

```rust
//! Goods: the closed set of consumable products (07-19 minimal-needs
//! spec). Same closed-enum pattern as `Role`: add a variant + extend
//! `ALL` and the compiler finds every match needing an update. The
//! per-good constants are the scenario's tuning table — data on the
//! enum, tunable later. Unit *prices* are NOT here: price is a
//! `Business` field set at worldgen.

use std::fmt;

/// One kind of consumable. `Copy + Eq + Hash` so it keys inventories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Good {
    Food,
    Entertainment,
    Luxury,
}

impl Good {
    /// Every variant, hand-enumerated — same zero-dep convention as
    /// `Role::ALL`. Extend this when adding a variant.
    pub const ALL: [Good; 3] = [Good::Food, Good::Entertainment, Good::Luxury];

    /// Units one agent consumes per tick (phase 5).
    pub fn consumption_rate(self) -> u32 {
        match self {
            Good::Food => 10,
            Good::Entertainment => 5,
            Good::Luxury => 2,
        }
    }

    /// Shopping-priority weight: the market scores goods by
    /// `weight / (held + planned + 1)` (diminishing returns, §8.6).
    pub fn weight(self) -> u32 {
        match self {
            Good::Food => 100,
            Good::Entertainment => 30,
            Good::Luxury => 10,
        }
    }

    /// Stockpile target in days: agents stop buying a good at
    /// `target_days × consumption_rate` units.
    pub fn target_days(self) -> u32 {
        match self {
            Good::Food => 7,
            Good::Entertainment => 7,
            Good::Luxury => 7,
        }
    }

    /// Units a staffed producer adds to its stock per tick (phase 2).
    pub fn production_rate(self) -> u32 {
        match self {
            Good::Food => 40,
            Good::Entertainment => 20,
            Good::Luxury => 8,
        }
    }
}

impl fmt::Display for Good {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Good::Food => "food",
            Good::Entertainment => "entertainment",
            Good::Luxury => "luxury",
        };
        write!(f, "{name}")
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test goods`
Expected: 3 passed. (`cargo check` may warn the module is unused outside
tests — fine until Task 3 wires it in; silence nothing.)

If unused-code warnings block a clean check, add at the top of the file,
mirroring `role.rs`'s convention:

```rust
// Data-only until the phases land (Tasks 6-10). Remove with Task 12's
// dead-code sweep.
#![allow(dead_code)]
```

- [ ] **Step 5: Commit**

```bash
git add src/goods.rs src/main.rs
git commit -m "feat: Good enum with per-good constants table (07-19 spec)"
```

---

### Task 2: `Money` value arithmetic for the market

**Files:**
- Modify: `src/money.rs:44-53` (visibility) and the `impl Money` block
- Test: `src/money.rs` (co-located)

**Interfaces:**
- Consumes: existing `Money`.
- Produces: `pub fn plus(self, other: Money) -> Money`,
  `pub fn minus(self, other: Money) -> Money`,
  `pub fn times(self, count: u32) -> Money` — all checked, panic on
  overflow/underflow. Used by Tasks 3 (wage_bill), 5 (budget tracking),
  8 (`units × price`). This is value arithmetic only — `Accounts` and the
  §8.2 chokepoint are untouched.

- [ ] **Step 1: Write the failing test**

Add to `src/money.rs` `mod tests`:

```rust
    #[test]
    fn times_scales_a_unit_price() {
        assert_eq!(Money::new(5).times(3), Money::new(15));
        assert_eq!(Money::new(5).times(0), Money::ZERO);
        assert_eq!(Money::ZERO.times(999), Money::ZERO);
    }

    #[test]
    #[should_panic(expected = "money overflow")]
    fn times_panics_on_overflow() {
        Money::new(u64::MAX).times(2);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test times`
Expected: compile error — no method `times`.

- [ ] **Step 3: Write the implementation**

In `src/money.rs`, change `fn plus` and `fn minus` to `pub fn` (docs
unchanged — "callers verify funds first" already covers the panic
contract), and add below `minus`:

```rust
    /// Checked multiplication by a unit count (`price × units`); panics
    /// on `u64` overflow rather than wrapping.
    pub fn times(self, count: u32) -> Money {
        Money(self.0.checked_mul(count as u64).expect("money overflow"))
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test money`
Expected: all money tests pass, including the two new ones.

- [ ] **Step 5: Commit**

```bash
git add src/money.rs
git commit -m "feat: public Money value arithmetic (plus/minus/times) for the market"
```

---

### Task 3: goods data on `Agent` and `Business`

**Files:**
- Modify: `src/agent.rs` (inventory field), `src/business.rs` (product /
  price / stock + `wage_bill`), `src/world.rs:53-71` (`spawn_agent`),
  `src/world.rs:243-260` (`create_business` signature) and its tests
- Test: `src/business.rs`, `src/world.rs` (co-located)

**Interfaces:**
- Consumes: `Good` (Task 1), `Money::plus`/`Money::times` (Task 2).
- Produces: `Agent.inventory: HashMap<Good, u32>` (missing entry = 0);
  `Business.product: Good`, `Business.price: Money`, `Business.stock:
  u32`, `pub fn wage_bill(&self) -> Money`;
  `World::create_business(&mut self, house: HouseId, product: Good,
  price: Money, roles: HashMap<Role, RoleSlot>) -> Result<AgentId,
  WorldError>` (stock starts 0). Tasks 5-11 rely on these exact names.

- [ ] **Step 1: Write the failing tests**

In `src/business.rs` `mod tests`, add:

```rust
    #[test]
    fn wage_bill_sums_wage_times_headcount_over_slots() {
        let mut roles = HashMap::new();
        roles.insert(Role::Engineer, RoleSlot { wage: Money::new(12), headcount: 2 });
        roles.insert(Role::Labourer, RoleSlot { wage: Money::new(7), headcount: 5 });
        let business = Business {
            id: AgentId(42),
            product: Good::Food,
            price: Money::new(1),
            stock: 0,
            roles,
        };
        // 12×2 + 7×5
        assert_eq!(business.wage_bill(), Money::new(59));
        let empty = Business {
            id: AgentId(43),
            product: Good::Luxury,
            price: Money::new(5),
            stock: 0,
            roles: HashMap::new(),
        };
        assert_eq!(empty.wage_bill(), Money::ZERO);
    }
```

In `src/world.rs` `mod tests`, add:

```rust
    #[test]
    fn create_business_sets_product_price_and_empty_stock() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        world
            .create_business(house, Good::Entertainment, Money::new(2), HashMap::new())
            .unwrap();
        let business = world.house(house).unwrap().business.as_ref().unwrap();
        assert_eq!(business.product, Good::Entertainment);
        assert_eq!(business.price, Money::new(2));
        assert_eq!(business.stock, 0);
    }

    #[test]
    fn spawned_agents_start_with_an_empty_inventory() {
        let mut world = World::new();
        let a = world.spawn_agent("a", None, None);
        assert!(world.agent(a).unwrap().inventory.is_empty());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test`
Expected: compile errors — `Business` has no `product` field,
`create_business` takes 2 arguments.

- [ ] **Step 3: Write the implementation**

`src/agent.rs` — add imports and field:

```rust
use std::collections::HashMap;

use crate::goods::Good;
use crate::housing::HouseId;
use crate::role::Role;
```

and on `Agent`, after `education`:

```rust
    /// Personal stock of goods: refilled by phase-4 purchases, drained by
    /// phase-5 consumption. Missing entry reads as 0. Goods are not money
    /// — no conservation audit applies.
    pub inventory: HashMap<Good, u32>,
```

`src/business.rs` — add `use crate::goods::Good;` and extend `Business`:

```rust
pub struct Business {
    /// Account key in [`Accounts`](crate::money::Accounts), allocated by
    /// `World::create_business` from the shared agent-id counter.
    pub id: AgentId,
    /// The one good this business produces and sells (v1: single-product,
    /// single-node). Production chains are a future spec.
    pub product: Good,
    /// Posted unit price, fixed at worldgen. Price *data* lives here;
    /// pricing *logic* stays in `market.rs` (§8.6). Never adjusts in this
    /// milestone.
    pub price: Money,
    /// Unsold units on hand: phase 2 adds, phase 4 sells.
    pub stock: u32,
    /// The roles this business employs — one wage/headcount per role
    /// (Amendment 11: role-differentiated, never a flat figure).
    pub roles: HashMap<Role, RoleSlot>,
}

impl Business {
    /// One tick of full staffing: sum over role slots of wage × headcount.
    /// Phase 8 mints exactly this per staffed business; worldgen seeds it
    /// once so tick 1's wages (paid before the first mint) never skip.
    pub fn wage_bill(&self) -> Money {
        self.roles
            .values()
            .fold(Money::ZERO, |sum, slot| sum.plus(slot.wage.times(slot.headcount)))
    }
}
```

`src/world.rs` — `spawn_agent` gains `inventory: HashMap::new(),` in the
`Agent { ... }` literal (import `Good` too: `use crate::goods::Good;`).
`create_business` becomes:

```rust
    /// Attaches a new business to `house`, allocating its account id from
    /// the same counter as `spawn_agent` — never a reserved id, never
    /// reused, and NO `Agent` struct is created (business ids are
    /// account-only, like Mint/External). Starts with zero stock; product
    /// and price are fixed at creation (07-19: prices never adjust).
    /// Validates before touching state: `Err` means nothing changed.
    pub fn create_business(
        &mut self,
        house: HouseId,
        product: Good,
        price: Money,
        roles: HashMap<Role, RoleSlot>,
    ) -> Result<AgentId, WorldError> {
        match self.house(house) {
            None => return Err(WorldError::UnknownHouse(house)),
            Some(existing) if existing.business.is_some() => {
                return Err(WorldError::BusinessAlreadyExists(house));
            }
            Some(_) => {}
        }
        let id = AgentId(self.next_agent_id);
        self.next_agent_id += 1;
        self.house_mut(house).expect("existence checked above").business =
            Some(Business { id, product, price, stock: 0, roles });
        Ok(id)
    }
```

Update every existing `create_business(house, HashMap::new())` call in
`src/world.rs` tests (in `create_business_allocates_id_from_agent_counter`,
`create_business_rejects_unknown_house`, `create_business_rejects_duplicate`,
`businesses_yields_only_hosting_houses_in_houses_order`,
`pay_accepts_business_ids`) to:

```rust
        world.create_business(house, Good::Food, Money::new(1), HashMap::new())
```

(add `use crate::goods::Good;` to the world tests' imports), and the
`Business { id: AgentId(42), roles }` literal in the existing
`business_roles_map_holds_distinct_wages_per_role` test gains
`product: Good::Food, price: Money::new(1), stock: 0,`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: all tests pass, including the three new ones.

- [ ] **Step 5: Commit**

```bash
git add src/agent.rs src/business.rs src/world.rs
git commit -m "feat: goods data on Agent and Business (inventory, product/price/stock, wage_bill)"
```

---

### Task 4: `World::employee_of`

**Files:**
- Modify: `src/world.rs` (command-layer `impl` block, next to `businesses`)
- Test: `src/world.rs` (co-located)

**Interfaces:**
- Consumes: `Agent.workplace`.
- Produces: `fn employee_of(&self, house: HouseId) -> Option<AgentId>` —
  first agent in `agents` order whose `workplace` is that house; `None`
  for unstaffed or unknown houses. Tasks 6, 7, 10 use it as the "staffed"
  test.

- [ ] **Step 1: Write the failing test**

Add to `src/world.rs` `mod tests`:

```rust
    #[test]
    fn employee_of_is_derived_first_match_in_agents_order() {
        let mut world = World::new();
        let shop = world.add_house("1 Mill Lane", vec![]);
        let idle_house = world.add_house("2 Kiln Row", vec![]);
        let first = world.spawn_agent("first", None, Some(shop));
        world.spawn_agent("second", None, Some(shop));
        // first match in `agents` order wins
        assert_eq!(world.employee_of(shop), Some(first));
        // unstaffed and unknown houses: None
        assert_eq!(world.employee_of(idle_house), None);
        assert_eq!(world.employee_of(HouseId(99)), None);
        // derived, never stored: quitting is visible immediately
        world.vacate_workplace(first).unwrap();
        let second = world.agent_by_name("second").unwrap().id;
        assert_eq!(world.employee_of(shop), Some(second));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test employee_of`
Expected: compile error — no method `employee_of`.

- [ ] **Step 3: Write the implementation**

In `src/world.rs`, inside the command-layer `impl World` block (after
`businesses`):

```rust
    /// The agent working at `house`: first match in `agents` order on the
    /// `workplace` field — derived per the link rule, never stored
    /// (mirrors [`occupants_of`](World::occupants_of)). `None` for
    /// unstaffed or unknown houses. v1 businesses are single-headcount,
    /// so "first" is "the" employee.
    pub fn employee_of(&self, house: HouseId) -> Option<AgentId> {
        self.agents
            .iter()
            .find(|agent| agent.workplace == Some(house))
            .map(|agent| agent.id)
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test employee_of`
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add src/world.rs
git commit -m "feat: World::employee_of — derived staffed-house query"
```

---

### Task 5: `market.rs` — the pure shopping planner

**Files:**
- Create: `src/market.rs`
- Modify: `src/main.rs` (mod declaration)
- Test: `src/market.rs` (co-located)

**Interfaces:**
- Consumes: `Good` + its constants (Task 1), `Money` comparison and
  `minus` (Task 2), `AgentId`.
- Produces: `pub struct Offer { pub business: AgentId, pub good: Good,
  pub price: Money, pub stock: u32 }`, `pub struct Purchase { pub
  business: AgentId, pub good: Good, pub units: u32 }`,
  `pub fn plan_purchases(wallet: Money, inventory: &HashMap<Good, u32>,
  offers: &[Offer]) -> Vec<Purchase>`. Task 8 maps `Purchase` →
  `Intent::Buy`.

- [ ] **Step 1: Write the failing tests**

Create `src/market.rs` with the test module (types in step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentId;
    use std::collections::HashMap;

    fn offer(business: u32, good: Good, price: u64, stock: u32) -> Offer {
        Offer { business: AgentId(business), good, price: Money::new(price), stock }
    }

    fn full_offers() -> Vec<Offer> {
        vec![
            offer(10, Good::Food, 1, 1000),
            offer(11, Good::Entertainment, 2, 1000),
            offer(12, Good::Luxury, 5, 1000),
        ]
    }

    #[test]
    fn no_offers_or_no_money_yields_the_empty_plan() {
        let inventory = HashMap::new();
        assert!(plan_purchases(Money::new(100), &inventory, &[]).is_empty());
        assert!(plan_purchases(Money::ZERO, &inventory, &full_offers()).is_empty());
    }

    #[test]
    fn highest_weight_wins_on_an_empty_stomach() {
        // one affordable unit: Food's weight 100 beats 30 and 10
        let plan = plan_purchases(Money::new(1), &HashMap::new(), &full_offers());
        assert_eq!(
            plan,
            vec![Purchase { business: AgentId(10), good: Good::Food, units: 1 }]
        );
    }

    #[test]
    fn diminishing_returns_divert_spending() {
        // held Food 10 → score 100/11 ≈ 9; empty Entertainment → 30/1
        let inventory = HashMap::from([(Good::Food, 10)]);
        let plan = plan_purchases(Money::new(2), &inventory, &full_offers());
        assert_eq!(
            plan,
            vec![Purchase { business: AgentId(11), good: Good::Entertainment, units: 1 }]
        );
    }

    #[test]
    fn purchases_coalesce_per_business_and_good() {
        // budget 3 buys food thrice from the same stall → one entry, units 3
        let plan = plan_purchases(Money::new(3), &HashMap::new(), &full_offers());
        assert_eq!(
            plan,
            vec![Purchase { business: AgentId(10), good: Good::Food, units: 3 }]
        );
    }

    #[test]
    fn offer_stock_is_respected() {
        // only 2 food on the shelf; the rest of the budget moves down-list
        let offers = vec![offer(10, Good::Food, 1, 2), offer(11, Good::Entertainment, 2, 1000)];
        let plan = plan_purchases(Money::new(4), &HashMap::new(), &offers);
        assert_eq!(
            plan,
            vec![
                Purchase { business: AgentId(10), good: Good::Food, units: 2 },
                Purchase { business: AgentId(11), good: Good::Entertainment, units: 1 },
            ]
        );
    }

    #[test]
    fn target_cap_stops_the_stockpile() {
        // Food cap = 7 days × 10/tick = 70; holding 69 leaves room for 1
        let inventory = HashMap::from([(Good::Food, 69)]);
        let offers = vec![offer(10, Good::Food, 1, 1000)];
        let plan = plan_purchases(Money::new(100), &inventory, &offers);
        assert_eq!(
            plan,
            vec![Purchase { business: AgentId(10), good: Good::Food, units: 1 }]
        );
        // at the cap exactly: nothing to buy
        let at_cap = HashMap::from([(Good::Food, 70)]);
        assert!(plan_purchases(Money::new(100), &at_cap, &offers).is_empty());
    }

    #[test]
    fn same_good_offers_go_cheapest_first_then_input_order() {
        // pricier stall listed first; cheaper one must still win
        let offers = vec![offer(20, Good::Food, 2, 1000), offer(21, Good::Food, 1, 1)];
        let plan = plan_purchases(Money::new(3), &HashMap::new(), &offers);
        assert_eq!(
            plan,
            vec![
                Purchase { business: AgentId(21), good: Good::Food, units: 1 },
                Purchase { business: AgentId(20), good: Good::Food, units: 1 },
            ]
        );
        // price tie: earlier input offer wins
        let tied = vec![offer(30, Good::Food, 1, 1000), offer(31, Good::Food, 1, 1000)];
        let plan = plan_purchases(Money::new(2), &HashMap::new(), &tied);
        assert_eq!(
            plan,
            vec![Purchase { business: AgentId(30), good: Good::Food, units: 2 }]
        );
    }

    #[test]
    fn unaffordable_goods_are_skipped_not_blocking() {
        // 1 coin: luxury (5) unaffordable, food affordable
        let inventory = HashMap::from([(Good::Food, 69), (Good::Entertainment, 35)]);
        // Entertainment at cap (35 = 7×5), food nearly capped: luxury would
        // score highest but costs too much — food's last unit still sells
        let plan = plan_purchases(Money::new(1), &inventory, &full_offers());
        assert_eq!(
            plan,
            vec![Purchase { business: AgentId(10), good: Good::Food, units: 1 }]
        );
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test market`
Expected: compile error — `Offer` / `plan_purchases` not found. (Also add
`mod market;` to `src/main.rs` now, between `mod housing;` and
`mod money;` — the mod list stays alphabetical.)

- [ ] **Step 3: Write the implementation**

Prepend to `src/market.rs`:

```rust
//! The goods market's shopping logic (§8.6: pricing/purchasing logic
//! lives HERE, never on agents or money). `plan_purchases` is pure —
//! wallet + inventory + posted offers in, purchase plan out; no world
//! access, fully deterministic. sim.rs builds `Offer`s from
//! `World::businesses()` and applies plans through `World::pay`.

use std::collections::HashMap;

use crate::agent::AgentId;
use crate::goods::Good;
use crate::money::Money;

/// One business's posted sale, snapshotted at phase start. Every agent
/// plans against the same snapshot; apply-time caps handle staleness.
#[derive(Debug, Clone)]
pub struct Offer {
    pub business: AgentId,
    pub good: Good,
    pub price: Money,
    pub stock: u32,
}

/// Planned units from one business — coalesced: at most one entry per
/// (business, good) pair, in first-planned order.
#[derive(Debug, PartialEq, Eq)]
pub struct Purchase {
    pub business: AgentId,
    pub good: Good,
    pub units: u32,
}

/// Greedy needs-shopping with diminishing returns: repeatedly buy 1 unit
/// of the highest-scoring good — score = weight / (held + planned + 1) —
/// that is (a) affordable in the remaining budget, (b) in remaining offer
/// stock, and (c) below its `target_days × consumption_rate` cap. Ties
/// between goods keep the earlier `Good::ALL` entry; same-good offers go
/// cheapest first, then input order. An empty plan is the valid "can't
/// afford anything" result. Terminates: every iteration moves some good
/// toward its finite cap.
pub fn plan_purchases(
    wallet: Money,
    inventory: &HashMap<Good, u32>,
    offers: &[Offer],
) -> Vec<Purchase> {
    let mut budget = wallet;
    let mut remaining: Vec<u32> = offers.iter().map(|offer| offer.stock).collect();
    let mut planned: HashMap<Good, u32> = HashMap::new();
    let mut purchases: Vec<Purchase> = Vec::new();

    while let Some(index) = best_buy(budget, inventory, offers, &remaining, &planned) {
        let offer = &offers[index];
        budget = budget.minus(offer.price); // affordability was checked
        remaining[index] -= 1;
        *planned.entry(offer.good).or_insert(0) += 1;
        let existing = purchases
            .iter_mut()
            .find(|p| p.business == offer.business && p.good == offer.good);
        match existing {
            Some(purchase) => purchase.units += 1,
            None => purchases.push(Purchase {
                business: offer.business,
                good: offer.good,
                units: 1,
            }),
        }
    }
    purchases
}

/// The single next unit to buy, as an index into `offers` — or `None`
/// when no good qualifies. Scores are compared by cross-multiplication
/// (`w_a/d_a > w_b/d_b  ⇔  w_a·d_b > w_b·d_a` in u64) so integer
/// division never truncates a ranking; the strict `>` keeps earlier
/// `Good::ALL` entries on ties.
fn best_buy(
    budget: Money,
    inventory: &HashMap<Good, u32>,
    offers: &[Offer],
    remaining: &[u32],
    planned: &HashMap<Good, u32>,
) -> Option<usize> {
    let mut best: Option<(usize, u32, u32)> = None; // (offer index, weight, denominator)
    for good in Good::ALL {
        let held = inventory.get(&good).copied().unwrap_or(0);
        let in_plan = planned.get(&good).copied().unwrap_or(0);
        if held + in_plan >= good.target_days() * good.consumption_rate() {
            continue; // cap reached — diminishing returns bottom out
        }
        let Some(index) = cheapest_offer(good, budget, offers, remaining) else {
            continue; // nothing affordable in stock
        };
        let denominator = held + in_plan + 1;
        let beats_best = match best {
            None => true,
            Some((_, best_weight, best_denominator)) => {
                u64::from(good.weight()) * u64::from(best_denominator)
                    > u64::from(best_weight) * u64::from(denominator)
            }
        };
        if beats_best {
            best = Some((index, good.weight(), denominator));
        }
    }
    best.map(|(index, _, _)| index)
}

/// Cheapest affordable offer of `good` with stock left; price ties keep
/// the earliest input offer.
fn cheapest_offer(
    good: Good,
    budget: Money,
    offers: &[Offer],
    remaining: &[u32],
) -> Option<usize> {
    let mut cheapest: Option<usize> = None;
    for (index, offer) in offers.iter().enumerate() {
        if offer.good != good || remaining[index] == 0 || offer.price > budget {
            continue;
        }
        if cheapest.is_none_or(|current| offer.price < offers[current].price) {
            cheapest = Some(index);
        }
    }
    cheapest
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test market`
Expected: 8 passed.

- [ ] **Step 5: Commit**

```bash
git add src/market.rs src/main.rs
git commit -m "feat: plan_purchases — pure greedy needs shopping (market.rs, §8.6)"
```

---

### Task 6: phase 2 — produce

**Files:**
- Modify: `src/sim.rs:41-44` (`produce`)
- Test: `src/sim.rs` (co-located; adds the shared `staffed_business`
  helper later phase tasks reuse)

**Interfaces:**
- Consumes: `World::businesses`, `World::employee_of` (Task 4),
  `Good::production_rate` (Task 1), `Business.stock` (Task 3).
- Produces: staffed businesses gain `production_rate(product)` stock per
  tick; unstaffed gain nothing. Also produces the test helper
  `staffed_business(world, address, product, price, wage, worker_name) ->
  (HouseId, AgentId, AgentId)` (house, business id, worker id) that
  Tasks 7, 8, 10 reuse.

- [ ] **Step 1: Write the failing test**

Add to `src/sim.rs` `mod tests` (extend the existing `use` list with
`crate::business::RoleSlot`, `crate::goods::Good`, `crate::housing::HouseId`,
`crate::role::Role`, `crate::world::World`, `std::collections::HashMap`
as needed):

```rust
    /// One single-role business at `wage`, staffed by a freshly spawned
    /// worker. Returns (house, business account, worker).
    fn staffed_business(
        world: &mut World,
        address: &str,
        product: Good,
        price: Money,
        wage: Money,
        worker_name: &str,
    ) -> (HouseId, AgentId, AgentId) {
        let house = world.add_house(address, vec![]);
        let mut roles = HashMap::new();
        roles.insert(Role::Labourer, RoleSlot { wage, headcount: 1 });
        let business = world
            .create_business(house, product, price, roles)
            .expect("fresh house");
        let worker = world.spawn_agent(worker_name, None, Some(house));
        world.agent_mut(worker).expect("just spawned").employed_role = Some(Role::Labourer);
        (house, business, worker)
    }

    fn stock_of(world: &World, house: HouseId) -> u32 {
        world.house(house).unwrap().business.as_ref().unwrap().stock
    }

    #[test]
    fn produce_fills_staffed_stock_only() {
        let mut world = World::new();
        let (farm, _, _) =
            staffed_business(&mut world, "Farm", Good::Food, Money::new(1), Money::new(35), "f");
        // unstaffed: business exists, nobody works there
        let idle_house = world.add_house("Idle", vec![]);
        world
            .create_business(idle_house, Good::Luxury, Money::new(5), HashMap::new())
            .unwrap();
        produce(&mut world);
        assert_eq!(stock_of(&world, farm), Good::Food.production_rate());
        assert_eq!(stock_of(&world, idle_house), 0);
        // stock accumulates tick over tick
        produce(&mut world);
        assert_eq!(stock_of(&world, farm), 2 * Good::Food.production_rate());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test produce_fills`
Expected: FAIL — `assert_eq!(stock_of(&world, farm), 40)` gets 0 (stub
body does nothing).

- [ ] **Step 3: Write the implementation**

Replace the `produce` stub in `src/sim.rs` (imports: add
`use crate::goods::Good;`, `use crate::housing::HouseId;` — extend as the
compiler directs):

```rust
/// Phase 2: labor + inputs → goods. Money ops allowed: none.
fn produce(world: &mut World) {
    // The staffed check borrows world immutably; collect first, then
    // mutate stock through house_mut.
    let staffed: Vec<HouseId> = world
        .businesses()
        .filter(|(house, _)| world.employee_of(house.id).is_some())
        .map(|(house, _)| house.id)
        .collect();
    for house_id in staffed {
        let house = world.house_mut(house_id).expect("collected from businesses()");
        let business = house.business.as_mut().expect("collected from businesses()");
        business.stock += business.product.production_rate();
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test produce_fills`
Expected: 1 passed. Also run `cargo test` — existing `n_ticks_run_clean`
must still pass (empty world: no businesses, phase no-ops).

- [ ] **Step 5: Commit**

```bash
git add src/sim.rs
git commit -m "feat: produce phase — staffed businesses build stock"
```

---

### Task 7: phase 3 — pay_wages

**Files:**
- Modify: `src/sim.rs:46-49` (`pay_wages`)
- Test: `src/sim.rs` (co-located)

**Interfaces:**
- Consumes: `World::businesses`, `World::employee_of`,
  `Agent.employed_role`, `RoleSlot.wage`, `World::pay` (transfer only —
  the phase's permitted money op).
- Produces: each staffed business transfers its worker's role wage;
  unfunded or role-less cases skip cleanly with no state change.

- [ ] **Step 1: Write the failing tests**

Add to `src/sim.rs` `mod tests`:

```rust
    #[test]
    fn pay_wages_transfers_the_role_wage() {
        let mut world = World::new();
        let (_, farm, worker) =
            staffed_business(&mut world, "Farm", Good::Food, Money::new(1), Money::new(35), "f");
        world.accounts.mint(farm, Money::new(50)); // funded
        pay_wages(&mut world);
        assert_eq!(world.accounts.balance_of(worker), Money::new(35));
        assert_eq!(world.accounts.balance_of(farm), Money::new(15));
        world.accounts.audit();
    }

    #[test]
    fn unfunded_wage_skips_cleanly() {
        let mut world = World::new();
        let (_, farm, worker) =
            staffed_business(&mut world, "Farm", Good::Food, Money::new(1), Money::new(35), "f");
        world.accounts.mint(farm, Money::new(10)); // less than the wage
        pay_wages(&mut world); // must not panic, must not partially pay (§8.5)
        assert_eq!(world.accounts.balance_of(worker), Money::ZERO);
        assert_eq!(world.accounts.balance_of(farm), Money::new(10));
    }

    #[test]
    fn unstaffed_business_pays_nobody() {
        let mut world = World::new();
        let house = world.add_house("Idle", vec![]);
        let mut roles = HashMap::new();
        roles.insert(Role::Labourer, RoleSlot { wage: Money::new(35), headcount: 1 });
        let business = world
            .create_business(house, Good::Food, Money::new(1), roles)
            .unwrap();
        world.accounts.mint(business, Money::new(50));
        pay_wages(&mut world);
        assert_eq!(world.accounts.balance_of(business), Money::new(50));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test pay_wages_transfers`
Expected: FAIL — worker balance stays 0 (stub does nothing).

- [ ] **Step 3: Write the implementation**

Replace the `pay_wages` stub (imports: `use crate::money::Money;` already
needed by tests; add `use crate::agent::AgentId;` alongside `Agent`):

```rust
/// Phase 3: firms pay agreed wages. Money ops allowed: transfer only.
fn pay_wages(world: &mut World) {
    // Decide from the snapshot: who is owed which role's wage. A worker
    // with no employed_role, or a role the business doesn't slot, earns
    // nothing this milestone.
    let owed: Vec<(AgentId, AgentId, Money)> = world
        .businesses()
        .filter_map(|(house, business)| {
            let worker = world.employee_of(house.id)?;
            let role = world.agent(worker)?.employed_role?;
            let slot = business.roles.get(&role)?;
            Some((business.id, worker, slot.wage))
        })
        .collect();
    // Apply through the validated chokepoint. An unfunded wage errs and
    // skips cleanly (§8.5) — never partial, never panicking.
    for (business, worker, wage) in owed {
        let _ = world.pay(business, worker, wage);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test wage`
Expected: the three new tests pass; then full `cargo test` stays green.

- [ ] **Step 5: Commit**

```bash
git add src/sim.rs
git commit -m "feat: pay_wages phase — role wage via World::pay, unfunded skips"
```

---

### Task 8: phase 4 — goods market (`Intent::Buy`)

**Files:**
- Modify: `src/sim.rs:8-12` (`Intent`), `src/sim.rs:54-76`
  (`goods_market`, `decide_goods`, `apply_goods_intent`)
- Test: `src/sim.rs` (co-located)

**Interfaces:**
- Consumes: `plan_purchases` / `Offer` / `Purchase` (Task 5),
  `Money::times` (Task 2), `Agent.inventory`, `Business.stock`/`price`,
  `World::pay`.
- Produces: `Intent::Buy { buyer: AgentId, business: AgentId, good: Good,
  units: u32 }`; `decide_goods(agent: &Agent, wallet: Money, offers:
  &[Offer]) -> Vec<Intent>` (pure); `apply_goods_intent` caps to live
  stock, pays, then moves goods — together or not at all.

- [ ] **Step 1: Write the failing tests**

Add to `src/sim.rs` `mod tests`:

```rust
    fn set_stock(world: &mut World, house: HouseId, stock: u32) {
        world
            .house_mut(house)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .stock = stock;
    }

    fn held(world: &World, agent: AgentId, good: Good) -> u32 {
        world
            .agent(agent)
            .unwrap()
            .inventory
            .get(&good)
            .copied()
            .unwrap_or(0)
    }

    #[test]
    fn buy_moves_money_and_goods_together() {
        let mut world = World::new();
        let (farm_house, farm, worker) =
            staffed_business(&mut world, "Farm", Good::Food, Money::new(2), Money::new(35), "f");
        set_stock(&mut world, farm_house, 50);
        world.accounts.mint(worker, Money::new(10));
        goods_market(&mut world);
        // 10 coins at price 2 → 5 units, capped well below stock and target
        assert_eq!(held(&world, worker, Good::Food), 5);
        assert_eq!(stock_of(&world, farm_house), 45);
        assert_eq!(world.accounts.balance_of(worker), Money::ZERO);
        assert_eq!(world.accounts.balance_of(farm), Money::new(10));
        world.accounts.audit();
    }

    #[test]
    fn stale_intents_cap_to_live_stock() {
        let mut world = World::new();
        let (farm_house, _, first) =
            staffed_business(&mut world, "Farm", Good::Food, Money::new(1), Money::new(35), "a");
        let second = world.spawn_agent("b", None, None);
        set_stock(&mut world, farm_house, 10);
        // both plan against the same 10-unit snapshot and could each afford it
        world.accounts.mint(first, Money::new(10));
        world.accounts.mint(second, Money::new(10));
        goods_market(&mut world);
        // agents-order: first drains the shelf, second is capped to zero
        assert_eq!(held(&world, first, Good::Food), 10);
        assert_eq!(held(&world, second, Good::Food), 0);
        assert_eq!(world.accounts.balance_of(second), Money::new(10)); // unspent
        assert_eq!(stock_of(&world, farm_house), 0);
        world.accounts.audit();
    }

    #[test]
    fn broke_buyers_change_nothing() {
        let mut world = World::new();
        let (farm_house, farm, worker) =
            staffed_business(&mut world, "Farm", Good::Food, Money::new(1), Money::new(35), "f");
        set_stock(&mut world, farm_house, 50);
        // no money minted to the worker at all
        goods_market(&mut world);
        assert_eq!(held(&world, worker, Good::Food), 0);
        assert_eq!(stock_of(&world, farm_house), 50);
        assert_eq!(world.accounts.balance_of(farm), Money::ZERO);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test buy_moves`
Expected: FAIL — nothing bought (decide pass still returns nothing).

- [ ] **Step 3: Write the implementation**

In `src/sim.rs`, replace the `Intent` enum, `goods_market`, `decide_goods`
and `apply_goods_intent` (imports: add
`use crate::market::{self, Offer};`):

```rust
/// What an agent wants to do, decided in a pure pass and executed in an
/// apply pass (see `goods_market` for the worked template). Mechanics add
/// variants; every `match intent` stays exhaustive so a new variant is a
/// compile-time forcing function on every apply fn.
pub enum Intent {
    /// Buy `units` of `good` from `business`'s stock (phase 4). Planned
    /// against the tick-start snapshot, so `units` may exceed stock by
    /// apply time — apply caps to what is really on the shelf.
    Buy {
        buyer: AgentId,
        business: AgentId,
        good: Good,
        units: u32,
    },
}
```

```rust
/// Phase 4: agents buy goods, prices adjust. Money ops allowed: transfer
/// only. This phase is the WORKED decide→apply TEMPLATE — every behavior
/// phase copies this two-pass shape.
fn goods_market(world: &mut World) {
    // Decide (pure): every agent plans against the same tick-start offer
    // snapshot. No `&mut` anywhere — unit-testable and free of
    // iteration-order effects. Collective staleness (two buyers wanting
    // the same last unit) is resolved at apply time.
    let offers: Vec<Offer> = world
        .businesses()
        .map(|(_, business)| Offer {
            business: business.id,
            good: business.product,
            price: business.price,
            stock: business.stock,
        })
        .collect();
    let intents: Vec<Intent> = world
        .agents
        .iter()
        .flat_map(|agent| decide_goods(agent, world.accounts.balance_of(agent.id), &offers))
        .collect();

    // Apply: the ONLY place this phase moves money. Unaffordable intents
    // fail cleanly (transfer errs) — wanting is unconstrained, paying is not.
    for intent in intents {
        apply_goods_intent(world, intent);
    }
}

/// Needs-driven purchasing. Stays pure; the shopping algorithm itself
/// lives in market.rs (§8.6) — this just binds it to one agent.
fn decide_goods(agent: &Agent, wallet: Money, offers: &[Offer]) -> Vec<Intent> {
    market::plan_purchases(wallet, &agent.inventory, offers)
        .into_iter()
        .map(|purchase| Intent::Buy {
            buyer: agent.id,
            business: purchase.business,
            good: purchase.good,
            units: purchase.units,
        })
        .collect()
}

fn apply_goods_intent(world: &mut World, intent: Intent) {
    match intent {
        Intent::Buy { buyer, business, good, units } => {
            // Re-read live stock: an earlier buyer this phase may have
            // emptied the shelf. Cap, pay, then hand over the goods —
            // money and goods move together or not at all.
            let found = world
                .businesses()
                .find(|(_, b)| b.id == business)
                .map(|(house, b)| (house.id, b.price));
            let Some((house_id, price)) = found else {
                return; // business vanished — intents don't outlive facts
            };
            let live_stock = world
                .house(house_id)
                .expect("found above")
                .business
                .as_ref()
                .expect("found above")
                .stock;
            let units = units.min(live_stock);
            if units == 0 {
                return;
            }
            if world.pay(buyer, business, price.times(units)).is_err() {
                return; // §8.5: skip cleanly, stock untouched
            }
            let house = world.house_mut(house_id).expect("found above");
            house.business.as_mut().expect("found above").stock -= units;
            let agent = world
                .agent_mut(buyer)
                .expect("intents are decided from world.agents");
            *agent.inventory.entry(good).or_insert(0) += units;
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: all pass — the three new tests and every earlier one
(`tick_runs_audit_last` still panics as demanded).

- [ ] **Step 5: Commit**

```bash
git add src/sim.rs
git commit -m "feat: goods market — Intent::Buy through the decide/apply template"
```

---

### Task 9: phase 5 — consume

**Files:**
- Modify: `src/sim.rs:78-81` (`consume`)
- Test: `src/sim.rs` (co-located)

**Interfaces:**
- Consumes: `Agent.inventory`, `Good::consumption_rate`.
- Produces: every agent's inventory drops by `consumption_rate(good)` per
  good, saturating at 0. No money ops.

- [ ] **Step 1: Write the failing test**

Add to `src/sim.rs` `mod tests`:

```rust
    #[test]
    fn consume_drains_inventories_saturating_at_zero() {
        let mut world = World::new();
        let a = world.spawn_agent("a", None, None);
        let agent = world.agent_mut(a).unwrap();
        agent.inventory.insert(Good::Food, 25);
        agent.inventory.insert(Good::Entertainment, 3); // below the rate of 5
        // Luxury absent: stays absent-or-zero, never underflows
        consume(&mut world);
        assert_eq!(held(&world, a, Good::Food), 15);
        assert_eq!(held(&world, a, Good::Entertainment), 0);
        assert_eq!(held(&world, a, Good::Luxury), 0);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test consume_drains`
Expected: FAIL — Food still 25 (stub does nothing).

- [ ] **Step 3: Write the implementation**

Replace the `consume` stub:

```rust
/// Phase 5: goods consumed toward needs. Money ops allowed: none.
/// Shortfall just bottoms out at zero this milestone — no starvation
/// consequences yet (07-19 spec: out of scope).
fn consume(world: &mut World) {
    for agent in &mut world.agents {
        for good in Good::ALL {
            let held = agent.inventory.entry(good).or_insert(0);
            *held = held.saturating_sub(good.consumption_rate());
        }
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test consume_drains`
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add src/sim.rs
git commit -m "feat: consume phase — inventories drain by consumption rate"
```

---

### Task 10: phase 8 — mint, plus the full-loop integration test

**Files:**
- Modify: `src/sim.rs:93-96` (`mint_phase`)
- Test: `src/sim.rs` (co-located)

**Interfaces:**
- Consumes: `World::businesses`, `World::employee_of`,
  `Business::wage_bill` (Task 3), `Accounts::mint` (the phase's only
  permitted money op).
- Produces: each staffed business is topped up by exactly one wage bill
  per tick — the spec's accepted ever-growing money supply. Full tick now
  runs the whole loop.

- [ ] **Step 1: Write the failing tests**

Add to `src/sim.rs` `mod tests`:

```rust
    #[test]
    fn mint_tops_up_staffed_businesses_by_one_wage_bill() {
        let mut world = World::new();
        let (_, farm, _) =
            staffed_business(&mut world, "Farm", Good::Food, Money::new(1), Money::new(35), "f");
        let idle_house = world.add_house("Idle", vec![]);
        let mut roles = HashMap::new();
        roles.insert(Role::Labourer, RoleSlot { wage: Money::new(35), headcount: 1 });
        let idle_business = world
            .create_business(idle_house, Good::Luxury, Money::new(5), roles)
            .unwrap();
        mint_phase(&mut world);
        assert_eq!(world.accounts.balance_of(farm), Money::new(35));
        // unstaffed: not a coin, even with slots posted
        assert_eq!(world.accounts.balance_of(idle_business), Money::ZERO);
        assert_eq!(world.accounts.total_minted(), Money::new(35));
        world.accounts.audit();
    }

    /// The first playable loop, end to end: one farm, one worker, one
    /// unemployed agent, seeded exactly like worldgen (wage bill on the
    /// business; wallet + one day's goods per agent). Every tick audits.
    #[test]
    fn minimal_economy_feeds_the_worker_and_breaks_the_idle() {
        let mut world = World::new();
        let (farm_house, farm, worker) =
            staffed_business(&mut world, "Farm", Good::Food, Money::new(1), Money::new(35), "f");
        let idle = world.spawn_agent("idle", None, None);
        world.accounts.mint(farm, Money::new(35)); // one wage bill (tick-1 seed)
        for id in [worker, idle] {
            world.accounts.mint(id, Money::new(35));
            let agent = world.agent_mut(id).unwrap();
            for good in Good::ALL {
                agent.inventory.insert(good, good.consumption_rate());
            }
        }
        for _ in 0..10 {
            tick(&mut world); // audit runs inside — any §8 break panics here
        }
        // money never appears outside the mint: seed 3×35, then one wage
        // bill (35) per tick — exact regardless of shopping dynamics
        assert_eq!(world.accounts.total_minted(), Money::new(105 + 10 * 35));
        // the worker keeps earning, eating, and holding stock
        assert!(world.accounts.balance_of(worker) > Money::ZERO);
        assert!(held(&world, worker, Good::Food) > 0);
        // the idle agent earned nothing: wallet drained, pantry empty
        // (07-19 spec: nobody saves the unemployed this milestone)
        assert_eq!(world.accounts.balance_of(idle), Money::ZERO);
        assert_eq!(held(&world, idle, Good::Food), 0);
        // overproduction piles up on the shelf (40/tick made, ~10 eaten)
        assert!(stock_of(&world, farm_house) > 0);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test mint_tops_up && cargo test minimal_economy`
Expected: `mint_tops_up…` FAILS (farm balance 0);
`minimal_economy…` FAILS (total_minted is only the 105 seed).

- [ ] **Step 3: Write the implementation**

Replace the `mint_phase` stub:

```rust
/// Phase 8: new money from reserve. Money ops allowed: mint only.
/// Tops up each staffed business by one wage bill, funding the NEXT
/// tick's phase 3 (worldgen seeds tick 1's). Accepted "broken" faucet
/// (07-19 spec): the supply grows every tick until a real gold-backed
/// mint job replaces this.
fn mint_phase(world: &mut World) {
    let bills: Vec<(AgentId, Money)> = world
        .businesses()
        .filter(|(house, _)| world.employee_of(house.id).is_some())
        .map(|(_, business)| (business.id, business.wage_bill()))
        .collect();
    for (business, bill) in bills {
        world.accounts.mint(business, bill);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: everything passes, including both new tests and the untouched
`n_ticks_run_clean` (empty world still mints nothing).

- [ ] **Step 5: Commit**

```bash
git add src/sim.rs
git commit -m "feat: mint phase tops up staffed wage bills — first full economic loop"
```

---

### Task 11: worldgen scenario and shell summary

**Files:**
- Modify: `src/engine/game_loop.rs:45-59` (`template_world`),
  `src/engine/game_loop.rs:69-105` (`render`),
  `src/engine/game_loop.rs:149-163` (`inspect`)

**Interfaces:**
- Consumes: everything above — `create_business` (Task 3), `wage_bill`
  (Task 3), `Good` constants (Task 1).
- Produces: the spec's playable scenario (3 staffed businesses + 1
  unemployed agent, all housed, seeded via `mint`) and a per-tick summary
  showing business stock/price/balance and agent inventories.

- [ ] **Step 1: Rewrite `template_world`**

Replace the function (extend the file's imports:
`use std::collections::HashMap;`, `use crate::business::RoleSlot;`,
`use crate::goods::Good;`, `use crate::money::Money;`,
`use crate::role::Role;`, and `Agent` in the existing `crate::agent`
import):

```rust
/// The 07-19 minimal-needs scenario: farm, theater, and jeweler (one
/// Labourer slot each at wage 35), three employed agents, one unemployed,
/// all housed at the residence. Worldgen seeds every business with one
/// wage bill — pay_wages runs before the first mint, so tick 1 must be
/// pre-funded — and every agent with a small wallet plus one day's goods.
/// All seeding goes through `mint`, so the audit counts it (§8.4).
fn template_world() -> World {
    let mut world = World::new();
    let residence = world.add_house("1 Mill Lane", vec![]);

    let farm = world.add_house("Greenrow Farm", vec![]);
    let theater = world.add_house("Gilt Curtain Theater", vec![]);
    let jeweler = world.add_house("Karat & Co", vec![]);
    let scenario = [
        (farm, Good::Food, Money::new(1), "alice"),
        (theater, Good::Entertainment, Money::new(2), "bob"),
        (jeweler, Good::Luxury, Money::new(5), "carol"),
    ];
    for (house, product, price, worker_name) in scenario {
        let mut roles = HashMap::new();
        roles.insert(Role::Labourer, RoleSlot { wage: Money::new(35), headcount: 1 });
        let business = world
            .create_business(house, product, price, roles)
            .expect("fresh house");
        let bill = world
            .house(house)
            .expect("just added")
            .business
            .as_ref()
            .expect("just created")
            .wage_bill();
        world.accounts.mint(business, bill);
        let worker = world.spawn_agent(worker_name, Some(residence), Some(house));
        world.agent_mut(worker).expect("just spawned").employed_role = Some(Role::Labourer);
    }
    world.spawn_agent("dave", Some(residence), None); // unemployed, housed

    let everyone: Vec<AgentId> = world.agents.iter().map(|agent| agent.id).collect();
    for id in everyone {
        world.accounts.mint(id, Money::new(35));
        let agent = world.agent_mut(id).expect("listed above");
        for good in Good::ALL {
            agent.inventory.insert(good, good.consumption_rate());
        }
    }
    world
}
```

- [ ] **Step 2: Extend `render` and `inspect`**

In `render`, inside the houses loop after the owners/occupants line, add:

```rust
        if let Some(business) = &house.business {
            println!(
                "    sells {} @{} · stock {} · balance {}",
                business.product,
                business.price,
                business.stock,
                world.accounts.balance_of(business.id),
            );
        }
```

Replace the agents loop body so each line carries the inventory:

```rust
    for agent in &world.agents {
        println!(
            "  {} — balance {} · home {} · {}",
            agent.name,
            world.accounts.balance_of(agent.id),
            describe_house(world, agent.home),
            describe_inventory(agent),
        );
    }
```

Add the helper next to `describe_house` (and `Agent` to the imports):

```rust
/// One line of pantry: `food 10 · entertainment 5 · luxury 2`.
fn describe_inventory(agent: &Agent) -> String {
    Good::ALL
        .iter()
        .map(|good| {
            let held = agent.inventory.get(good).copied().unwrap_or(0);
            format!("{good} {held}")
        })
        .collect::<Vec<_>>()
        .join(" · ")
}
```

In `inspect`, after the `workplace` line, add:

```rust
            println!("  goods     {}", describe_inventory(agent));
```

- [ ] **Step 3: Verify by running the sim**

Run: `cargo check && printf '\n\n\nq\n' | cargo run`
Expected: compiles clean; the final frame (tick 3) shows businesses with
non-zero stock and balances, alice/bob/carol holding goods with money
left, and dave's balance falling toward 0. No panic — the audit passed
every tick. Eyeball that wage 35 vs. consumption cost 30 shows workers
slowly accumulating.

- [ ] **Step 4: Run the full test suite**

Run: `cargo test`
Expected: all tests pass (worldgen is exercised only via `cargo run`, but
must not break compilation of the engine module).

- [ ] **Step 5: Commit**

```bash
git add src/engine/game_loop.rs
git commit -m "feat: minimal-needs worldgen scenario and per-tick shell summary"
```

---

### Task 12: dead-code sweep, docs, final verification

**Files:**
- Modify: `src/money.rs:17-19`, `src/business.rs:6-8`, `src/role.rs:7-9`
  (crate-level allows), `src/agent.rs` (field allows),
  `src/world.rs:134,158` (command-layer allows), `src/main.rs:9-25`
  (module map), `CLAUDE.md` ("Current code state" section)

**Interfaces:**
- Consumes: the finished feature.
- Produces: only still-true `#[allow(dead_code)]`s remain, each with an
  accurate comment; docs match the code.

- [ ] **Step 1: Tighten the allows**

The behavior now has real callers, so the blanket "nothing reads this
yet" allows are stale. Remove these and re-run `cargo check` after each:

- `src/business.rs`: delete the crate-level `#![allow(dead_code)]` block
  and its comment (every field is now read).
- `src/agent.rs`: delete `#[allow(dead_code)]` from `employed_role` (read
  by pay_wages). Keep the allows on `specialization` and `education` —
  still unread by design.
- `src/money.rs`: delete the crate-level `#![allow(dead_code)]` and its
  comment; add a targeted allow only where `cargo check` still warns
  (expected: `burn` — annotate it
  `#[allow(dead_code)] // the sinks phase (7) lands later` above the fn).
- `src/world.rs`: delete the two `#[allow(dead_code)]` lines on
  `WorldError` and the command-layer `impl` block; add targeted
  `#[allow(dead_code)] // no caller until the labor market lands` on
  whichever commands `cargo check` still flags (expected: `assign_home`,
  `vacate_home`, `assign_workplace`, `vacate_workplace`).
- `src/role.rs`: delete the crate-level allow; add a targeted allow on
  `Role::ALL` if flagged, same comment style.
- `src/goods.rs`: delete the Task-1 provisional allow if it was added.

Let the compiler be the referee: after edits, `cargo check` must be
warning-free with the narrowest possible allows.

- [ ] **Step 2: Update the docs**

`src/main.rs` module map — add bullets (alphabetical with the list):

```rust
//! - [`goods`] — [`goods::Good`], the closed set of consumables and their
//!   per-good constants.
//! - [`market`] — the goods market's pure shopping logic (§8.6);
//!   [`market::plan_purchases`] turns wallet + inventory + offers into a
//!   purchase plan.
```

`CLAUDE.md` — rewrite the "Current code state vs. target layout" bullets
to add:

```markdown
- `src/goods.rs` — `Good` (closed consumable enum) + the 07-19 per-good
  constants table (consumption, weight, target days, production).
- `src/market.rs` — `plan_purchases`: pure greedy needs-shopping (§8.6);
  sim.rs builds `Offer`s from `businesses()` and applies via `pay`.
```

and replace the "No economic behavior runs yet" paragraph with:

```markdown
The 07-19 minimal-needs loop runs: phases 2 (produce), 3 (wages),
4 (goods market via `Intent::Buy`), 5 (consume), and 8 (mint tops up
staffed wage bills — the accepted ever-growing supply) have behavior;
phases 1, 6, 7 remain TODO stubs. Worldgen seeds the farm/theater/jeweler
scenario. If you change structure, update this section.
```

Also update the "Next up" line: the multi-metal spec remains pending; the
minimal-needs spec is now implemented (drop it from "pending" phrasing if
listed, mirroring how earlier specs were stamped).

- [ ] **Step 3: Full verification**

Run: `cargo check && cargo clippy && cargo test`
Expected: no warnings, no clippy lints, all tests pass. Quote the test
summary line (e.g. `test result: ok. NN passed; 0 failed`) in the
completion report.

Run: `printf '\nq\n' | cargo run`
Expected: one tick renders the scenario without panicking.

- [ ] **Step 4: Commit**

```bash
git add src/ CLAUDE.md
git commit -m "docs: stamp minimal-needs economy implemented; tighten dead-code allows"
```
