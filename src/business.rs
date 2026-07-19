//! Businesses: a house's capability to employ, attached by composition
//! (`House::business`, Amendment 10 — never a `BuildingKind` enum).
//! Money-wise a business is only an account id (§8.2): balances live in
//! `Accounts`, never here. Wages are per-role (Amendment 11), never flat.

use std::collections::HashMap;

use crate::agent::AgentId;
use crate::goods::Good;
use crate::money::Money;
use crate::role::Role;

/// One role a business employs: the wage it offers and how many workers it
/// wants. Read by phase 3 (`pay_wages`, via `wage`) and phase 8
/// (`mint_phase`, via `wage_bill`). `wage` types against today's
/// single-metal `Money`; the multi-metal migration pass revises it (single
/// metal vs. bundle is that spec's open question) — don't design around the
/// current type being final.
#[derive(Debug)]
pub struct RoleSlot {
    pub wage: Money,
    pub headcount: u32,
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
        self.roles.values().fold(Money::ZERO, |sum, slot| {
            sum.plus(slot.wage.times(slot.headcount))
        })
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
            },
        );
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(7),
                headcount: 5,
            },
        );
        let business = Business {
            id: AgentId(42),
            product: Good::Food,
            price: Money::new(1),
            stock: 0,
            roles,
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
            },
        );
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(7),
                headcount: 5,
            },
        );
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
}
