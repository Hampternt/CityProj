//! The world: all agents, houses, and the money accounts in one value.
//! Mint and External are reserved as plain accounts here — no structs,
//! no behavior — so phase contracts can name them and ids never get
//! reassigned.

use crate::agent::{Agent, AgentId};
use crate::housing::{House, HouseId};
use crate::money::Accounts;

pub struct World {
    pub agents: Vec<Agent>,
    pub houses: Vec<House>,
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

    pub fn add_house(&mut self, address: &str, owners: Vec<AgentId>) -> HouseId {
        let id = HouseId(self.next_house_id);
        self.next_house_id += 1;
        self.houses.push(House {
            id,
            address: address.to_string(),
            owners,
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

    pub fn agent(&self, id: AgentId) -> Option<&Agent> {
        self.agents.iter().find(|agent| agent.id == id)
    }

    pub fn agent_by_name(&self, name: &str) -> Option<&Agent> {
        self.agents.iter().find(|agent| agent.name == name)
    }

    pub fn house(&self, id: HouseId) -> Option<&House> {
        self.houses.iter().find(|house| house.id == id)
    }

    pub fn agent_mut(&mut self, id: AgentId) -> Option<&mut Agent> {
        self.agents.iter_mut().find(|agent| agent.id == id)
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Money;

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
}
