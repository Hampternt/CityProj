//! The fixed 9-phase tick (parent doc §6). A new mechanic lands INSIDE its
//! phase; adding or reordering phases requires amending the spec's phase
//! contract table. The conservation audit (§8.3) is unconditionally last.

use crate::agent::{Agent, AgentId};
use crate::housing::HouseId;
use crate::money::Money;
use crate::world::World;

/// What an agent wants to do, decided in a pure pass and executed in an
/// apply pass (see `goods_market` for the worked template). Mechanics add
/// variants; the skeleton has none, so every `match intent {}` is
/// exhaustive and adding a variant is a compile-time forcing function.
pub enum Intent {}

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
        let house = world.house_mut(house_id).expect("collected from businesses()");
        let business = house.business.as_mut().expect("collected from businesses()");
        business.stock += business.product.production_rate();
    }
}

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

/// Phase 4: agents buy goods, prices adjust. Money ops allowed: transfer
/// only. This phase is the WORKED decide→apply TEMPLATE — every behavior
/// phase copies this two-pass shape.
fn goods_market(world: &mut World) {
    // Decide (pure): each agent reads the tick-start snapshot and returns
    // what it WANTS to do. No `&mut` anywhere — unit-testable and free of
    // iteration-order effects.
    let intents: Vec<Intent> = world.agents.iter().flat_map(decide_goods).collect();

    // Apply: the ONLY place this phase moves money. Unaffordable intents
    // fail cleanly (transfer errs) — wanting is unconstrained, paying is not.
    for intent in intents {
        apply_goods_intent(world, intent);
    }
}

/// TODO: needs-driven purchasing lands here. Stays pure.
fn decide_goods(_agent: &Agent) -> Vec<Intent> {
    Vec::new()
}

fn apply_goods_intent(_world: &mut World, intent: Intent) {
    // Exhaustive over zero variants: adding an Intent variant forces every
    // apply fn to handle it at compile time.
    match intent {}
}

/// Phase 5: goods consumed toward needs. Money ops allowed: none.
fn consume(_world: &mut World) {
    // TODO: needs fulfillment lands here.
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
fn mint_phase(_world: &mut World) {
    // TODO: the mint job (and later the gold-backing cap) lands here.
}

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

    #[test]
    fn n_ticks_run_clean() {
        let mut world = World::new();
        for _ in 0..100 {
            tick(&mut world);
        }
        // nothing mints yet, so the money supply must still be zero
        assert_eq!(world.accounts.total_money(), Money::ZERO);
    }

    #[test]
    #[should_panic]
    fn tick_runs_audit_last() {
        let mut world = World::new();
        // corrupt the books via the sanctioned test hook; if any path
        // through tick skipped the audit, this would NOT panic
        world.accounts.set_balance_for_test(AgentId(7), Money::new(999));
        tick(&mut world);
    }

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
}
