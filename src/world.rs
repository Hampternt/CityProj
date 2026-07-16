//! The world: all agents, houses, and the money accounts in one value.
//! Mint and External are reserved as plain accounts here — no structs,
//! no behavior — so phase contracts can name them and ids never get
//! reassigned.

use crate::agent::{Agent, AgentId};
use crate::housing::{House, HouseId};
use crate::money::{Accounts, Money, MoneyError};

/// The complete simulation state for one node: who exists, where they live,
/// and every balance. [`sim::tick`](crate::sim::tick) advances exactly one
/// of these per tick. v1 is a single node, but nothing here assumes it stays
/// that way.
pub struct World {
    /// Every person in the node. Reserved ids (Mint, External) have NO entry
    /// here — they are accounts only.
    pub agents: Vec<Agent>,
    /// Every place in the node.
    pub houses: Vec<House>,
    /// The single money book (§8.2). All balances live here, never on
    /// agents.
    pub accounts: Accounts,
    /// Reserved account: the money faucet (§8.4). Plain account, no struct.
    pub mint_id: AgentId,
    /// Reserved account: the out-of-node seam for imports/exports (and
    /// future node-to-node trade). Plain account, no struct.
    pub external_id: AgentId,
    next_agent_id: u32,
    next_house_id: u32,
}

impl World {
    /// An empty world: no agents, no houses, an empty book — only the two
    /// reserved account ids (Mint = 0, External = 1) are claimed, forever.
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            houses: Vec::new(),
            accounts: Accounts::new(),
            mint_id: AgentId(0),
            external_id: AgentId(1),
            next_agent_id: 2, // 0 and 1 are reserved forever
            next_house_id: 0,
        }
    }

    /// Creates a person with the next free id (never a reserved one) and
    /// returns it. No account entry is made — accounts appear at first
    /// credit.
    pub fn spawn_agent(
        &mut self,
        name: &str,
        home: Option<HouseId>,
        workplace: Option<HouseId>,
    ) -> AgentId {
        let id = AgentId(self.next_agent_id);
        self.next_agent_id += 1;
        self.agents.push(Agent {
            id,
            name: name.to_string(),
            home,
            workplace,
        });
        id
    }

    /// Creates a place with the next free [`HouseId`] and returns it.
    pub fn add_house(&mut self, address: &str, owners: Vec<AgentId>) -> HouseId {
        let id = HouseId(self.next_house_id);
        self.next_house_id += 1;
        self.houses.push(House {
            id,
            address: address.to_string(),
            owners,
            business: None,
        });
        id
    }

    /// Derived fresh from agents' `home` fields — never from stored state
    /// (link rule). Unknown house yields empty.
    pub fn occupants_of(&self, house: HouseId) -> Vec<AgentId> {
        self.agents
            .iter()
            .filter(|agent| agent.home == Some(house))
            .map(|agent| agent.id)
            .collect()
    }

    /// Looks up a person by id. Reserved ids return `None` — they have
    /// accounts, not `Agent` structs.
    pub fn agent(&self, id: AgentId) -> Option<&Agent> {
        self.agents.iter().find(|agent| agent.id == id)
    }

    /// Looks up a person by exact name (the interactive shell's inspect
    /// path). First match wins; names are not enforced unique.
    pub fn agent_by_name(&self, name: &str) -> Option<&Agent> {
        self.agents.iter().find(|agent| agent.name == name)
    }

    /// Looks up a place by id.
    pub fn house(&self, id: HouseId) -> Option<&House> {
        self.houses.iter().find(|house| house.id == id)
    }

    /// Mutable lookup by id — how agents move house: rewrite `home` and
    /// derived occupancy follows.
    pub fn agent_mut(&mut self, id: AgentId) -> Option<&mut Agent> {
        self.agents.iter_mut().find(|agent| agent.id == id)
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

/// Why a `World` command refused. The variant names the FIRST failed check;
/// `Err` always means nothing changed (layer property, 07-03 spec). `Money`
/// wraps the core's error unchanged (§8.5 no overdraft).
#[allow(dead_code)] // no phase calls the command layer yet — same rationale as money.rs's crate allow
#[derive(Debug, PartialEq, Eq)]
pub enum WorldError {
    /// The id is neither a spawned agent nor a reserved account — paying it
    /// would silently park money on a phantom account.
    UnknownAgent(AgentId),
    /// No house with this id exists.
    UnknownHouse(HouseId),
    /// The money core refused; wrapped unchanged.
    Money(MoneyError),
}

impl From<MoneyError> for WorldError {
    fn from(err: MoneyError) -> Self {
        WorldError::Money(err)
    }
}

/// The command layer (07-03): validated wrappers that tick phases, worldgen,
/// and the interactive shell all reuse. Every command validates BEFORE
/// touching any state, so `Err` always means nothing changed.
#[allow(dead_code)] // no phase calls these yet — same rationale as money.rs's crate allow
impl World {
    /// Known to the books: a spawned agent or a reserved account id. `pay`'s
    /// guard against parking money on phantom (typo'd) ids — `Accounts`
    /// itself creates accounts implicitly and cannot tell.
    fn is_known_account(&self, id: AgentId) -> bool {
        id == self.mint_id || id == self.external_id || self.agent(id).is_some()
    }

    /// Validated money movement: checks both ids (`from` first), then
    /// forwards to the §8.2 chokepoint unchanged — including the zero and
    /// self-pay no-ops and the §8.5 refusal. Reserved ids are legal in BOTH
    /// positions (sinks pay External; paying Mint merely parks counted
    /// money).
    pub fn pay(&mut self, from: AgentId, to: AgentId, amount: Money) -> Result<(), WorldError> {
        if !self.is_known_account(from) {
            return Err(WorldError::UnknownAgent(from));
        }
        if !self.is_known_account(to) {
            return Err(WorldError::UnknownAgent(to));
        }
        self.accounts.transfer(from, to, amount)?;
        Ok(())
    }

    /// Houses `agent` at `house` (link rule: writes only the agent-side
    /// field; occupancy stays derived). Re-assigning an already-housed
    /// agent moves them.
    pub fn assign_home(&mut self, agent: AgentId, house: HouseId) -> Result<(), WorldError> {
        if self.agent(agent).is_none() {
            return Err(WorldError::UnknownAgent(agent)); // agent checked first
        }
        if self.house(house).is_none() {
            return Err(WorldError::UnknownHouse(house));
        }
        self.agent_mut(agent).expect("existence checked above").home = Some(house);
        Ok(())
    }

    /// Clears `agent`'s home; already-homeless is an Ok no-op.
    pub fn vacate_home(&mut self, agent: AgentId) -> Result<(), WorldError> {
        match self.agent_mut(agent) {
            Some(person) => {
                person.home = None;
                Ok(())
            }
            None => Err(WorldError::UnknownAgent(agent)),
        }
    }

    /// Sets `agent`'s workplace. Identical contract to
    /// [`assign_home`](World::assign_home) on the `workplace` field. No
    /// firm-side checks in v1 — any existing house qualifies; firm
    /// validation arrives via spec amendment when firms land.
    pub fn assign_workplace(&mut self, agent: AgentId, house: HouseId) -> Result<(), WorldError> {
        if self.agent(agent).is_none() {
            return Err(WorldError::UnknownAgent(agent)); // agent checked first
        }
        if self.house(house).is_none() {
            return Err(WorldError::UnknownHouse(house));
        }
        self.agent_mut(agent).expect("existence checked above").workplace = Some(house);
        Ok(())
    }

    /// Clears `agent`'s workplace; already-unemployed is an Ok no-op.
    pub fn vacate_workplace(&mut self, agent: AgentId) -> Result<(), WorldError> {
        match self.agent_mut(agent) {
            Some(person) => {
                person.workplace = None;
                Ok(())
            }
            None => Err(WorldError::UnknownAgent(agent)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::{Money, MoneyError};

    #[test]
    fn reserved_ids_exist() {
        let mut world = World::new();
        assert_ne!(world.mint_id, world.external_id);
        // plain accounts: readable from day one, zero until money moves
        assert_eq!(world.accounts.balance_of(world.mint_id), Money::ZERO);
        assert_eq!(world.accounts.balance_of(world.external_id), Money::ZERO);
        // spawning never reuses a reserved id
        let first = world.spawn_agent("first", None, None);
        assert_ne!(first, world.mint_id);
        assert_ne!(first, world.external_id);
    }

    #[test]
    fn occupants_derived_from_agent_homes() {
        let mut world = World::new();
        let h1 = world.add_house("1 Mill Lane", vec![]);
        let h2 = world.add_house("2 Kiln Row", vec![]);
        let a1 = world.spawn_agent("a1", Some(h1), None);
        let a2 = world.spawn_agent("a2", Some(h1), None);
        let a3 = world.spawn_agent("a3", Some(h2), None);
        assert_eq!(world.occupants_of(h1), vec![a1, a2]);
        // moving an agent is reflected immediately — nothing is stored
        world.agent_mut(a2).unwrap().home = Some(h2);
        assert_eq!(world.occupants_of(h1), vec![a1]);
        assert_eq!(world.occupants_of(h2), vec![a2, a3]);
    }

    #[test]
    fn occupants_of_unknown_house_is_empty() {
        let world = World::new();
        assert!(world.occupants_of(HouseId(99)).is_empty());
    }

    #[test]
    fn pay_moves_money_between_spawned_agents() {
        let mut world = World::new();
        let a = world.spawn_agent("a", None, None);
        let b = world.spawn_agent("b", None, None);
        world.accounts.mint(a, Money::new(100)); // sanctioned test funding
        world.pay(a, b, Money::new(30)).unwrap();
        assert_eq!(world.accounts.balance_of(a), Money::new(70));
        assert_eq!(world.accounts.balance_of(b), Money::new(30));
        world.accounts.audit();
    }

    #[test]
    fn pay_rejects_unknown_ids_before_anything_else() {
        let mut world = World::new();
        let a = world.spawn_agent("a", None, None);
        let ghost = AgentId(99);
        // both unknown: `from` is reported (checked first)
        assert_eq!(
            world.pay(ghost, ghost, Money::new(5)),
            Err(WorldError::UnknownAgent(ghost))
        );
        // validation precedes the zero no-op (spec edge: rejecting phantom
        // ids is the point of this method)
        assert_eq!(
            world.pay(ghost, a, Money::ZERO),
            Err(WorldError::UnknownAgent(ghost))
        );
        // `to` is validated too — the phantom-account defence
        assert_eq!(
            world.pay(a, ghost, Money::new(5)),
            Err(WorldError::UnknownAgent(ghost))
        );
        // nothing moved on any Err
        assert_eq!(world.accounts.total_money(), Money::ZERO);
    }

    #[test]
    fn pay_surfaces_insufficient_funds_unchanged() {
        let mut world = World::new();
        let a = world.spawn_agent("a", None, None);
        let b = world.spawn_agent("b", None, None);
        world.accounts.mint(a, Money::new(10));
        assert_eq!(
            world.pay(a, b, Money::new(20)),
            Err(WorldError::Money(MoneyError::InsufficientFunds))
        );
        // §8.5 atomic — books untouched
        assert_eq!(world.accounts.balance_of(a), Money::new(10));
        assert_eq!(world.accounts.balance_of(b), Money::ZERO);
    }

    #[test]
    fn pay_allows_reserved_ids_both_ends() {
        let mut world = World::new();
        world.accounts.mint(world.mint_id, Money::new(50));
        world
            .pay(world.mint_id, world.external_id, Money::new(20))
            .unwrap();
        assert_eq!(world.accounts.balance_of(world.external_id), Money::new(20));
        world.accounts.audit();
    }

    #[test]
    fn assign_home_sets_and_moves() {
        let mut world = World::new();
        let h1 = world.add_house("1 Mill Lane", vec![]);
        let h2 = world.add_house("2 Kiln Row", vec![]);
        let a = world.spawn_agent("a", None, None);
        world.assign_home(a, h1).unwrap();
        assert_eq!(world.occupants_of(h1), vec![a]);
        // re-assigning moves — derived occupancy follows (link rule)
        world.assign_home(a, h2).unwrap();
        assert!(world.occupants_of(h1).is_empty());
        assert_eq!(world.occupants_of(h2), vec![a]);
    }

    #[test]
    fn assign_home_checks_agent_then_house() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        let a = world.spawn_agent("a", None, None);
        let ghost_agent = AgentId(99);
        let ghost_house = HouseId(99);
        assert_eq!(
            world.assign_home(ghost_agent, house),
            Err(WorldError::UnknownAgent(ghost_agent))
        );
        assert_eq!(
            world.assign_home(a, ghost_house),
            Err(WorldError::UnknownHouse(ghost_house))
        );
        // both unknown: agent reported (checked first)
        assert_eq!(
            world.assign_home(ghost_agent, ghost_house),
            Err(WorldError::UnknownAgent(ghost_agent))
        );
        // reserved ids are NOT agents — accounts, not Agent structs
        let mint = world.mint_id;
        assert_eq!(
            world.assign_home(mint, house),
            Err(WorldError::UnknownAgent(mint))
        );
        // nothing changed on any Err
        assert!(world.occupants_of(house).is_empty());
    }

    #[test]
    fn vacate_home_clears_and_tolerates_homeless() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        let a = world.spawn_agent("a", Some(house), None);
        world.vacate_home(a).unwrap();
        assert_eq!(world.agent(a).unwrap().home, None);
        // already-homeless is an Ok no-op
        world.vacate_home(a).unwrap();
        let ghost = AgentId(99);
        assert_eq!(world.vacate_home(ghost), Err(WorldError::UnknownAgent(ghost)));
    }

    #[test]
    fn assign_workplace_sets_and_moves() {
        let mut world = World::new();
        let h1 = world.add_house("1 Mill Lane", vec![]);
        let h2 = world.add_house("2 Kiln Row", vec![]);
        let a = world.spawn_agent("a", None, None);
        world.assign_workplace(a, h1).unwrap();
        assert_eq!(world.agent(a).unwrap().workplace, Some(h1));
        // re-assigning moves the workplace link
        world.assign_workplace(a, h2).unwrap();
        assert_eq!(world.agent(a).unwrap().workplace, Some(h2));
    }

    #[test]
    fn assign_workplace_checks_agent_then_house() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        let a = world.spawn_agent("a", None, None);
        let ghost_agent = AgentId(99);
        let ghost_house = HouseId(99);
        assert_eq!(
            world.assign_workplace(ghost_agent, house),
            Err(WorldError::UnknownAgent(ghost_agent))
        );
        assert_eq!(
            world.assign_workplace(a, ghost_house),
            Err(WorldError::UnknownHouse(ghost_house))
        );
        assert_eq!(
            world.assign_workplace(ghost_agent, ghost_house),
            Err(WorldError::UnknownAgent(ghost_agent))
        );
        // nothing changed on any Err
        assert_eq!(world.agent(a).unwrap().workplace, None);
    }

    #[test]
    fn vacate_workplace_clears_and_tolerates_unemployed() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        let a = world.spawn_agent("a", None, Some(house));
        world.vacate_workplace(a).unwrap();
        assert_eq!(world.agent(a).unwrap().workplace, None);
        // already-unemployed is an Ok no-op
        world.vacate_workplace(a).unwrap();
        let ghost = AgentId(99);
        assert_eq!(
            world.vacate_workplace(ghost),
            Err(WorldError::UnknownAgent(ghost))
        );
    }

    #[test]
    fn houses_start_without_business() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        assert!(world.house(house).unwrap().business.is_none());
    }
}
