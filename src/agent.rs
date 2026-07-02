//! Person-agents: the economic actor is the individual person (spec
//! amendment 3). Balances live only in `Accounts`, never on `Agent`.

use crate::housing::HouseId;

/// Identifies one agent. Also keys `Accounts` balances — including the
/// reserved Mint and External accounts, which have no `Agent` struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentId(pub u32);

/// A person. `home`/`workplace` are the stored direction of the agent↔house
/// link; house→occupants is always derived (`World::occupants_of`).
#[derive(Debug)]
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub home: Option<HouseId>,
    pub workplace: Option<HouseId>,
}
