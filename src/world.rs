//! The world: all agents, houses, and the money accounts in one value.
//! Mint and External are reserved as plain accounts here — no structs,
//! no behavior — so phase contracts can name them and ids never get
//! reassigned.

use crate::agent::{Agent, AgentId};
use crate::business::{Business, RoleSlot};
use crate::goods::Good;
use crate::housing::{House, HouseId};
use crate::metal::Metal;
use crate::money::{Accounts, Money, MoneyError};
use crate::role::Role;
use std::collections::HashMap;

/// The complete simulation state for one node: who exists, where they live,
/// and every balance. [`sim::tick`](crate::sim::tick) advances exactly one
/// of these per tick. v1 is a single node, but nothing here assumes it stays
/// that way.
pub struct World {
    /// Every person in the node. Reserved ids (Mint, External) and business
    /// ids have NO entry here — they are accounts only.
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
    /// How many immigrants have ever arrived — the deterministic
    /// name-table counter (town-colony pack 4). Written only by
    /// [`immigrate`](World::immigrate); never decremented, so names
    /// stay unique across departures.
    pub arrivals: u32,
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
            arrivals: 0,
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
            specialization: None,
            employed_role: None,
            education: 0,
            inventory: HashMap::new(),
            hunger: 0,
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

    /// Mutable lookup by id — mirrors [`agent_mut`](World::agent_mut).
    pub fn house_mut(&mut self, id: HouseId) -> Option<&mut House> {
        self.houses.iter_mut().find(|house| house.id == id)
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
#[derive(Debug, PartialEq, Eq)]
pub enum WorldError {
    /// The id is neither a spawned agent, a reserved account, nor an
    /// existing business id (Am. 14) — paying it would silently park money
    /// on a phantom account.
    UnknownAgent(AgentId),
    /// No house with this id exists.
    UnknownHouse(HouseId),
    /// The house already hosts a business — at most one per house (v1).
    BusinessAlreadyExists(HouseId),
    /// The house exists but hosts no business — the structural inverse of
    /// [`BusinessAlreadyExists`](WorldError::BusinessAlreadyExists).
    /// Refused by `close_business`.
    NoBusinessHere(HouseId),
    /// Not a vacant residence: it has occupants or hosts a business —
    /// v1's entire vacancy rule (ownership plays no part). Refused by
    /// `immigrate`.
    HouseNotVacant(HouseId),
    /// The money core refused; wrapped unchanged.
    Money(MoneyError),
}

/// What one `close_business` actually moved, step by step (firm-lifecycle
/// spec, pack 2). The **event-measurement mechanism**: balance deltas
/// measured around the *whole* command cannot attribute flows that share
/// a wallet, and the canonical case is the owner-as-creditor — every
/// shipped owner is their venue's first seeded worker, so a dying firm
/// settles its owner in step 1 AND sweeps them the residual in step 3,
/// leaving `Settled` and `Closed.proceeds` underdetermined from outside.
/// Each amount here is measured around its own internal `pay`; callers
/// emit events FROM the receipt, never by re-deriving.
#[derive(Debug, Clone, PartialEq)]
pub struct ClosureReceipt {
    /// The dead firm's account id. Resolves to nothing once the command
    /// returns — `is_known_account` scans the live `businesses()` set.
    pub business: AgentId,
    /// The freed house. Carried because `business` no longer resolves,
    /// and because the house is what the shell can still name.
    pub house: HouseId,
    /// Who took the residual. Alive at every call site's emission point,
    /// so the caller supplies the display name (no `String` here).
    pub owner: AgentId,
    /// Step 1: what each creditor was actually paid, ascending `AgentId`.
    /// POSITIVE amounts only — the receipt means "what moved" (the
    /// `apply_sinks_intent` `> ZERO` filter, applied at source). Written
    /// -off remainders are silent: the preceding `PayrollShort`s already
    /// told that story.
    pub settlements: Vec<(AgentId, Money)>,
    /// Step 2: everyone whose workplace this was, ascending `AgentId`
    /// (`employees_of` order). They re-enter the applicant pool next tick.
    pub laid_off: Vec<AgentId>,
    /// Step 3: the per-metal residual swept to the owner — every
    /// `Metal::ALL` entry in that order, **zeros included** (the
    /// `Departed { took }` visible-zeros precedent, D3).
    pub residual: Vec<(Metal, Money)>,
}

impl From<MoneyError> for WorldError {
    fn from(err: MoneyError) -> Self {
        WorldError::Money(err)
    }
}

/// The command layer (07-03): validated wrappers that tick phases, worldgen,
/// and the interactive shell all reuse. Every command validates BEFORE
/// touching any state, so `Err` always means nothing changed.
impl World {
    /// Known to the books: a spawned agent, a reserved account id, or an
    /// existing business id (Amendment 14). `pay`'s guard against parking
    /// money on phantom (typo'd) ids — `Accounts` itself creates accounts
    /// implicitly and cannot tell.
    fn is_known_account(&self, id: AgentId) -> bool {
        id == self.mint_id
            || id == self.external_id
            || self.agent(id).is_some()
            || self.businesses().any(|(_, business)| business.id == id)
    }

    /// Validated money movement: checks both ids (`from` first), then
    /// forwards to the §8.2 chokepoint unchanged — including the zero and
    /// self-pay no-ops and the §8.5 refusal. Reserved ids are legal in BOTH
    /// positions (sinks pay External; paying Mint merely parks counted
    /// money).
    pub fn pay(
        &mut self,
        from: AgentId,
        to: AgentId,
        metal: Metal,
        amount: Money,
    ) -> Result<(), WorldError> {
        if !self.is_known_account(from) {
            return Err(WorldError::UnknownAgent(from));
        }
        if !self.is_known_account(to) {
            return Err(WorldError::UnknownAgent(to));
        }
        self.accounts.transfer(from, to, metal, amount)?;
        Ok(())
    }

    /// Houses `agent` at `house` (link rule: writes only the agent-side
    /// field; occupancy stays derived). Re-assigning an already-housed
    /// agent moves them.
    #[allow(dead_code)] // no caller until the labor market lands
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
    #[allow(dead_code)] // no caller until the labor market lands
    pub fn vacate_home(&mut self, agent: AgentId) -> Result<(), WorldError> {
        match self.agent_mut(agent) {
            Some(person) => {
                person.home = None;
                Ok(())
            }
            None => Err(WorldError::UnknownAgent(agent)),
        }
    }

    /// Hires `agent` into `role` at `house`: writes `workplace` and
    /// `employed_role` together (town-colony spec, Intent contract — a
    /// hire that set only `workplace` would fill the slot yet never
    /// earn, since `pay_wages` pays on `employed_role`). Same validation
    /// shape as [`assign_home`](World::assign_home); no firm-side checks
    /// in v1 — any existing house qualifies, and headcount capping is
    /// the labor-market apply's job, not this command's.
    pub fn assign_workplace(
        &mut self,
        agent: AgentId,
        house: HouseId,
        role: Role,
    ) -> Result<(), WorldError> {
        if self.agent(agent).is_none() {
            return Err(WorldError::UnknownAgent(agent)); // agent checked first
        }
        if self.house(house).is_none() {
            return Err(WorldError::UnknownHouse(house));
        }
        let person = self.agent_mut(agent).expect("existence checked above");
        person.workplace = Some(house);
        person.employed_role = Some(role);
        Ok(())
    }

    /// Clears `agent`'s workplace AND `employed_role` together (the
    /// employed_role-implies-workplace invariant survives quitting);
    /// already-unemployed is an Ok no-op.
    pub fn vacate_workplace(&mut self, agent: AgentId) -> Result<(), WorldError> {
        match self.agent_mut(agent) {
            Some(person) => {
                person.workplace = None;
                person.employed_role = None;
                Ok(())
            }
            None => Err(WorldError::UnknownAgent(agent)),
        }
    }

    /// Attaches a new business to `house`, owned by `owner`, allocating
    /// its account id from the same counter as `spawn_agent` — never a
    /// reserved id, never reused, and NO `Agent` struct is created
    /// (business ids are account-only, like Mint/External). `owner` must
    /// be a real spawned agent (firm-lifecycle spec: every business
    /// always names a living owner — reserved ids, business ids, and
    /// ghosts refuse) and is checked FIRST, per the agent-checked-first
    /// convention of the other commands. Starts with zero stock; product
    /// is fixed at creation, price is the initial posted price; phase 4's
    /// `market::adjust_price` adjusts price each tick based on
    /// sell-through. Validates before touching state: `Err` means nothing
    /// changed.
    pub fn create_business(
        &mut self,
        house: HouseId,
        owner: AgentId,
        product: Good,
        price: Money,
        roles: HashMap<Role, RoleSlot>,
    ) -> Result<AgentId, WorldError> {
        if self.agent(owner).is_none() {
            return Err(WorldError::UnknownAgent(owner)); // owner checked first
        }
        match self.house(house) {
            None => return Err(WorldError::UnknownHouse(house)),
            Some(existing) if existing.business.is_some() => {
                return Err(WorldError::BusinessAlreadyExists(house));
            }
            Some(_) => {}
        }
        let id = AgentId(self.next_agent_id);
        self.next_agent_id += 1;
        self.house_mut(house)
            .expect("existence checked above")
            .business = Some(Business {
            id,
            owner,
            product,
            price,
            stock: 0,
            roles,
            owed_to: HashMap::new(),
            insolvent_ticks: 0,
        });
        Ok(id)
    }

    /// Every house that hosts a business, paired with it, in `houses` order
    /// — the ONE shared query future phases (`labor_market`, `produce`,
    /// `pay_wages`, `invest`) use to find businesses, each on its own turn
    /// under its own money-permission contract (Amendment 13: no
    /// per-entity-type resolve phase). Read-only; a `businesses_mut` is
    /// future work, added only when a phase mutates `Business` fields.
    pub fn businesses(&self) -> impl Iterator<Item = (&House, &Business)> {
        self.houses
            .iter()
            .filter_map(|house| house.business.as_ref().map(|business| (house, business)))
    }

    /// Every agent working at `house`, in ascending `AgentId` order —
    /// `agents` is append-ordered by the ascending id counter, so a scan
    /// IS id order (pinned by test, never re-sorted). Derived per the
    /// link rule, never stored (mirrors
    /// [`occupants_of`](World::occupants_of)); unstaffed and unknown
    /// houses yield empty. The deterministic-order contract every
    /// contended decide pass inherits (town-colony spec). Replaced the
    /// single-headcount `employee_of` in pack 2.
    pub fn employees_of(&self, house: HouseId) -> Vec<AgentId> {
        self.agents
            .iter()
            .filter(|agent| agent.workplace == Some(house))
            .map(|agent| agent.id)
            .collect()
    }

    /// Liquidates the business at `house` — the closure command
    /// (firm-lifecycle pack 2). Validates first: the house must exist
    /// (`UnknownHouse`) and host a business (`NoBusinessHere`); `Err`
    /// means nothing changed. Then, in order:
    ///
    /// 1. **Creditor settlement** — every `owed_to` entry, current AND
    ///    former workers, keys explicitly sorted ascending `AgentId`
    ///    (the ledger is a `HashMap`; the no-RNG guarantee is only as
    ///    good as pinned iteration), each paid `min(remaining gold
    ///    coffer, owed)`. Remainders are written off and the whole
    ///    ledger is cleared — zero-amount entries included, since
    ///    `pay_wages` inserts unconditionally and no entry may keep
    ///    naming a dead firm.
    /// 2. **Layoffs** — `vacate_workplace` for every `employees_of`,
    ///    clearing `workplace` and `employed_role` together. They
    ///    re-enter the applicant pool next tick.
    /// 3. **Residual sweep** — every `Metal::ALL` balance to the
    ///    **owner's** wallet. Liquidation proceeds belong to a living
    ///    resident and stay in-node; `External` is the seam for money
    ///    *leaving* the node, which this is not. Completeness is proven
    ///    per-account by no-orphan assertions on the dead id — the
    ///    totals-only audit cannot see a conservation-legal orphan.
    /// 4. **Detach** — `house.business = None`, LAST, and mechanically
    ///    so: [`is_known_account`](World::is_known_account) answers "is
    ///    this a business id?" by scanning the live `businesses()` set,
    ///    so detaching is a capability revocation — the instant it runs,
    ///    every `pay` naming that id refuses and the `.expect()` idiom
    ///    would panic. Afterwards the house — zero occupants, hosting
    ///    nothing — satisfies the `immigrate` vacancy rule verbatim:
    ///    closure manufactures landing pads by design.
    ///
    /// Atomic by construction after validation: every amount is
    /// min-bounded by a live balance and both ids are known. The firm's
    /// `stock` dies with its `Business` (pack-2 decision D6: the sim's
    /// first goods sink — no invariant is touched, since goods carry no
    /// conservation rule, and the physical-goods spec inherits the
    /// question). Steady-state caller: phase 6's closure pass. Forced
    /// caller: [`remove_agent`](World::remove_agent) (Amendment 19).
    pub fn close_business(&mut self, house: HouseId) -> Result<ClosureReceipt, WorldError> {
        let Some(existing) = self.house(house) else {
            return Err(WorldError::UnknownHouse(house));
        };
        let Some(business) = existing.business.as_ref() else {
            return Err(WorldError::NoBusinessHere(house));
        };
        let business_id = business.id;
        let owner = business.owner;
        // Collect the ledger out before any `pay`: holding a `&Business`
        // across `self.pay` is E0502 (remove_agent's shape, mirrored).
        // `AgentId` derives no `Ord`, so sort on the inner u32.
        let mut debts: Vec<(AgentId, Money)> = business
            .owed_to
            .iter()
            .map(|(&creditor, &owed)| (creditor, owed))
            .collect();
        debts.sort_by_key(|(creditor, _)| creditor.0);

        let mut settlements = Vec::new();
        for (creditor, owed) in debts {
            let settlement = self.accounts.balance_of(business_id, Metal::Gold).min(owed);
            if settlement > Money::ZERO {
                self.pay(business_id, creditor, Metal::Gold, settlement)
                    .expect("min-bounded by the live coffer, both ids validated");
                settlements.push((creditor, settlement));
            }
        }
        // Remainders written off; one clear also strips the zero-amount
        // entries the settlement filter above never sees.
        self.house_mut(house)
            .expect("existence checked above")
            .business
            .as_mut()
            .expect("existence checked above")
            .owed_to
            .clear();

        let laid_off = self.employees_of(house);
        for worker in &laid_off {
            self.vacate_workplace(*worker)
                .expect("collected from employees_of");
        }

        let mut residual = Vec::new();
        for metal in Metal::ALL {
            let balance = self.accounts.balance_of(business_id, metal);
            residual.push((metal, balance)); // zeros included, D3
            if balance > Money::ZERO {
                self.pay(business_id, owner, metal, balance)
                    .expect("min-bounded by the live balance, both ids validated");
            }
        }

        self.house_mut(house)
            .expect("existence checked above")
            .business = None; // LAST — retires the account id
        Ok(ClosureReceipt {
            business: business_id,
            house,
            owner,
            settlements,
            laid_off,
            residual,
        })
    }

    /// Removes `agent` from the world — the emigration command
    /// (town-colony pack 4; Amendment 17). Validates first: only a real
    /// spawned agent qualifies — reserved and business ids refuse — and
    /// `Err(WorldError::UnknownAgent)` means nothing changed. Then, in
    /// order: every business still owing the leaver settles
    /// `min(gold coffer, owed)` into their wallet and the remainder is
    /// written off (gate ruling 2 — the ledger entry goes either way);
    /// every `Metal::ALL` balance sweeps to External through the §8.2
    /// chokepoint, settlement included, inert silver/copper included —
    /// no orphan balance survives, proven per-account by tests (the
    /// totals-only audit cannot see a conservation-legal orphan); the
    /// leaver's id is stripped from every `House.owners`; the `Agent` is
    /// removed, which clears home/workplace/employed_role with it since
    /// occupancy and staffing are derived, never stored. After
    /// validation no internal transfer can fail — every amount is
    /// min-bounded by a live balance and both ids are known — so the
    /// command is atomic by construction.
    pub fn remove_agent(&mut self, agent: AgentId) -> Result<Vec<ClosureReceipt>, WorldError> {
        if self.agent(agent).is_none() {
            return Err(WorldError::UnknownAgent(agent));
        }
        // Step 0 (Amendment 19): the leaver's own firms are liquidated
        // FIRST — before the A17 settlement, so any debt those firms owe
        // the leaver is paid by `close_business`'s own creditor pass and
        // the proceeds ride the sweep below. The `Vec<HouseId>` snapshot
        // is mandatory: iterating `businesses()` while calling
        // `close_business` is E0502. It is also safe against
        // mutation-during-iteration, since closing only detaches houses
        // already named in it. The leaver is still a known account here
        // (removal is last), so the residual `pay` to them succeeds.
        let owned: Vec<HouseId> = self
            .businesses()
            .filter(|(_, business)| business.owner == agent)
            .map(|(house, _)| house.id)
            .collect();
        let mut receipts = Vec::new();
        for house in owned {
            receipts.push(
                self.close_business(house)
                    .expect("collected from businesses()"),
            );
        }
        // Settlement (Amendment 17): businesses in houses order. Firms
        // closed in step 0 are already detached, so they cannot be
        // double-settled here — no exclusion filter is needed at this
        // site (the one the spec mandates belongs to the CALLER's
        // creditors snapshot, which is taken before the command).
        let debts: Vec<(HouseId, AgentId, Money)> = self
            .businesses()
            .filter_map(|(house, business)| {
                business
                    .owed_to
                    .get(&agent)
                    .copied()
                    .filter(|&owed| owed > Money::ZERO)
                    .map(|owed| (house.id, business.id, owed))
            })
            .collect();
        for (_, business_id, owed) in debts {
            let settlement = self.accounts.balance_of(business_id, Metal::Gold).min(owed);
            if settlement > Money::ZERO {
                self.pay(business_id, agent, Metal::Gold, settlement)
                    .expect("min-bounded by the live coffer, both ids validated");
            }
        }
        // Strip the leaver from EVERY ledger, zero-amount entries
        // included (pay_wages inserts unconditionally, so a zero-wage
        // slot leaves a 0g entry the settlement filter above never
        // sees) — no entry may keep naming a removed id.
        for house in &mut self.houses {
            if let Some(business) = house.business.as_mut() {
                business.owed_to.remove(&agent); // remainder written off
            }
        }
        // The per-metal sweep: everything they hold goes to External.
        for metal in Metal::ALL {
            let balance = self.accounts.balance_of(agent, metal);
            if balance > Money::ZERO {
                self.pay(agent, self.external_id, metal, balance)
                    .expect("min-bounded by the live balance, both ids validated");
            }
        }
        for house in &mut self.houses {
            house.owners.retain(|&owner| owner != agent);
        }
        self.agents.retain(|person| person.id != agent);
        Ok(receipts)
    }

    /// The migration-gated arrival command (town-colony pack 4) — a
    /// distinct name, NOT a change to `spawn_agent` (07-13's "do not
    /// widen the constructor" stands; Rust has no overloading, a trap
    /// the multi-metal ledger already recorded). Validates the house
    /// exists and is a vacant residence — zero occupants AND hosts no
    /// business, v1's entire vacancy rule — then builds on the untouched
    /// constructor: next id, housed at `home`, unemployed, empty
    /// inventory, zero balances, hunger 0. Moves no money (07-03's
    /// refusal of free-money wrappers stands — the grubstake is a
    /// separate, capped `pay` in the Arrive apply, so a failed stake
    /// leaves a penniless-but-valid newcomer). Bumps `arrivals`, the
    /// immigrant-name counter. `Err` means nothing changed.
    pub fn immigrate(&mut self, name: String, home: HouseId) -> Result<AgentId, WorldError> {
        match self.house(home) {
            None => return Err(WorldError::UnknownHouse(home)),
            Some(house) if house.business.is_some() => {
                return Err(WorldError::HouseNotVacant(home));
            }
            Some(_) if !self.occupants_of(home).is_empty() => {
                return Err(WorldError::HouseNotVacant(home));
            }
            Some(_) => {}
        }
        let id = self.spawn_agent(&name, Some(home), None);
        self.arrivals += 1;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goods::Good;
    use crate::metal::Metal;
    use crate::money::{Money, MoneyError};
    use std::collections::HashMap;

    #[test]
    fn reserved_ids_exist() {
        let mut world = World::new();
        assert_ne!(world.mint_id, world.external_id);
        // plain accounts: readable from day one, zero until money moves
        assert_eq!(
            world.accounts.balance_of(world.mint_id, Metal::Gold),
            Money::ZERO
        );
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Gold),
            Money::ZERO
        );
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
        world.accounts.mint(a, Metal::Gold, Money::new(100)); // sanctioned test funding
        world.pay(a, b, Metal::Gold, Money::new(30)).unwrap();
        assert_eq!(world.accounts.balance_of(a, Metal::Gold), Money::new(70));
        assert_eq!(world.accounts.balance_of(b, Metal::Gold), Money::new(30));
        world.accounts.audit();
    }

    #[test]
    fn pay_moves_only_the_named_metal() {
        let mut world = World::new();
        let a = world.spawn_agent("a", None, None);
        let b = world.spawn_agent("b", None, None);
        world.accounts.mint(a, Metal::Gold, Money::new(100)); // sanctioned test funding
        world.accounts.mint(a, Metal::Silver, Money::new(40));
        world.pay(a, b, Metal::Silver, Money::new(25)).unwrap();
        // silver moved…
        assert_eq!(world.accounts.balance_of(a, Metal::Silver), Money::new(15));
        assert_eq!(world.accounts.balance_of(b, Metal::Silver), Money::new(25));
        // …and gold — balance and totals — is untouched
        assert_eq!(world.accounts.balance_of(a, Metal::Gold), Money::new(100));
        assert_eq!(world.accounts.balance_of(b, Metal::Gold), Money::ZERO);
        assert_eq!(world.accounts.total_money(Metal::Gold), Money::new(100));
        assert_eq!(world.accounts.total_minted(Metal::Gold), Money::new(100));
        world.accounts.audit();
    }

    #[test]
    fn pay_rejects_unknown_ids_before_anything_else() {
        let mut world = World::new();
        let a = world.spawn_agent("a", None, None);
        let ghost = AgentId(99);
        // both unknown: `from` is reported (checked first)
        assert_eq!(
            world.pay(ghost, ghost, Metal::Gold, Money::new(5)),
            Err(WorldError::UnknownAgent(ghost))
        );
        // validation precedes the zero no-op (spec edge: rejecting phantom
        // ids is the point of this method)
        assert_eq!(
            world.pay(ghost, a, Metal::Gold, Money::ZERO),
            Err(WorldError::UnknownAgent(ghost))
        );
        // `to` is validated too — the phantom-account defence
        assert_eq!(
            world.pay(a, ghost, Metal::Gold, Money::new(5)),
            Err(WorldError::UnknownAgent(ghost))
        );
        // nothing moved on any Err
        assert_eq!(world.accounts.total_money(Metal::Gold), Money::ZERO);
    }

    #[test]
    fn pay_surfaces_insufficient_funds_unchanged() {
        let mut world = World::new();
        let a = world.spawn_agent("a", None, None);
        let b = world.spawn_agent("b", None, None);
        world.accounts.mint(a, Metal::Gold, Money::new(10));
        assert_eq!(
            world.pay(a, b, Metal::Gold, Money::new(20)),
            Err(WorldError::Money(MoneyError::InsufficientFunds))
        );
        // §8.5 atomic — books untouched
        assert_eq!(world.accounts.balance_of(a, Metal::Gold), Money::new(10));
        assert_eq!(world.accounts.balance_of(b, Metal::Gold), Money::ZERO);
    }

    #[test]
    fn pay_allows_reserved_ids_both_ends() {
        let mut world = World::new();
        world
            .accounts
            .mint(world.mint_id, Metal::Gold, Money::new(50));
        world
            .pay(
                world.mint_id,
                world.external_id,
                Metal::Gold,
                Money::new(20),
            )
            .unwrap();
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Gold),
            Money::new(20)
        );
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
        assert_eq!(
            world.vacate_home(ghost),
            Err(WorldError::UnknownAgent(ghost))
        );
    }

    #[test]
    fn assign_workplace_writes_workplace_and_role_together() {
        let mut world = World::new();
        let h1 = world.add_house("1 Mill Lane", vec![]);
        let h2 = world.add_house("2 Kiln Row", vec![]);
        let a = world.spawn_agent("a", None, None);
        world.assign_workplace(a, h1, Role::Labourer).unwrap();
        assert_eq!(world.agent(a).unwrap().workplace, Some(h1));
        assert_eq!(world.agent(a).unwrap().employed_role, Some(Role::Labourer));
        // re-assigning moves BOTH fields — never a stale role at a new job
        world.assign_workplace(a, h2, Role::Engineer).unwrap();
        assert_eq!(world.agent(a).unwrap().workplace, Some(h2));
        assert_eq!(world.agent(a).unwrap().employed_role, Some(Role::Engineer));
    }

    #[test]
    fn assign_workplace_checks_agent_then_house() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        let a = world.spawn_agent("a", None, None);
        let ghost_agent = AgentId(99);
        let ghost_house = HouseId(99);
        assert_eq!(
            world.assign_workplace(ghost_agent, house, Role::Labourer),
            Err(WorldError::UnknownAgent(ghost_agent))
        );
        assert_eq!(
            world.assign_workplace(a, ghost_house, Role::Labourer),
            Err(WorldError::UnknownHouse(ghost_house))
        );
        assert_eq!(
            world.assign_workplace(ghost_agent, ghost_house, Role::Labourer),
            Err(WorldError::UnknownAgent(ghost_agent))
        );
        // nothing changed on any Err — neither field
        assert_eq!(world.agent(a).unwrap().workplace, None);
        assert_eq!(world.agent(a).unwrap().employed_role, None);
    }

    #[test]
    fn vacate_workplace_clears_both_and_tolerates_unemployed() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        let a = world.spawn_agent("a", None, Some(house));
        world.agent_mut(a).unwrap().employed_role = Some(Role::Labourer);
        world.vacate_workplace(a).unwrap();
        assert_eq!(world.agent(a).unwrap().workplace, None);
        // the role goes with the job (employed_role-implies-workplace)
        assert_eq!(world.agent(a).unwrap().employed_role, None);
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

    #[test]
    fn agent_new_fields_default_to_none_and_zero() {
        let mut world = World::new();
        let a = world.spawn_agent("a", None, None);
        let agent = world.agent(a).unwrap();
        assert_eq!(agent.specialization, None);
        assert_eq!(agent.employed_role, None);
        assert_eq!(agent.education, 0);
    }

    #[test]
    fn create_business_allocates_id_from_agent_counter() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        let person = world.spawn_agent("a", None, None);
        let business = world
            .create_business(house, person, Good::Food, Money::new(1), HashMap::new())
            .unwrap();
        // shared counter: distinct from reserved ids and every spawned agent
        assert_ne!(business, world.mint_id);
        assert_ne!(business, world.external_id);
        assert_ne!(business, person);
        // account-only id: no Agent struct behind it (like Mint/External)
        assert!(world.agent(business).is_none());
        // the counter moved on — later spawns can't collide either
        let later = world.spawn_agent("b", None, None);
        assert_ne!(later, business);
    }

    #[test]
    fn create_business_rejects_unknown_house() {
        let mut world = World::new();
        let owner = world.spawn_agent("owner", None, None);
        let ghost = HouseId(99);
        assert_eq!(
            world.create_business(ghost, owner, Good::Food, Money::new(1), HashMap::new()),
            Err(WorldError::UnknownHouse(ghost))
        );
    }

    #[test]
    fn create_business_rejects_duplicate() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        let owner = world.spawn_agent("owner", None, None);
        let first = world
            .create_business(house, owner, Good::Food, Money::new(1), HashMap::new())
            .unwrap();
        assert_eq!(
            world.create_business(house, owner, Good::Food, Money::new(1), HashMap::new()),
            Err(WorldError::BusinessAlreadyExists(house))
        );
        // Err changed nothing: the original business is untouched
        assert_eq!(
            world.house(house).unwrap().business.as_ref().unwrap().id,
            first
        );
    }

    #[test]
    fn create_business_validates_owner_first() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        let person = world.spawn_agent("a", None, None);
        let shop = world.add_house("Shop", vec![]);
        let business = world
            .create_business(shop, person, Good::Food, Money::new(1), HashMap::new())
            .unwrap();
        let ghost = AgentId(99);
        let ghost_house = HouseId(99);
        // every non-agent owner refuses: ghosts, reserved ids, business ids
        for refused in [ghost, world.mint_id, world.external_id, business] {
            assert_eq!(
                world.create_business(house, refused, Good::Food, Money::new(1), HashMap::new()),
                Err(WorldError::UnknownAgent(refused))
            );
        }
        // owner is checked FIRST: both bad ⇒ the owner is reported
        assert_eq!(
            world.create_business(
                ghost_house,
                ghost,
                Good::Food,
                Money::new(1),
                HashMap::new()
            ),
            Err(WorldError::UnknownAgent(ghost))
        );
        // nothing changed on any Err
        assert!(world.house(house).unwrap().business.is_none());
    }

    #[test]
    fn businesses_yields_only_hosting_houses_in_houses_order() {
        let mut world = World::new();
        let h1 = world.add_house("1 Mill Lane", vec![]);
        world.add_house("2 Kiln Row", vec![]); // hosts nothing — must be skipped
        let h3 = world.add_house("3 Forge Way", vec![]);
        let owner = world.spawn_agent("owner", None, None);
        // created out of order to prove iteration follows `houses`, not creation
        let b3 = world
            .create_business(h3, owner, Good::Food, Money::new(1), HashMap::new())
            .unwrap();
        let b1 = world
            .create_business(h1, owner, Good::Food, Money::new(1), HashMap::new())
            .unwrap();
        let found: Vec<_> = world
            .businesses()
            .map(|(house, business)| (house.id, business.id))
            .collect();
        assert_eq!(found, vec![(h1, b1), (h3, b3)]);
    }

    #[test]
    fn pay_accepts_business_ids() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        let worker = world.spawn_agent("a", None, None);
        let business = world
            .create_business(house, worker, Good::Food, Money::new(1), HashMap::new())
            .unwrap();
        world.accounts.mint(business, Metal::Gold, Money::new(100)); // sanctioned test funding
        // business → agent: the future pay_wages direction
        world
            .pay(business, worker, Metal::Gold, Money::new(40))
            .unwrap();
        // agent → business: the future goods-purchase direction
        world
            .pay(worker, business, Metal::Gold, Money::new(10))
            .unwrap();
        assert_eq!(
            world.accounts.balance_of(business, Metal::Gold),
            Money::new(70)
        );
        assert_eq!(
            world.accounts.balance_of(worker, Metal::Gold),
            Money::new(30)
        );
        // unknown non-business ids are still refused
        let ghost = AgentId(99);
        assert_eq!(
            world.pay(ghost, worker, Metal::Gold, Money::new(1)),
            Err(WorldError::UnknownAgent(ghost))
        );
        world.accounts.audit();
    }

    #[test]
    fn create_business_sets_product_price_and_empty_stock() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        let owner = world.spawn_agent("owner", None, None);
        world
            .create_business(
                house,
                owner,
                Good::Entertainment,
                Money::new(2),
                HashMap::new(),
            )
            .unwrap();
        let business = world.house(house).unwrap().business.as_ref().unwrap();
        assert_eq!(business.product, Good::Entertainment);
        assert_eq!(business.price, Money::new(2));
        assert_eq!(business.stock, 0);
    }

    #[test]
    fn spawned_agents_start_with_an_empty_inventory() {
        let mut world = World::new();
        let a = world.spawn_agent("a", None, None);
        assert!(world.agent(a).unwrap().inventory.is_empty());
    }

    // --- Pack-4 emigration command ---

    #[test]
    fn remove_agent_sweeps_every_metal_no_orphans() {
        let mut world = World::new();
        let leaver = world.spawn_agent("leaver", None, None);
        world.accounts.mint(leaver, Metal::Gold, Money::new(12));
        world.accounts.mint(leaver, Metal::Silver, Money::new(3));
        world.accounts.mint(leaver, Metal::Copper, Money::new(5));
        world.remove_agent(leaver).unwrap();
        // per-account, per-metal — the spec insists totals-identical is
        // vacuously true and the audit cannot see an orphan
        for metal in Metal::ALL {
            assert_eq!(world.accounts.balance_of(leaver, metal), Money::ZERO);
        }
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Gold),
            Money::new(12)
        );
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Silver),
            Money::new(3)
        );
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Copper),
            Money::new(5)
        );
        assert!(world.agent(leaver).is_none());
        world.accounts.audit();
    }

    #[test]
    fn close_business_refuses_unknown_and_business_less_houses_changing_nothing() {
        let mut world = World::new();
        let empty = world.add_house("Empty", vec![]);
        assert_eq!(
            world.close_business(HouseId(99)),
            Err(WorldError::UnknownHouse(HouseId(99)))
        );
        assert_eq!(
            world.close_business(empty),
            Err(WorldError::NoBusinessHere(empty))
        );
        // house checked first, then the business — and nothing moved
        assert!(world.house(empty).unwrap().business.is_none());
        world.accounts.audit();
    }

    #[test]
    fn close_business_settles_ascending_writes_off_and_sweeps_no_orphans() {
        let mut world = World::new();
        let shop = world.add_house("Shop", vec![]);
        // The owner is ALSO a creditor — the case the ClosureReceipt
        // exists for: settlement (step 1) and residual (step 3) share one
        // wallet, so deltas around the whole command cannot attribute
        // either flow. Spawned first, so they hold the lowest id and
        // ascending order is observable against the later creditors.
        let owner = world.spawn_agent("owner", None, Some(shop));
        let hand = world.spawn_agent("hand", None, Some(shop));
        let ghost = world.spawn_agent("ghost", None, None); // former worker
        let business = world
            .create_business(shop, owner, Good::Food, Money::new(1), HashMap::new())
            .unwrap();
        // 90g coffer against 150g of debt: the first two creditors are
        // paid in full, the third takes what is left, nothing is negative.
        world.accounts.mint(business, Metal::Gold, Money::new(90));
        world.accounts.mint(business, Metal::Silver, Money::new(7));
        // ...and copper stays zero, so the receipt's visible-zero shows.
        {
            let ledger = &mut world
                .house_mut(shop)
                .unwrap()
                .business
                .as_mut()
                .unwrap()
                .owed_to;
            ledger.insert(owner, Money::new(40));
            ledger.insert(hand, Money::new(30));
            ledger.insert(ghost, Money::new(80));
            // a zero-amount entry: pay_wages inserts unconditionally, and
            // no entry may keep naming a dead firm (pack-4 precedent)
            let stranger = AgentId(404);
            ledger.insert(stranger, Money::ZERO);
        }

        let receipt = world.close_business(shop).unwrap();

        // 1. ascending AgentId, positive amounts only, min-bounded
        assert_eq!(
            receipt.settlements,
            vec![
                (owner, Money::new(40)),
                (hand, Money::new(30)),
                (ghost, Money::new(20)), // 90 − 40 − 30: the rest written off
            ],
            "settlements must be ascending AgentId with the remainder written off"
        );
        // 2. everyone on the premises is off it, role cleared with it
        assert_eq!(receipt.laid_off, vec![owner, hand]);
        for worker in [owner, hand] {
            let person = world.agent(worker).unwrap();
            assert_eq!(person.workplace, None);
            assert_eq!(person.employed_role, None);
        }
        // 3. every metal swept, zeros listed (D3 visible-zeros)
        assert_eq!(
            receipt.residual,
            vec![
                (Metal::Gold, Money::ZERO), // the coffer went to creditors
                (Metal::Silver, Money::new(7)),
                (Metal::Copper, Money::ZERO),
            ]
        );
        // The owner's wallet is settlement PLUS proceeds — never
        // "proceeds = owner delta", which conflates the two flows.
        assert_eq!(
            world.accounts.balance_of(owner, Metal::Gold),
            Money::new(40)
        );
        assert_eq!(
            world.accounts.balance_of(owner, Metal::Silver),
            Money::new(7)
        );
        // 4. detached last, and NO ORPHAN on the dead id, per account —
        // the totals-only audit cannot see a conservation-legal orphan
        assert!(world.house(shop).unwrap().business.is_none());
        for metal in Metal::ALL {
            assert_eq!(
                world.accounts.balance_of(business, metal),
                Money::ZERO,
                "orphan balance parked on the closed {business:?}"
            );
        }
        // the freed house passes the immigration vacancy predicate
        // verbatim: closure manufactures landing pads
        assert!(world.occupants_of(shop).is_empty());
        let newcomer = world.immigrate("Mara".to_string(), shop).unwrap();
        assert_eq!(world.agent(newcomer).unwrap().home, Some(shop));
        // and the retired id refuses money for good
        assert_eq!(
            world.pay(world.external_id, business, Metal::Gold, Money::new(1)),
            Err(WorldError::UnknownAgent(business))
        );
        world.accounts.audit();
    }

    #[test]
    fn remove_agent_settles_min_coffer_owed_then_writes_off() {
        // coffer covers the debt: full settlement rides to External with
        // the sweep
        let mut world = World::new();
        let shop = world.add_house("Shop", vec![]);
        let owner = world.spawn_agent("shopkeep", None, None);
        let business = world
            .create_business(shop, owner, Good::Food, Money::new(1), HashMap::new())
            .unwrap();
        let leaver = world.spawn_agent("leaver", None, None);
        world.accounts.mint(business, Metal::Gold, Money::new(100));
        world
            .house_mut(shop)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .owed_to
            .insert(leaver, Money::new(30));
        world.remove_agent(leaver).unwrap();
        assert_eq!(
            world.accounts.balance_of(business, Metal::Gold),
            Money::new(70)
        );
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Gold),
            Money::new(30)
        );
        assert!(
            world
                .house(shop)
                .unwrap()
                .business
                .as_ref()
                .unwrap()
                .owed_to
                .is_empty()
        );
        world.accounts.audit();

        // coffer short of the debt: partial settlement, remainder written
        // off — the entry leaves the ledger regardless
        let mut world = World::new();
        let shop = world.add_house("Shop", vec![]);
        let owner = world.spawn_agent("shopkeep", None, None);
        let business = world
            .create_business(shop, owner, Good::Food, Money::new(1), HashMap::new())
            .unwrap();
        let leaver = world.spawn_agent("leaver", None, None);
        world.accounts.mint(business, Metal::Gold, Money::new(20));
        world
            .house_mut(shop)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .owed_to
            .insert(leaver, Money::new(50));
        world.remove_agent(leaver).unwrap();
        assert_eq!(
            world.accounts.balance_of(business, Metal::Gold),
            Money::ZERO
        );
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Gold),
            Money::new(20)
        );
        assert!(
            world
                .house(shop)
                .unwrap()
                .business
                .as_ref()
                .unwrap()
                .owed_to
                .is_empty()
        );

        // an empty coffer settles nothing but still writes the debt off
        let mut world = World::new();
        let shop = world.add_house("Shop", vec![]);
        let owner = world.spawn_agent("shopkeep", None, None);
        world
            .create_business(shop, owner, Good::Food, Money::new(1), HashMap::new())
            .unwrap();
        let leaver = world.spawn_agent("leaver", None, None);
        world
            .house_mut(shop)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .owed_to
            .insert(leaver, Money::new(50));
        world.remove_agent(leaver).unwrap();
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Gold),
            Money::ZERO
        );
        assert!(
            world
                .house(shop)
                .unwrap()
                .business
                .as_ref()
                .unwrap()
                .owed_to
                .is_empty()
        );
        world.accounts.audit();

        // even a ZERO-amount entry (a zero-wage slot's bookkeeping
        // residue) must not survive naming a removed id
        let mut world = World::new();
        let shop = world.add_house("Shop", vec![]);
        let owner = world.spawn_agent("shopkeep", None, None);
        world
            .create_business(shop, owner, Good::Food, Money::new(1), HashMap::new())
            .unwrap();
        let leaver = world.spawn_agent("leaver", None, None);
        world
            .house_mut(shop)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .owed_to
            .insert(leaver, Money::ZERO);
        world.remove_agent(leaver).unwrap();
        assert!(
            world
                .house(shop)
                .unwrap()
                .business
                .as_ref()
                .unwrap()
                .owed_to
                .is_empty()
        );
    }

    #[test]
    fn remove_agent_rejects_non_agents_with_nothing_changed() {
        let mut world = World::new();
        let house = world.add_house("Shop", vec![]);
        let owner = world.spawn_agent("shopkeep", None, None);
        let business = world
            .create_business(house, owner, Good::Food, Money::new(1), HashMap::new())
            .unwrap();
        world.accounts.mint(business, Metal::Gold, Money::new(50));
        let ghost = AgentId(99);
        for refused in [ghost, world.mint_id, world.external_id, business] {
            assert_eq!(
                world.remove_agent(refused),
                Err(WorldError::UnknownAgent(refused))
            );
        }
        // nothing changed: the business account still stands untouched
        assert_eq!(
            world.accounts.balance_of(business, Metal::Gold),
            Money::new(50)
        );
        world.accounts.audit();
    }

    #[test]
    fn remove_agent_strips_ownership_and_derived_links() {
        let mut world = World::new();
        let leaver = world.spawn_agent("leaver", None, None);
        let other = world.spawn_agent("other", None, None);
        let home = world.add_house("1 Mill Lane", vec![leaver, other]);
        let shop = world.add_house("Shop", vec![leaver]);
        world
            .create_business(shop, other, Good::Food, Money::new(1), HashMap::new())
            .unwrap();
        world.assign_home(leaver, home).unwrap();
        world
            .assign_workplace(leaver, shop, Role::Labourer)
            .unwrap();
        world.remove_agent(leaver).unwrap();
        // ownership is stored, so the strip is explicit — no dangling ids
        assert_eq!(world.house(home).unwrap().owners, vec![other]);
        assert!(world.house(shop).unwrap().owners.is_empty());
        // occupancy and staffing are derived, so removal IS the update
        assert!(world.occupants_of(home).is_empty());
        assert!(world.employees_of(shop).is_empty());
    }

    #[test]
    fn immigrate_is_money_free_and_wraps_the_constructor() {
        let mut world = World::new();
        let home = world.add_house("5 Weir Cottage", vec![]);
        let newcomer = world.immigrate("Mara".to_string(), home).unwrap();
        let agent = world.agent(newcomer).unwrap();
        assert_eq!(agent.name, "Mara");
        assert_eq!(agent.home, Some(home));
        assert_eq!(agent.workplace, None);
        assert_eq!(agent.employed_role, None);
        assert_eq!(agent.hunger, 0);
        assert!(agent.inventory.is_empty());
        // money-free: zero on every metal, nothing minted anywhere
        for metal in Metal::ALL {
            assert_eq!(world.accounts.balance_of(newcomer, metal), Money::ZERO);
            assert_eq!(world.accounts.total_money(metal), Money::ZERO);
        }
        assert_eq!(world.arrivals, 1);
        world.accounts.audit();
    }

    #[test]
    fn immigrate_rejects_non_vacant_and_unknown_houses() {
        let mut world = World::new();
        let occupied = world.add_house("1 Mill Lane", vec![]);
        let resident = world.spawn_agent("resident", Some(occupied), None);
        let shop = world.add_house("Shop", vec![]);
        world
            .create_business(shop, resident, Good::Food, Money::new(1), HashMap::new())
            .unwrap();
        let ghost = HouseId(99);
        let before = world.agents.len();
        assert_eq!(
            world.immigrate("Mara".to_string(), occupied),
            Err(WorldError::HouseNotVacant(occupied))
        );
        assert_eq!(
            world.immigrate("Mara".to_string(), shop),
            Err(WorldError::HouseNotVacant(shop))
        );
        assert_eq!(
            world.immigrate("Mara".to_string(), ghost),
            Err(WorldError::UnknownHouse(ghost))
        );
        // nothing changed on any Err
        assert_eq!(world.agents.len(), before);
        assert_eq!(world.arrivals, 0);
    }

    #[test]
    fn employees_of_is_derived_in_ascending_id_order() {
        let mut world = World::new();
        let shop = world.add_house("1 Mill Lane", vec![]);
        let idle_house = world.add_house("2 Kiln Row", vec![]);
        let first = world.spawn_agent("first", None, Some(shop));
        let second = world.spawn_agent("second", None, Some(shop));
        assert!(first.0 < second.0, "ids ascend in spawn order");
        // the full staff, ascending — the contended-decide order contract
        assert_eq!(world.employees_of(shop), vec![first, second]);
        // unstaffed and unknown houses: empty
        assert!(world.employees_of(idle_house).is_empty());
        assert!(world.employees_of(HouseId(99)).is_empty());
        // derived, never stored: quitting is visible immediately
        world.vacate_workplace(first).unwrap();
        assert_eq!(world.employees_of(shop), vec![second]);
    }
}
