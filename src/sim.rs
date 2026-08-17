//! The fixed 9-phase tick (parent doc §6). A new mechanic lands INSIDE its
//! phase; adding or reordering phases requires amending the spec's phase
//! contract table. The conservation audit (§8.3) is unconditionally last.

use crate::agent::{Agent, AgentId};
use crate::goods::Good;
use crate::housing::HouseId;
use crate::market::{self, Offer};
use crate::metal::Metal;
use crate::money::Money;
use crate::world::World;
use std::collections::HashMap;

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

/// Runs one tick: phases 1–8 in exactly the spec table's order — labor
/// clears, produce, wages, goods clear, consume, invest, sinks, mint — then
/// the conservation audit, unconditionally last; no early return skips it.
///
/// # Panics
///
/// Panics if the closing [`audit`](crate::money::Accounts::audit) finds the
/// books imbalanced (§8.3) — meaning some phase moved money outside the
/// §8.2 chokepoint.
pub fn tick(world: &mut World) {
    labor_market(world);
    produce(world);
    pay_wages(world);
    goods_market(world);
    consume(world);
    invest(world);
    sinks(world);
    mint_phase(world);
    // Phase 9: audit (§8.3) — read-only, never gains behavior.
    world.accounts.audit();
}

/// Phase 1: match hires, adjust wage offers. Money ops allowed: none.
fn labor_market(_world: &mut World) {
    // TODO: firms + labor market land here.
}

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
        let house = world
            .house_mut(house_id)
            .expect("collected from businesses()");
        let business = house
            .business
            .as_mut()
            .expect("collected from businesses()");
        business.stock += business.product.production_rate();
    }
}

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
        let payable = world
            .accounts
            .balance_of(business_id, Metal::Gold)
            .min(owed);
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
        .flat_map(|agent| {
            decide_goods(
                agent,
                world.accounts.balance_of(agent.id, Metal::Gold),
                &offers,
            )
        })
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

fn apply_goods_intent(world: &mut World, intent: Intent, sold: &mut HashMap<AgentId, u32>) {
    match intent {
        Intent::Buy {
            buyer,
            business,
            good,
            units,
        } => {
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
            *sold.entry(business).or_insert(0) += units;
        }
    }
}

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

/// Phase 6: expand capacity / take profit. Money ops allowed: transfer only.
fn invest(_world: &mut World) {
    // TODO: firm investment lands here.
}

/// Phase 7: degradation, imports. Money ops allowed: burn, transfer→External.
fn sinks(_world: &mut World) {
    // TODO: demurrage and external purchases land here.
}

/// Phase 8: new money from reserve. Money ops allowed: mint only.
/// Inert since the 07-19 pricing spec closed the tick-time faucet:
/// worldgen's seed is the entire supply and the §8.3 audit pins
/// `total_money()` there forever. TODO: the literal staffed Mint
/// business (parent doc §2.1, gold goods → coins) lands here.
fn mint_phase(_world: &mut World) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentId;
    use crate::business::RoleSlot;
    use crate::goods::Good;
    use crate::housing::HouseId;
    use crate::money::Money;
    use crate::role::Role;
    use crate::world::World;
    use std::collections::HashMap;

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

    fn price_of(world: &World, house: HouseId) -> Money {
        world.house(house).unwrap().business.as_ref().unwrap().price
    }

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

    #[test]
    fn produce_fills_staffed_stock_only() {
        let mut world = World::new();
        let (farm, _, _) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
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

    #[test]
    fn n_ticks_run_clean() {
        let mut world = World::new();
        for _ in 0..100 {
            tick(&mut world);
        }
        // nothing mints yet, so the money supply must still be zero
        assert_eq!(world.accounts.total_money(Metal::Gold), Money::ZERO);
    }

    #[test]
    #[should_panic]
    fn tick_runs_audit_last() {
        let mut world = World::new();
        // corrupt the books via the sanctioned test hook; if any path
        // through tick skipped the audit, this would NOT panic
        world
            .accounts
            .set_balance_for_test(AgentId(7), Money::new(999));
        tick(&mut world);
    }

    #[test]
    fn pay_wages_transfers_the_role_wage() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        world.accounts.mint(farm, Money::new(50)); // funded
        pay_wages(&mut world);
        assert_eq!(
            world.accounts.balance_of(worker, Metal::Gold),
            Money::new(35)
        );
        assert_eq!(world.accounts.balance_of(farm, Metal::Gold), Money::new(15));
        // fully funded: nothing carried
        assert_eq!(owed(&world, farm_house, worker), Money::ZERO);
        world.accounts.audit();
    }

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
        assert_eq!(
            world.accounts.balance_of(worker, Metal::Gold),
            Money::new(10)
        );
        assert_eq!(world.accounts.balance_of(farm, Metal::Gold), Money::ZERO);
        assert_eq!(owed(&world, farm_house, worker), Money::new(25));
        world.accounts.audit();
    }

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
        assert_eq!(world.accounts.balance_of(worker, Metal::Gold), Money::ZERO);
        assert_eq!(owed(&world, farm_house, worker), Money::new(35));
        // revenue returns: this tick's wage joins the pot and all 70 clears
        world.accounts.mint(farm, Money::new(100));
        pay_wages(&mut world);
        assert_eq!(
            world.accounts.balance_of(worker, Metal::Gold),
            Money::new(70)
        );
        assert_eq!(world.accounts.balance_of(farm, Metal::Gold), Money::new(30));
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

    #[test]
    fn unstaffed_business_pays_nobody() {
        let mut world = World::new();
        let house = world.add_house("Idle", vec![]);
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(35),
                headcount: 1,
            },
        );
        let business = world
            .create_business(house, Good::Food, Money::new(1), roles)
            .unwrap();
        world.accounts.mint(business, Money::new(50));
        pay_wages(&mut world);
        assert_eq!(
            world.accounts.balance_of(business, Metal::Gold),
            Money::new(50)
        );
    }

    #[test]
    fn roleless_worker_earns_nothing() {
        let mut world = World::new();
        let house = world.add_house("Farm", vec![]);
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(35),
                headcount: 1,
            },
        );
        let business = world
            .create_business(house, Good::Food, Money::new(1), roles)
            .unwrap();
        // Spawn worker at the workplace but WITHOUT setting employed_role
        let worker = world.spawn_agent("f", None, Some(house));
        // employed_role stays None
        world.accounts.mint(business, Money::new(50));
        pay_wages(&mut world);
        assert_eq!(world.accounts.balance_of(worker, Metal::Gold), Money::ZERO);
        assert_eq!(
            world.accounts.balance_of(business, Metal::Gold),
            Money::new(50)
        );
    }

    #[test]
    fn unslotted_role_earns_nothing() {
        let mut world = World::new();
        let house = world.add_house("Farm", vec![]);
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(35),
                headcount: 1,
            },
        );
        let business = world
            .create_business(house, Good::Food, Money::new(1), roles)
            .unwrap();
        // Spawn worker and assign Engineer role, which is NOT in the business's roles
        let worker = world.spawn_agent("e", None, Some(house));
        world.agent_mut(worker).expect("just spawned").employed_role = Some(Role::Engineer);
        world.accounts.mint(business, Money::new(50));
        pay_wages(&mut world);
        assert_eq!(world.accounts.balance_of(worker, Metal::Gold), Money::ZERO);
        assert_eq!(
            world.accounts.balance_of(business, Metal::Gold),
            Money::new(50)
        );
    }

    #[test]
    fn buy_moves_money_and_goods_together() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(2),
            Money::new(35),
            "f",
        );
        set_stock(&mut world, farm_house, 50);
        world.accounts.mint(worker, Money::new(10));
        goods_market(&mut world);
        // 10 coins at price 2 → 5 units, capped well below stock and target
        assert_eq!(held(&world, worker, Good::Food), 5);
        assert_eq!(stock_of(&world, farm_house), 45);
        assert_eq!(world.accounts.balance_of(worker, Metal::Gold), Money::ZERO);
        assert_eq!(world.accounts.balance_of(farm, Metal::Gold), Money::new(10));
        world.accounts.audit();
    }

    #[test]
    fn stale_intents_cap_to_live_stock() {
        let mut world = World::new();
        let (farm_house, _, first) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "a",
        );
        let second = world.spawn_agent("b", None, None);
        set_stock(&mut world, farm_house, 10);
        // both plan against the same 10-unit snapshot and could each afford it
        world.accounts.mint(first, Money::new(10));
        world.accounts.mint(second, Money::new(10));
        goods_market(&mut world);
        // agents-order: first drains the shelf, second is capped to zero
        assert_eq!(held(&world, first, Good::Food), 10);
        assert_eq!(held(&world, second, Good::Food), 0);
        assert_eq!(
            world.accounts.balance_of(second, Metal::Gold),
            Money::new(10)
        ); // unspent
        assert_eq!(stock_of(&world, farm_house), 0);
        world.accounts.audit();
    }

    #[test]
    fn broke_buyers_change_nothing() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        set_stock(&mut world, farm_house, 50);
        // no money minted to the worker at all
        goods_market(&mut world);
        assert_eq!(held(&world, worker, Good::Food), 0);
        assert_eq!(stock_of(&world, farm_house), 50);
        assert_eq!(world.accounts.balance_of(farm, Metal::Gold), Money::ZERO);
    }

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
        assert_eq!(world.accounts.total_minted(Metal::Gold), Money::new(35));
        assert_eq!(world.accounts.total_money(Metal::Gold), Money::new(35));
        world.accounts.audit();
    }

    /// The first playable loop, end to end: one farm, one worker, one
    /// unemployed agent, seeded exactly like worldgen (wage bill on the
    /// business; wallet + one day's goods per agent). Every tick audits.
    #[test]
    fn minimal_economy_feeds_the_worker_and_breaks_the_idle() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
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
        // the worldgen seed (3 × 35) is the ENTIRE money supply, forever
        // — the audit pins it there every tick
        assert_eq!(world.accounts.total_minted(Metal::Gold), Money::new(105));
        assert_eq!(world.accounts.total_money(Metal::Gold), Money::new(105));
        // the worker keeps earning, eating, and holding stock
        assert!(world.accounts.balance_of(worker, Metal::Gold) > Money::ZERO);
        assert!(held(&world, worker, Good::Food) > 0);
        // the idle agent earned nothing: wallet drained, pantry empty
        // (07-19 spec: nobody saves the unemployed this milestone)
        assert_eq!(world.accounts.balance_of(idle, Metal::Gold), Money::ZERO);
        assert_eq!(held(&world, idle, Good::Food), 0);
        // overproduction piles up on the shelf (40/tick made, ~10 eaten)
        assert!(stock_of(&world, farm_house) > 0);
    }

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
        assert_eq!(world.accounts.balance_of(farm, Metal::Gold), Money::new(20));
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
}
