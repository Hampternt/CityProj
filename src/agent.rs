//! Person-agents: the economic actor is the individual person (spec
//! amendment 3). Balances live only in `Accounts`, never on `Agent`.

use crate::housing::HouseId;
use crate::role::Role;

/// Identifies one agent. Also keys `Accounts` balances — including the
/// reserved Mint/External accounts AND business ids, which have accounts
/// but no `Agent` struct.
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
    /// Role this agent is trained for; `None` = unspecialized/general
    /// labour. Independent of `employed_role` — agents can work off-spec.
    #[allow(dead_code)]
    pub specialization: Option<Role>,
    /// Role currently filled at `workplace`. Stored, not derived — nothing
    /// else records it. Intended invariant, documented but NOT yet
    /// enforced: `employed_role.is_some()` implies `workplace.is_some()`;
    /// enforcement belongs to the future `assign_workplace` extension.
    #[allow(dead_code)]
    pub employed_role: Option<Role>,
    /// Reserved skill scalar with no defined effect yet — range and meaning
    /// are decided by the first spec that reads it.
    #[allow(dead_code)]
    pub education: u8,
}
