# Variable Goods Pricing + Wage Arrears Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Businesses adjust posted prices each tick from sell-through, wages are paid from business coffers only (shortfalls carried as an arrears ledger), and the phase-8 wage-bill mint faucet is deleted so worldgen's seed is the entire money supply forever.

**Architecture:** A pure `adjust_price` function in `market.rs` (§8.6) is wired into phase 4's existing decide→apply template as a post-apply write-back; phase 3 gains a per-worker `owed_to` debt ledger on `Business` where each tick's wage accrues before payment drains the coffers; phase 8 becomes an empty stub. Every payment stays a normal validated `transfer` — arrears are bookkeeping, never negative balances.

**Tech Stack:** Rust (edition 2024), std only. Spec: `docs/superpowers/specs/2026-07-19-variable-pricing-wage-arrears-design.md`.

## Global Constraints

- §8.1 Integer money only — all ratio checks are integer cross-multiplication (`sold * 10 >= offered * 9`, `sold * 2 < offered`); no floats anywhere.
- §8.2 Money moves only via `transfer`/`mint`/`burn` in `money.rs`; `World::pay` is the validated route sim phases use.
- §8.3 The tick-final audit is never weakened; after this feature it pins `total_money()` at the worldgen seed.
- §8.4 Mint stays the only creation path — it is simply not called during ticks anymore.
- §8.5 No overdraft; a partial wage payment is a full valid transfer of a smaller amount. Never settle the ledger without its transfer.
- §8.6 Pricing logic lives in `market.rs`; `sim.rs` only calls it.
- Tuning constants (spec table, same status as the goods table): raise at sell-through ≥ 9/10, lower at < 1/2, step `max(1, price / 10)`, floor `Money(1)`.
- `cargo fmt` before every commit (fmt sweeps are part of the workflow).

## File Structure

- `src/money.rs` — gains `Money::divided_by` (checked integer division for the proportional step). No other changes.
- `src/market.rs` — gains the pricing tuning constants and pure `adjust_price`. `plan_purchases` untouched.
- `src/business.rs` — `Business` gains `owed_to: HashMap<AgentId, Money>` and `owed_total()`; stale doc comments updated.
- `src/world.rs` — one-line change: `create_business` initializes `owed_to`.
- `src/sim.rs` — phase 3 rewrite (accrue → pay → settle), phase 4 write-back (sold counting + `adjust_price`), phase 8 stubbed; tests updated.
- `src/engine/game_loop.rs` — render adds per-business `owed` figure. Worldgen untouched (its seed is now the whole supply by construction).
- `CLAUDE.md` — current-state section updated; new Roadmap section.

---

### Task 1: `Money::divided_by` + `adjust_price` (market.rs)

**Files:**
- Modify: `src/money.rs` (new method after `times`, ~line 55; test in existing `tests` mod)
- Modify: `src/market.rs` (constants + function after the imports; tests in existing `tests` mod)

**Interfaces:**
- Consumes: `Money::{new, plus, minus, ZERO}` (existing), `Money: Ord` (derived — `.max()` works).
- Produces: `pub fn divided_by(self, divisor: u64) -> Money` on `Money` (panics on zero divisor); `pub fn adjust_price(price: Money, offered: u32, sold: u32) -> Money` in `market.rs` — pure, total, exactly the spec contract. Task 4 calls `market::adjust_price`.

- [ ] **Step 1: Write the failing tests**

In `src/money.rs` tests mod:

```rust
#[test]
fn divided_by_floors() {
    assert_eq!(Money::new(25).divided_by(10), Money::new(2));
    assert_eq!(Money::new(9).divided_by(10), Money::ZERO);
    assert_eq!(Money::ZERO.divided_by(10), Money::ZERO);
}
```

In `src/market.rs` tests mod:

```rust
#[test]
fn adjust_price_empty_shelf_is_no_signal() {
    // offered 0 → unchanged, NOT treated as poor sales
    assert_eq!(adjust_price(Money::new(5), 0, 0), Money::new(5));
}

#[test]
fn adjust_price_raises_on_high_sell_through() {
    // 9/10 exactly hits the threshold; step = max(1, 5/10) = 1
    assert_eq!(adjust_price(Money::new(5), 10, 9), Money::new(6));
    // sold out
    assert_eq!(adjust_price(Money::new(5), 10, 10), Money::new(6));
    // proportional step: 100/10 = 10
    assert_eq!(adjust_price(Money::new(100), 10, 10), Money::new(110));
}

#[test]
fn adjust_price_lowers_on_poor_sales_saturating_at_floor() {
    // 4/10 < 1/2 → down one step (100/10 = 10)
    assert_eq!(adjust_price(Money::new(100), 10, 4), Money::new(90));
    // 2 − max(1, 2/10) lands exactly on the floor
    assert_eq!(adjust_price(Money::new(2), 10, 0), Money::new(1));
    // a floor-price seller with poor sales stays at the floor
    assert_eq!(adjust_price(Money::new(1), 10, 0), Money::new(1));
}

#[test]
fn adjust_price_middling_sales_hold_the_price() {
    // 5/10: exactly 1/2 is not < 1/2; 8/10 is below the raise threshold
    assert_eq!(adjust_price(Money::new(10), 10, 5), Money::new(10));
    assert_eq!(adjust_price(Money::new(10), 10, 8), Money::new(10));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test adjust_price; cargo test divided_by`
Expected: compile error — `adjust_price` / `divided_by` not found.

- [ ] **Step 3: Implement**

In `src/money.rs`, after `times` (~line 55):

```rust
    /// Checked integer division, flooring — the proportional-step
    /// helper for pricing (§8.1: stays integer). Panics on a zero
    /// divisor; callers pass literal constants.
    pub fn divided_by(self, divisor: u64) -> Money {
        Money(self.0.checked_div(divisor).expect("money division by zero"))
    }
```

In `src/market.rs`, after the `Purchase` struct (before `plan_purchases`):

```rust
/// Pricing tuning constants (07-19 variable-pricing spec) — same status
/// as the goods table: gameplay knobs, change freely.
/// Sell-through ≥ 9/10 of offered raises the price one step.
const RAISE_THRESHOLD: (u64, u64) = (9, 10); // (numerator, denominator)
/// Sell-through < 1/2 of offered lowers the price one step.
const LOWER_THRESHOLD: (u64, u64) = (1, 2);
/// One step is `max(1, price / STEP_DIVISOR)` — proportional, integer-safe.
const STEP_DIVISOR: u64 = 10;
/// Prices never fall below this, so they can always recover upward.
const PRICE_FLOOR: Money = Money::new(1);

/// Per-business Walrasian tâtonnement (§8.6): sold out → raise, didn't
/// sell → lower, one proportional step per tick, saturating at
/// `PRICE_FLOOR`. Pure and total. `offered == 0` is "no signal", not
/// poor sales — the price holds. Callers guarantee `sold <= offered`.
/// Ratio checks are integer cross-multiplication — no floats (§8.1).
pub fn adjust_price(price: Money, offered: u32, sold: u32) -> Money {
    if offered == 0 {
        return price;
    }
    let step = Money::new(1).max(price.divided_by(STEP_DIVISOR));
    let (sold, offered) = (u64::from(sold), u64::from(offered));
    if sold * RAISE_THRESHOLD.1 >= offered * RAISE_THRESHOLD.0 {
        price.plus(step)
    } else if sold * LOWER_THRESHOLD.1 < offered * LOWER_THRESHOLD.0 {
        if price > step.plus(PRICE_FLOOR) {
            price.minus(step)
        } else {
            PRICE_FLOOR
        }
    } else {
        price
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test adjust_price; cargo test divided_by`
Expected: 4 + 1 tests PASS.

- [ ] **Step 5: Format and commit**

```bash
cargo fmt
git add src/money.rs src/market.rs
git commit -m "feat: adjust_price tatonnement + Money::divided_by"
```

---

### Task 2: `owed_to` arrears ledger on `Business`

**Files:**
- Modify: `src/business.rs` (struct field, `owed_total()`, three test constructors)
- Modify: `src/world.rs:268-274` (`create_business` initializer)

**Interfaces:**
- Consumes: `Money::{ZERO, plus}`, `AgentId`.
- Produces: `pub owed_to: HashMap<AgentId, Money>` on `Business` (empty map = fully paid; entries are removed at zero, never left as `Money::ZERO`); `pub fn owed_total(&self) -> Money`. Task 3 mutates `owed_to`; Task 6 displays `owed_total()`.

- [ ] **Step 1: Write the failing test**

In `src/business.rs` tests mod:

```rust
#[test]
fn owed_total_sums_the_arrears_ledger() {
    let mut business = Business {
        id: AgentId(42),
        product: Good::Food,
        price: Money::new(1),
        stock: 0,
        roles: HashMap::new(),
        owed_to: HashMap::new(),
    };
    assert_eq!(business.owed_total(), Money::ZERO);
    business.owed_to.insert(AgentId(1), Money::new(30));
    business.owed_to.insert(AgentId(2), Money::new(12));
    assert_eq!(business.owed_total(), Money::new(42));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test owed_total`
Expected: compile error — no field `owed_to`.

- [ ] **Step 3: Implement**

In `src/business.rs`, add to the `Business` struct after `roles`:

```rust
    /// Wage debt per worker (arrears, 07-19 pricing spec): phase 3
    /// accrues each tick's wage here, pays what the coffers cover via a
    /// normal transfer, and keeps the shortfall. Entries are removed at
    /// zero — an empty map means fully paid. Bookkeeping only, never a
    /// negative balance (§8.2/§8.5).
    pub owed_to: HashMap<AgentId, Money>,
```

Add to `impl Business` after `wage_bill`:

```rust
    /// Total outstanding wage debt across all workers. Display and
    /// diagnostics; phase 3 works per-worker, not from this sum.
    pub fn owed_total(&self) -> Money {
        self.owed_to
            .values()
            .fold(Money::ZERO, |sum, &owed| sum.plus(owed))
    }
```

Fix every constructor to compile: in `src/world.rs` `create_business` (~line 268) add `owed_to: HashMap::new(),` to the `Business { ... }` literal; in `src/business.rs` tests, add `owed_to: HashMap::new(),` to the two `Business` literals in `business_roles_map_holds_distinct_wages_per_role` and the two in `wage_bill_sums_wage_times_headcount_over_slots` (`business` and `empty`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: all tests PASS (whole suite — the field touches every constructor).

- [ ] **Step 5: Format and commit**

```bash
cargo fmt
git add src/business.rs src/world.rs
git commit -m "feat: owed_to wage-arrears ledger on Business"
```

---

### Task 3: `pay_wages` with arrears (sim.rs phase 3)

**Files:**
- Modify: `src/sim.rs:76-95` (`pay_wages`) and its tests (rewrite `unfunded_wage_skips_cleanly`, extend `pay_wages_transfers_the_role_wage`, add two tests + one helper)

**Interfaces:**
- Consumes: `Business.owed_to` and eligibility rules exactly as today (`employee_of`, `employed_role`, `roles.get`); `World::pay` for the transfer; `Money::{min via Ord, plus, minus, ZERO}`.
- Produces: phase-3 behavior per the spec contract — per business in `World::businesses()` iteration order: (1) accrue `slot.wage` into `owed_to[worker]`, (2) pay `min(balance_of(business), owed)` via `World::pay` (skip the call at zero), (3) subtract what was paid, removing the entry at zero. A funded business behaves identically to today.

- [ ] **Step 1: Write the failing tests**

In `src/sim.rs` tests mod, add a helper next to `held`:

```rust
    fn owed(world: &World, house: HouseId, worker: AgentId) -> Money {
        world
            .house(house)
            .unwrap()
            .business
            .as_ref()
            .unwrap()
            .owed_to
            .get(&worker)
            .copied()
            .unwrap_or(Money::ZERO)
    }
```

Replace `unfunded_wage_skips_cleanly` (it encoded the old skip-entirely behavior) with:

```rust
    #[test]
    fn underfunded_wage_drains_coffers_and_records_the_rest_as_arrears() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        world.accounts.mint(farm, Money::new(10)); // less than the wage
        pay_wages(&mut world);
        // partial payment IS a full valid transfer of a smaller amount (§8.5)
        assert_eq!(world.accounts.balance_of(worker), Money::new(10));
        assert_eq!(world.accounts.balance_of(farm), Money::ZERO);
        assert_eq!(owed(&world, farm_house, worker), Money::new(25));
        world.accounts.audit();
    }
```

Add:

```rust
    #[test]
    fn arrears_accrue_and_repay_when_revenue_returns() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        // broke business: the full wage becomes debt, no transfer happens
        pay_wages(&mut world);
        assert_eq!(world.accounts.balance_of(worker), Money::ZERO);
        assert_eq!(owed(&world, farm_house, worker), Money::new(35));
        // revenue returns: this tick's wage joins the pot and all 70 clears
        world.accounts.mint(farm, Money::new(100));
        pay_wages(&mut world);
        assert_eq!(world.accounts.balance_of(worker), Money::new(70));
        assert_eq!(world.accounts.balance_of(farm), Money::new(30));
        // paid-off entries leave the map entirely
        assert!(
            world
                .house(farm_house)
                .unwrap()
                .business
                .as_ref()
                .unwrap()
                .owed_to
                .is_empty()
        );
        world.accounts.audit();
    }
```

In `pay_wages_transfers_the_role_wage`, capture the house id (`let (farm_house, farm, worker) = ...`) and add after the existing balance assertions:

```rust
        // fully funded: nothing carried
        assert_eq!(owed(&world, farm_house, worker), Money::ZERO);
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -- arrears underfunded`
Expected: `underfunded_wage_drains_coffers_and_records_the_rest_as_arrears` FAILS (worker balance is `0`, not `10`) and `arrears_accrue_and_repay_when_revenue_returns` FAILS (owed is `0`, not `35`).

- [ ] **Step 3: Implement**

Replace `pay_wages` in `src/sim.rs` entirely:

```rust
/// Phase 3: firms pay agreed wages from their own coffers. Money ops
/// allowed: transfer only. Each tick's wage first joins the business's
/// `owed_to` ledger, then the business pays whatever its balance covers
/// — coffers drain to exactly zero before any wage goes unpaid, and
/// past-due wages repay automatically when revenue returns (arrears and
/// the current wage share one pot).
fn pay_wages(world: &mut World) {
    // Decide from the snapshot: who accrues which role's wage. A worker
    // with no employed_role, or a role the business doesn't slot, earns
    // nothing this milestone.
    let accruals: Vec<(HouseId, AgentId, AgentId, Money)> = world
        .businesses()
        .filter_map(|(house, business)| {
            let worker = world.employee_of(house.id)?;
            let role = world.agent(worker)?.employed_role?;
            let slot = business.roles.get(&role)?;
            Some((house.id, business.id, worker, slot.wage))
        })
        .collect();
    for (house_id, business_id, worker, wage) in accruals {
        let business = world
            .house_mut(house_id)
            .expect("collected from businesses()")
            .business
            .as_mut()
            .expect("collected from businesses()");
        let owed = business
            .owed_to
            .get(&worker)
            .copied()
            .unwrap_or(Money::ZERO)
            .plus(wage);
        business.owed_to.insert(worker, owed);
        // Pay what the coffers cover. Amount ≤ balance by construction,
        // so the transfer cannot err — but if it ever does, skip cleanly
        // (§8.5): the ledger keeps the full debt, never settled without
        // its payment.
        let payable = world.accounts.balance_of(business_id).min(owed);
        if payable == Money::ZERO || world.pay(business_id, worker, payable).is_err() {
            continue;
        }
        let business = world
            .house_mut(house_id)
            .expect("collected from businesses()")
            .business
            .as_mut()
            .expect("collected from businesses()");
        if owed == payable {
            business.owed_to.remove(&worker);
        } else {
            business.owed_to.insert(worker, owed.minus(payable));
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: whole suite PASSES — including the untouched eligibility tests (`unstaffed_business_pays_nobody`, `roleless_worker_earns_nothing`, `unslotted_role_earns_nothing`: no accrual for ineligible workers) and `minimal_economy_feeds_the_worker_and_breaks_the_idle` (funded path unchanged; faucet still open until Task 5).

- [ ] **Step 5: Format and commit**

```bash
cargo fmt
git add src/sim.rs
git commit -m "feat: pay wages from coffers with arrears carry-over"
```

---

### Task 4: goods_market price write-back (sim.rs phase 4)

**Files:**
- Modify: `src/sim.rs:100-181` (`goods_market`, `apply_goods_intent`) + new tests and a `price_of` helper; add `use std::collections::HashMap;` to the top-level imports
- Modify: `src/business.rs:39-41` (stale `price` field doc comment)

**Interfaces:**
- Consumes: `market::adjust_price(price, offered, sold)` from Task 1; the existing `Offer` snapshot (`offer.stock` at snapshot time IS `offered`).
- Produces: after the last intent applies, each snapshotted business's `price` is replaced by `adjust_price(price, offered, sold)`; new prices take effect next tick — the decide pass never observes a mid-phase change. `apply_goods_intent` gains a `sold: &mut HashMap<AgentId, u32>` parameter (units actually transacted, per business).

- [ ] **Step 1: Write the failing tests**

In `src/sim.rs` tests mod, add a helper next to `stock_of`:

```rust
    fn price_of(world: &World, house: HouseId) -> Money {
        world.house(house).unwrap().business.as_ref().unwrap().price
    }
```

Add three tests:

```rust
    #[test]
    fn sell_out_raises_the_price_for_the_next_tick() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(2),
            Money::new(35),
            "f",
        );
        set_stock(&mut world, farm_house, 10);
        world.accounts.mint(worker, Money::new(20));
        goods_market(&mut world);
        // the whole shelf sold at the OLD price (10 × 2 = 20 coins)…
        assert_eq!(held(&world, worker, Good::Food), 10);
        assert_eq!(world.accounts.balance_of(farm), Money::new(20));
        // …and the new price only exists after the phase: 2 + max(1, 2/10)
        assert_eq!(price_of(&world, farm_house), Money::new(3));
        world.accounts.audit();
    }

    #[test]
    fn poor_sales_lower_the_price_saturating_at_the_floor() {
        let mut world = World::new();
        let (farm_house, _, _) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(5),
            Money::new(35),
            "f",
        );
        let (stall_house, _, _) = staffed_business(
            &mut world,
            "Stall",
            Good::Entertainment,
            Money::new(1),
            Money::new(35),
            "s",
        );
        set_stock(&mut world, farm_house, 50);
        set_stock(&mut world, stall_house, 50);
        // nobody has money → 0 of 50 sold everywhere
        goods_market(&mut world);
        assert_eq!(price_of(&world, farm_house), Money::new(4));
        // a floor-price seller stays at the floor
        assert_eq!(price_of(&world, stall_house), Money::new(1));
    }

    #[test]
    fn empty_shelf_gives_no_price_signal() {
        let mut world = World::new();
        let (farm_house, _, _) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(7),
            Money::new(35),
            "f",
        );
        // stock 0 → offered 0: the price holds, NOT treated as poor sales
        goods_market(&mut world);
        assert_eq!(price_of(&world, farm_house), Money::new(7));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -- price_for_the_next_tick saturating_at_the_floor no_price_signal`
Expected: `sell_out_raises_the_price_for_the_next_tick` FAILS (price still `2`) and `poor_sales_lower_the_price_saturating_at_the_floor` FAILS (price still `5`). `empty_shelf_gives_no_price_signal` passes already (prices never change yet) — that is fine; it pins the contract against the implementation about to land.

- [ ] **Step 3: Implement**

Add `use std::collections::HashMap;` to `src/sim.rs`'s top-level imports.

In `goods_market`, replace the snapshot collection so it carries house ids for the write-back, keep the decide pass as-is, thread a `sold` map through apply, and append the write-back loop:

```rust
/// Phase 4: agents buy goods, prices adjust. Money ops allowed: transfer
/// only. This phase is the WORKED decide→apply TEMPLATE — every behavior
/// phase copies this two-pass shape.
fn goods_market(world: &mut World) {
    // Decide (pure): every agent plans against the same tick-start offer
    // snapshot. No `&mut` anywhere — unit-testable and free of
    // iteration-order effects. Collective staleness (two buyers wanting
    // the same last unit) is resolved at apply time. `houses` runs
    // parallel to `offers` so the write-back below can reach each
    // offer's business without a second lookup.
    let (houses, offers): (Vec<HouseId>, Vec<Offer>) = world
        .businesses()
        .map(|(house, business)| {
            (
                house.id,
                Offer {
                    business: business.id,
                    good: business.product,
                    price: business.price,
                    stock: business.stock,
                },
            )
        })
        .unzip();
    let intents: Vec<Intent> = world
        .agents
        .iter()
        .flat_map(|agent| decide_goods(agent, world.accounts.balance_of(agent.id), &offers))
        .collect();

    // Apply: the ONLY place this phase moves money. Unaffordable intents
    // fail cleanly (transfer errs) — wanting is unconstrained, paying is
    // not. `sold` counts units actually transacted, per business.
    let mut sold: HashMap<AgentId, u32> = HashMap::new();
    for intent in intents {
        apply_goods_intent(world, intent, &mut sold);
    }

    // Price write-back (logic in market.rs, §8.6): each price adjusts
    // from this tick's sell-through against the snapshot it was offered
    // at. New prices take effect next tick — the decide pass above only
    // ever saw the snapshot.
    for (house_id, offer) in houses.into_iter().zip(offers) {
        let units = sold.get(&offer.business).copied().unwrap_or(0);
        world
            .house_mut(house_id)
            .expect("snapshotted from businesses()")
            .business
            .as_mut()
            .expect("snapshotted from businesses()")
            .price = market::adjust_price(offer.price, offer.stock, units);
    }
}
```

In `apply_goods_intent`, change the signature and record the sale as the final statement of the `Intent::Buy` arm:

```rust
fn apply_goods_intent(world: &mut World, intent: Intent, sold: &mut HashMap<AgentId, u32>) {
```

```rust
            *agent.inventory.entry(good).or_insert(0) += units;
            *sold.entry(business).or_insert(0) += units;
```

In `src/business.rs`, the `price` field doc is now stale — replace it:

```rust
    /// Posted unit price, seeded at worldgen and adjusted every tick by
    /// phase 4's `market::adjust_price` write-back. Price *data* lives
    /// here; pricing *logic* stays in `market.rs` (§8.6).
    pub price: Money,
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: whole suite PASSES. Existing phase-4 tests assert balances/stock only, so the new write-back does not disturb them.

- [ ] **Step 5: Format and commit**

```bash
cargo fmt
git add src/sim.rs src/business.rs
git commit -m "feat: per-business price adjustment from sell-through"
```

---

### Task 5: Close the faucet — `mint_phase` becomes a stub

**Files:**
- Modify: `src/sim.rs:205-219` (`mint_phase`) + tests (`mint_tops_up_staffed_businesses_by_one_wage_bill` replaced, `minimal_economy_feeds_the_worker_and_breaks_the_idle` assertions updated)
- Modify: `src/business.rs` (stale `RoleSlot` and `wage_bill` doc comments)

**Interfaces:**
- Consumes: nothing.
- Produces: `mint_phase` performs no money operation of any kind; `total_minted` changes only at worldgen and the §8.3 audit pins `total_money()` at the seed forever. `Business::wage_bill()` keeps its signature and existing callers (worldgen, tests) — phase 8 just no longer reads it.

- [ ] **Step 1: Update the tests to the new contract (failing first)**

In `src/sim.rs` tests, delete `mint_tops_up_staffed_businesses_by_one_wage_bill` and add:

```rust
    #[test]
    fn mint_phase_creates_no_money() {
        let mut world = World::new();
        let (_, farm, _) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        world.accounts.mint(farm, Money::new(35)); // worldgen-style seed
        mint_phase(&mut world);
        // the tick-time faucet is closed: nothing beyond the seed, ever
        assert_eq!(world.accounts.total_minted(), Money::new(35));
        assert_eq!(world.accounts.total_money(), Money::new(35));
        world.accounts.audit();
    }
```

In `minimal_economy_feeds_the_worker_and_breaks_the_idle`, replace the two-line comment and the `total_minted` assertion:

```rust
        // the worldgen seed (3 × 35) is the ENTIRE money supply, forever
        // — the audit pins it there every tick
        assert_eq!(world.accounts.total_minted(), Money::new(105));
        assert_eq!(world.accounts.total_money(), Money::new(105));
```

(Keep every other assertion: the worker still earns and eats — wages now recycle through farm revenue and arrears instead of fresh mint; the idle agent still drains to zero; the shelf still piles up.)

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -- mint_phase_creates_no_money minimal_economy`
Expected: both FAIL — `total_minted` is `70` (35 seed + 35 topped up) in the first and `455` in the second, because the faucet still runs.

- [ ] **Step 3: Implement**

Replace `mint_phase` in `src/sim.rs` entirely:

```rust
/// Phase 8: new money from reserve. Money ops allowed: mint only.
/// Inert since the 07-19 pricing spec closed the tick-time faucet:
/// worldgen's seed is the entire supply and the §8.3 audit pins
/// `total_money()` there forever. TODO: the literal staffed Mint
/// business (parent doc §2.1, gold goods → coins) lands here.
fn mint_phase(_world: &mut World) {}
```

In `src/business.rs`, update the two doc comments that name phase 8 as a reader:

`RoleSlot` (keep the multi-metal caveat sentence unchanged):

```rust
/// One role a business employs: the wage it offers and how many workers
/// it wants. Read by phase 3 (`pay_wages`, via `wage`). `wage` types
/// against today's single-metal `Money`; the multi-metal migration pass
/// revises it (single metal vs. bundle is that spec's open question) —
/// don't design around the current type being final.
```

`wage_bill`:

```rust
    /// One tick of full staffing: sum over role slots of wage × headcount.
    /// Worldgen seeds each business with exactly this so tick 1's wages
    /// (paid before any revenue) never skip; nothing reads it per-tick
    /// since the 07-19 spec closed the phase-8 faucet.
    pub fn wage_bill(&self) -> Money {
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: whole suite PASSES — `n_ticks_run_clean` (empty world, zero supply) is unaffected; `minimal_economy` now runs on a fixed 105-coin supply with wages recycling through revenue.

- [ ] **Step 5: Format and commit**

```bash
cargo fmt
git add src/sim.rs src/business.rs
git commit -m "feat: close the phase-8 wage-bill faucet — supply fixed at seed"
```

---

### Task 6: Shell arrears display + documentation

**Files:**
- Modify: `src/engine/game_loop.rs:137-146` (business line in `render`)
- Modify: `CLAUDE.md` (current-state section + new Roadmap section)

**Interfaces:**
- Consumes: `Business::owed_total()` from Task 2.
- Produces: display only — no behavior. This task ends the feature; run the full verification gate.

- [ ] **Step 1: Add arrears to the business render line**

In `render`, replace the business `println!`:

```rust
        if let Some(business) = &house.business {
            println!(
                "    sells {} @{} · stock {} · balance {} · owed {}",
                business.product,
                business.price,
                business.stock,
                world.accounts.balance_of(business.id),
                business.owed_total(),
            );
        }
```

(The total money supply and per-business price are already on screen; `owed` is the one missing figure the spec asks for.)

- [ ] **Step 2: Update CLAUDE.md**

Three edits:

**(a)** In "Current code state vs. target layout", replace the `role.rs`/`business.rs` bullet's phase description:

```markdown
- `src/role.rs`, `src/business.rs` — `Role` (closed job-role enum) and
  `Business`/`RoleSlot` (per-role wages, account-only money); phase 3
  (`pay_wages`) reads `employed_role` and `RoleSlot.wage` and keeps the
  per-worker `owed_to` wage-arrears ledger.
```

and extend the `market.rs` bullet:

```markdown
- `src/market.rs` — `plan_purchases`: pure greedy needs-shopping (§8.6);
  sim.rs builds `Offer`s from `businesses()` and applies via `pay`;
  `adjust_price`: per-business tâtonnement (sold out → raise, didn't
  sell → lower) with its tuning constants alongside.
```

**(b)** Replace the "The 07-19 minimal-needs loop runs" paragraph:

```markdown
The 07-19 loops run: phases 2 (produce), 3 (wages from business coffers,
shortfalls carried as `owed_to` arrears and repaid when revenue returns),
4 (goods market via `Intent::Buy`, then per-business `adjust_price`
write-back — new prices take effect next tick), and 5 (consume) have
behavior; phases 1, 6, 7, and 8 are TODO stubs. The tick-time mint
faucet is closed: worldgen's seed is the entire money supply and the
audit pins it there. Worldgen seeds the farm/theater/jeweler scenario.
If you change structure, update this section.
```

**(c)** Add a `## Roadmap` section immediately after the "Current code state vs. target layout" section (before "Hard invariants"):

```markdown
## Roadmap (recorded 2026-07-19; future specs design these)

- **Physical goods movement.** Goods will physically travel: producer →
  market / wherever needed, moved by hauler/logistics-type businesses.
  Agents already have inventories; houses and market venues get their own
  storage; agents travel to a market to buy and back home to stow. Until
  then businesses sell from their own `stock` — the seam where "shelf"
  becomes "market venue inventory" is the `Offer` struct, so don't bake
  in the assumption that stock lives only on businesses.
- **Money creation.** As designed in the parent doc §2.1: a *literal
  staffed Mint business* that consumes precious-metal goods to mint coins
  (seigniorage formula deferred by the 07-12 multi-metal spec), plus
  trade with outside markets through `External`.
- **Wage market.** The `adjust_price` pattern applied to `RoleSlot.wage`
  in phase 1 (can't fill a slot → raise, queue of applicants → lower),
  plus employee happiness / job-switching driven by the arrears ledger.
```

- [ ] **Step 3: Full verification gate**

Run: `cargo fmt && cargo check && cargo clippy && cargo test`
Expected: fmt makes no further changes; check/clippy clean (no new warnings); all tests PASS. Quote the test-count line.

Then smoke the shell: run `cargo run`, advance a few ticks with Enter, and confirm the frame shows `owed 0` on funded businesses, `money: total=` constant across ticks, and prices moving (food drifting toward the floor is the expected oversupply signal, not a bug). Quit with `q`.

- [ ] **Step 4: Commit**

```bash
git add src/engine/game_loop.rs CLAUDE.md
git commit -m "feat: show arrears in the shell; docs for closed faucet + roadmap"
```
