//! Person-agents: the economic actor is the individual person (spec
//! amendment 3). Balances live only in `Accounts`, never on `Agent`.

use crate::housing::HouseId;

/// Identifies one agent. Also keys `Accounts` balances — including the
/// reserved Mint and External accounts, which have no `Agent` struct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AgentId(pub u32);

/// A person. `home`/`workplace` are the stored direction of the agent↔house
/// link; house→occupants is always derived
/// ([`World::occupants_of`](crate::world::World::occupants_of)).
///
/// Note what is *not* here: no balance (lives in
/// [`Accounts`](crate::money::Accounts)) and no pricing or wage logic
/// (lives in the markets).
#[derive(Debug)]
pub struct Agent {
    /// Unique, never reused; ids 0 and 1 are reserved by `World`.
    pub id: AgentId,
    /// Display name; also the inspect key in the interactive shell.
    pub name: String,
    /// Where this agent lives, if anywhere.
    pub home: Option<HouseId>,
    /// Where this agent works, if anywhere. Unused until firms land.
    pub workplace: Option<HouseId>,
}
