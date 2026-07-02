//! The fixed 9-phase tick (parent doc §6). A new mechanic lands INSIDE its
//! phase; adding or reordering phases requires amending the spec's phase
//! contract table. The conservation audit (§8.3) is unconditionally last.

use crate::agent::Agent;
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
fn produce(_world: &mut World) {
    // TODO: firm production lands here.
}

/// Phase 3: firms pay agreed wages. Money ops allowed: transfer only.
fn pay_wages(_world: &mut World) {
    // TODO: wages land here (needs firms).
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
    use crate::money::Money;

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
}
