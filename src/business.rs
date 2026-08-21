//! Businesses: a house's capability to employ, attached by composition
//! (`House::business`, Amendment 10 — never a `BuildingKind` enum).
//! Money-wise a business is only an account id (§8.2): balances live in
//! `Accounts`, never here. Wages are per-role (Amendment 11), never flat.

use std::collections::HashMap;

use crate::agent::AgentId;
use crate::goods::Good;
use crate::money::Money;
use crate::role::Role;

/// One role a business employs: the wage it offers and how many workers
/// it wants. Read by phase 3 (`pay_wages`, via `wage`). `wage` types
/// against today's single-metal `Money`; the multi-metal migration pass
/// revises it (single metal vs. bundle is that spec's open question) —
/// don't design around the current type being final.
#[derive(Debug)]
pub struct RoleSlot {
    pub wage: Money,
    pub headcount: u32,
    /// Consecutive ticks this slot has had open headcount, measured
    /// post-matching (town-colony pack 4: the vacancy-pull age phase 1's
    /// Arrive rule reads). SINGLE WRITER: phase 1's write-back —
    /// incremented while open, reset to 0 when fully staffed. Worldgen
    /// seeds 0; boot-time openings age from tick 1.
    pub unfilled_ticks: u32,
}

/// A business attached to a house via `House::business`. `id` keys
/// `Accounts` like any agent id but has NO `Agent` struct behind it —
/// account-only, same category as the reserved Mint/External ids. No
/// balance field here, ever (§8.2). `inputs`/`outputs` deliberately absent
/// — deferred to a future `goods.rs` spec.
#[derive(Debug)]
pub struct Business {
    /// Account key in [`Accounts`](crate::money::Accounts), allocated by
    /// `World::create_business` from the shared agent-id counter.
    pub id: AgentId,
    /// The one good this business produces and sells (v1: single-product,
    /// single-node). Production chains are a future spec.
    pub product: Good,
    /// Posted unit price, seeded at worldgen and adjusted every tick by
    /// phase 4's `market::adjust_price` write-back. Price *data* lives
    /// here; pricing *logic* stays in `market.rs` (§8.6).
    pub price: Money,
    /// Unsold units on hand: phase 2 adds, phase 4 sells.
    pub stock: u32,
    /// The roles this business employs — one wage/headcount per role
    /// (Amendment 11: role-differentiated, never a flat figure).
    pub roles: HashMap<Role, RoleSlot>,
    /// Wage debt per worker (arrears, 07-19 pricing spec): phase 3
    /// accrues each tick's wage here, pays what the coffers cover via a
    /// normal transfer, and keeps the shortfall. Only the current employee's
    /// debt is paid down by phase 3; a departed worker's entry persists
    /// unpaid until a future job-switching/payout mechanic reads the ledger.
    /// Entries are removed at zero — an empty map means fully paid.
    /// Bookkeeping only, never a negative balance (§8.2/§8.5).
    pub owed_to: HashMap<AgentId, Money>,
}

impl Business {
    /// One tick of full staffing: sum over role slots of wage × headcount.
    /// Worldgen seeds each business with exactly this so tick 1's wages
    /// (paid before any revenue) never skip; nothing reads it per-tick
    /// since the 07-19 spec closed the phase-8 faucet.
    pub fn wage_bill(&self) -> Money {
        self.roles.values().fold(Money::ZERO, |sum, slot| {
            sum.plus(slot.wage.times(slot.headcount))
        })
    }

    /// Total outstanding wage debt across all workers. Display and
    /// diagnostics; phase 3 works per-worker, not from this sum.
    pub fn owed_total(&self) -> Money {
        self.owed_to
            .values()
            .fold(Money::ZERO, |sum, &owed| sum.plus(owed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentId;

    #[test]
    fn business_roles_map_holds_distinct_wages_per_role() {
        let mut roles = HashMap::new();
        roles.insert(
            Role::Engineer,
            RoleSlot {
                wage: Money::new(12),
                headcount: 2,
                unfilled_ticks: 0,
            },
        );
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(7),
                headcount: 5,
                unfilled_ticks: 0,
            },
        );
        let business = Business {
            id: AgentId(42),
            product: Good::Food,
            price: Money::new(1),
            stock: 0,
            roles,
            owed_to: HashMap::new(),
        };
        assert_eq!(business.roles[&Role::Engineer].wage, Money::new(12));
        assert_eq!(business.roles[&Role::Engineer].headcount, 2);
        assert_eq!(business.roles[&Role::Labourer].wage, Money::new(7));
        assert_eq!(business.roles[&Role::Labourer].headcount, 5);
    }

    #[test]
    fn wage_bill_sums_wage_times_headcount_over_slots() {
        let mut roles = HashMap::new();
        roles.insert(
            Role::Engineer,
            RoleSlot {
                wage: Money::new(12),
                headcount: 2,
                unfilled_ticks: 0,
            },
        );
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(7),
                headcount: 5,
                unfilled_ticks: 0,
            },
        );
        let business = Business {
            id: AgentId(42),
            product: Good::Food,
            price: Money::new(1),
            stock: 0,
            roles,
            owed_to: HashMap::new(),
        };
        // 12×2 + 7×5
        assert_eq!(business.wage_bill(), Money::new(59));
        let empty = Business {
            id: AgentId(43),
            product: Good::Luxury,
            price: Money::new(5),
            stock: 0,
            roles: HashMap::new(),
            owed_to: HashMap::new(),
        };
        assert_eq!(empty.wage_bill(), Money::ZERO);
    }

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
}
