//! Houses / living areas: first-class places (spec amendment 1). Finite per
//! area. Firms will later attach to a `HouseId`, possibly shared with homes.

use crate::agent::AgentId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HouseId(pub u32);

/// A place. `owners` is the stored direction of the ownership link;
/// occupants are always derived by scanning agents — two stored lists
/// silently desync.
#[derive(Debug)]
pub struct House {
    pub id: HouseId,
    pub address: String,
    pub owners: Vec<AgentId>,
    // traits TODO: designed together with agent needs (quality, capacity, …)
}
