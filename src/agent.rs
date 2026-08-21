//! Person-agents: the economic actor is the individual person (spec
//! amendment 3). Balances live only in `Accounts`, never on `Agent`.

use std::collections::HashMap;

use crate::goods::Good;
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
    /// Where this agent works, if anywhere. Read by `World::employees_of`
    /// for the staffed-business phases; hiring itself lands with the
    /// labor market.
    pub workplace: Option<HouseId>,
    /// Role this agent is trained for; `None` = unspecialized/general
    /// labour. Independent of `employed_role` — agents can work off-spec.
    #[allow(dead_code)]
    pub specialization: Option<Role>,
    /// Role currently filled at `workplace`. Stored, not derived — nothing
    /// else records it. Invariant: `employed_role.is_some()` implies
    /// `workplace.is_some()` — the workplace commands write and clear the
    /// pair together (`World::assign_workplace` takes the role;
    /// `vacate_workplace` clears both). Worldgen sets the fields directly
    /// and keeps the pair consistent by construction.
    pub employed_role: Option<Role>,
    /// Reserved skill scalar with no defined effect yet — range and meaning
    /// are decided by the first spec that reads it.
    #[allow(dead_code)]
    pub education: u8,
    /// Personal stock of goods: refilled by phase-4 purchases, drained by
    /// phase-5 consumption. Missing entry reads as 0. Goods are not money
    /// — no conservation audit applies.
    pub inventory: HashMap<Good, u32>,
    /// Consecutive ticks short of Food (town-colony spec, gate ruling 3 —
    /// a deliberately throwaway stopgap ahead of the needs model).
    /// SINGLE WRITER: phase-5 consume — saturating-increments on a tick
    /// where Food can't cover one `consumption_rate`, resets to 0 on a
    /// fed tick. Read by phase 7's Depart rule and the shell.
    pub hunger: u8,
}
