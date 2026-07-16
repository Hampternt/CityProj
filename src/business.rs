//! Businesses: a house's capability to employ, attached by composition
//! (`House::business`, Amendment 10 — never a `BuildingKind` enum).
//! Money-wise a business is only an account id (§8.2): balances live in
//! `Accounts`, never here. Wages are per-role (Amendment 11), never flat.

// Struct-only refactor: nothing reads these yet. Remove once hiring or
// wage phases land. Same rationale as money.rs's crate allow.
#![allow(dead_code)]

use std::collections::HashMap;

use crate::agent::AgentId;
use crate::money::Money;
use crate::role::Role;

/// One role a business employs: the wage it offers and how many workers it
/// wants. No behavior reads this yet. `wage` types against today's
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
    /// The roles this business employs — one wage/headcount per role
    /// (Amendment 11: role-differentiated, never a flat figure).
    pub roles: HashMap<Role, RoleSlot>,
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
            roles,
        };
        assert_eq!(business.roles[&Role::Engineer].wage, Money::new(12));
        assert_eq!(business.roles[&Role::Engineer].headcount, 2);
        assert_eq!(business.roles[&Role::Labourer].wage, Money::new(7));
        assert_eq!(business.roles[&Role::Labourer].headcount, 5);
    }
}
