//! Houses / living areas: first-class places (spec amendment 1). Finite per
//! area. Firms will later attach to a `HouseId`, possibly shared with homes.

use crate::agent::AgentId;
use crate::business::Business;

/// Identifies one house. Separate from [`AgentId`] on purpose: places are
/// not economic actors and never hold money.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HouseId(pub u32);

/// A place. `owners` is the stored direction of the ownership link;
/// occupants are always derived by scanning agents
/// ([`World::occupants_of`](crate::world::World::occupants_of)) — two
/// stored lists silently desync.
#[derive(Debug)]
pub struct House {
    /// Unique within the world; assigned sequentially by `World::add_house`.
    pub id: HouseId,
    /// Human-readable label, e.g. `"1 Mill Lane"`.
    pub address: String,
    /// Who owns the place. Ownership is stored here; occupancy never is.
    pub owners: Vec<AgentId>,
    /// The business operating here, if any (composition, Amendment 10 — a
    /// house either hosts one or doesn't; capability is data, not a type
    /// split). At most one per house (v1). Attach via
    /// `World::create_business`.
    pub business: Option<Business>,
    // traits TODO: designed together with agent needs (quality, capacity, …)
}
