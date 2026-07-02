# Money & Game-Loop Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** [docs/superpowers/specs/2026-07-02-money-gameloop-skeleton-design.md](../specs/2026-07-02-money-gameloop-skeleton-design.md) — its Contracts section is the source of truth for every signature below; they are copied verbatim, never rename.

**Goal:** Build the sim's core skeleton — trusted money core, person-agents + houses, and the fixed 9-phase tick with the conservation audit live from tick 0 — steppable and inspectable interactively.

**Architecture:** Five new top-level modules (`money`, `agent`, `housing`, `world`, `sim`); `engine/game_loop.rs` slims to a pure I/O shell that calls `sim::tick`; the placeholder factory scaffold (`engine/game_state.rs`) is deleted. Money moves only through `Accounts::{transfer, mint, burn}`; `audit()` panics on any conservation violation and runs unconditionally last every tick.

**Tech Stack:** Rust, edition 2024, no dependencies. Binary crate (no `lib.rs`), so all tests are in-module `#[cfg(test)] mod tests` blocks — an external `tests/` directory cannot import from a binary crate.

## Global Constraints

- **§8.1 Integer money only:** `Money(u64)`, smallest unit, never a float.
- **§8.1 Checked arithmetic:** all `Money` math uses `checked_add`/`checked_sub`. Overflow policy: `.expect("money overflow")` — an explicit panic path, never silent wraparound. (Overflow is unreachable in practice; panicking keeps `transfer`'s error type exactly as the contract specifies: `MoneyError::InsufficientFunds` only.)
- **§8.2 Single chokepoint:** `Accounts.balances` is a private field; the only public mutators are `transfer`, `mint`, `burn`. The sole exception is `set_balance_for_test`, which is `#[cfg(test)]` — sanctioned by the spec so tests can prove the audit panics; never compiled into the sim.
- **§8.3 Audit every tick:** `audit()` panics on imbalance; it is the last call in `sim::tick` with no path around it. Never soften to a `Result`.
- **§8.4 Mint/burn sole creation/destruction**, logged to `total_minted`/`total_burned`.
- **§8.5 No overdraft:** `transfer` and `burn` return `Err(MoneyError::InsufficientFunds)` with **no partial application** — no state change on error.
- **Link rule:** agent→house (`home`, `workplace`) and house→`owners` are stored; house→occupants is always derived by scanning agents. Never store both directions.
- **Reserved ids:** `AgentId(0)` = Mint, `AgentId(1)` = External, reserved by `World::new`; agent spawning starts at id 2 and never reuses them. They are plain accounts — no structs, no behavior.
- **Warnings policy:** `money.rs` carries a file-level `#![allow(dead_code)]` (the full §8.2 API ships before any mechanic calls it; tests exercise it until then — remove the allow once mechanics use the movers). Tasks 1–3 will show `dead_code` warnings for not-yet-wired items in other files; they disappear in Task 4. Warnings are acceptable mid-plan; compile *errors* are not.
- **Commits:** conventional style (`feat:`, `chore:`, `docs:`) matching the repo's history.

## File structure (end state)

```
src/
  main.rs              — mod declarations + call into engine::game_loop::run()
  money.rs             — Money, MoneyError, Accounts (the trusted core; §8)
  agent.rs             — AgentId, Agent (a person)
  housing.rs           — HouseId, House (a place)
  world.rs             — World: agents + houses + accounts + reserved ids
  sim.rs               — tick(): the 9 fixed phases + Intent + decide/apply template
  engine/
    mod.rs             — pub mod game_loop; (game_state removed)
    game_loop.rs       — interactive shell only: clear, render, read command
```

---

### Task 1: Money core (`money.rs`) + id/data types

**Files:**
- Create: `src/money.rs`
- Create: `src/agent.rs`
- Create: `src/housing.rs`
- Modify: `src/main.rs` (add mod declarations)
- Test: in-module `#[cfg(test)] mod tests` at the bottom of `src/money.rs`

**Interfaces:**
- Consumes: nothing (first task).
- Produces (later tasks rely on these exact names):
  - `agent::AgentId(pub u32)` — `Copy, Eq, Hash`; `agent::Agent { pub id: AgentId, pub name: String, pub home: Option<HouseId>, pub workplace: Option<HouseId> }`
  - `housing::HouseId(pub u32)` — `Copy, Eq, Hash`; `housing::House { pub id: HouseId, pub address: String, pub owners: Vec<AgentId> }`
  - `money::Money` — `Money::new(u64)`, `Money::ZERO`, `Display`, `Ord`
  - `money::MoneyError::InsufficientFunds`
  - `money::Accounts` — `new()`, `transfer(from, to, amount) -> Result<(), MoneyError>`, `mint(to, amount)`, `burn(from, amount) -> Result<(), MoneyError>`, `audit()`, `balance_of(id) -> Money`, `total_money() -> Money`, `total_minted() -> Money`, `total_burned() -> Money`, `#[cfg(test)] set_balance_for_test(id, amount)`

- [ ] **Step 1: Create the data-type files (no tests needed — pure data, no behavior)**

`src/agent.rs`:

```rust
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
```

`src/housing.rs`:

```rust
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
```

`src/main.rs` becomes:

```rust
mod agent;
mod engine;
mod housing;
mod money;

fn main() {
    engine::game_loop::run();
}
```

(`mod money;` won't compile until Step 2 creates the file — that's fine, Steps 1–2 are written together before the first `cargo` run.)

- [ ] **Step 2: Write the failing tests for mint + reads (TDD cycle 1)**

Create `src/money.rs` with the skeleton *types only* plus the first tests. The method bodies don't exist yet, so the test fails to compile — that is the failing state for this cycle.

```rust
//! The trusted money core (§8). All money movement goes through `Accounts`:
//! `transfer`, `mint`, and `burn` are the only mutators (§8.2), and `audit`
//! panics the sim on any conservation violation (§8.3).

// The full §8.2 API ships before any mechanic calls it; tests exercise it
// until the first mechanic does. Remove once the movers have real callers.
#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt;

use crate::agent::AgentId;

/// An amount of money in the smallest indivisible unit (§8.1 — never a
/// float). All arithmetic is checked; overflow panics explicitly rather
/// than wrapping silently.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Money(u64);

impl Money {
    pub const ZERO: Money = Money(0);

    pub const fn new(amount: u64) -> Self {
        Money(amount)
    }

    fn plus(self, other: Money) -> Money {
        Money(self.0.checked_add(other.0).expect("money overflow"))
    }

    fn minus(self, other: Money) -> Money {
        Money(self.0.checked_sub(other.0).expect("money underflow"))
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MoneyError {
    InsufficientFunds,
}

/// The single store of all balances (§8.2). `balances` is private; the only
/// public mutators are `transfer`, `mint`, and `burn`.
#[derive(Debug, Default)]
pub struct Accounts {
    balances: HashMap<AgentId, Money>,
    total_minted: Money,
    total_burned: Money,
}

impl Accounts {
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a() -> AgentId {
        AgentId(10)
    }

    fn b() -> AgentId {
        AgentId(11)
    }

    #[test]
    fn mint_credits_and_logs() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Money::new(100));
        assert_eq!(accounts.balance_of(a()), Money::new(100));
        assert_eq!(accounts.total_minted(), Money::new(100));
        assert_eq!(accounts.total_money(), Money::new(100));
        accounts.audit();
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test mint_credits_and_logs`
Expected: FAIL to compile with `error[E0599]: no method named 'mint' found` (and the same for `balance_of`, `total_minted`, `total_money`, `audit`).

- [ ] **Step 4: Implement mint + the read queries + audit**

Add inside `impl Accounts` (after `new`):

```rust
    /// Read-only. Unknown id reads as zero — accounts are created implicitly
    /// at first credit.
    pub fn balance_of(&self, id: AgentId) -> Money {
        self.balances.get(&id).copied().unwrap_or(Money::ZERO)
    }

    /// Sum of ALL balances, including External.
    pub fn total_money(&self) -> Money {
        self.balances.values().fold(Money::ZERO, |sum, &b| sum.plus(b))
    }

    pub fn total_minted(&self) -> Money {
        self.total_minted
    }

    pub fn total_burned(&self) -> Money {
        self.total_burned
    }

    /// §8.4: the ONLY way money is created. Gold-reserve cap deferred — spec
    /// amendment needed when the mint job arrives.
    pub fn mint(&mut self, to: AgentId, amount: Money) {
        let balance = self.balance_of(to);
        self.balances.insert(to, balance.plus(amount));
        self.total_minted = self.total_minted.plus(amount);
    }

    /// §8.3: asserts conservation, PANICS on imbalance — by design, never
    /// softened to a `Result`. Initial supply is zero (no genesis), so
    /// circulating money must equal minted − burned exactly.
    pub fn audit(&self) {
        let expected = self
            .total_minted
            .0
            .checked_sub(self.total_burned.0)
            .expect("audit failed: total_burned exceeds total_minted (§8.3)");
        assert_eq!(
            self.total_money(),
            Money(expected),
            "conservation audit failed: circulating money != minted - burned (§8.3)"
        );
    }
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test mint_credits_and_logs`
Expected: PASS (`test money::tests::mint_credits_and_logs ... ok`, `1 passed`). Warnings about unused items in `agent.rs`/`housing.rs` are expected until Task 4.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs src/agent.rs src/housing.rs src/money.rs
git commit -m "feat: add Money core with mint, read queries, and conservation audit"
```

- [ ] **Step 7: Write the failing tests for transfer (TDD cycle 2)**

Add to `mod tests` in `src/money.rs`. These are the spec's acceptance tests verbatim — Given/Then is binding.

```rust
    #[test]
    fn transfer_moves_exact_amount() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Money::new(100));
        accounts.transfer(a(), b(), Money::new(30)).unwrap();
        assert_eq!(accounts.balance_of(a()), Money::new(70));
        assert_eq!(accounts.balance_of(b()), Money::new(30));
        assert_eq!(accounts.total_money(), Money::new(100));
    }

    #[test]
    fn transfer_insufficient_funds_is_atomic() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Money::new(10));
        let result = accounts.transfer(a(), b(), Money::new(20));
        assert_eq!(result, Err(MoneyError::InsufficientFunds));
        // no partial application — nothing changed
        assert_eq!(accounts.balance_of(a()), Money::new(10));
        assert_eq!(accounts.balance_of(b()), Money::ZERO);
    }

    #[test]
    fn transfer_zero_is_noop() {
        let mut accounts = Accounts::new();
        accounts.transfer(a(), b(), Money::ZERO).unwrap();
        assert_eq!(accounts.total_money(), Money::ZERO);
        // creates no account entry (tests may touch private fields — same module)
        assert!(accounts.balances.is_empty());
    }

    #[test]
    fn transfer_to_self() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Money::new(50));
        accounts.transfer(a(), a(), Money::new(20)).unwrap();
        assert_eq!(accounts.balance_of(a()), Money::new(50));
    }
```

- [ ] **Step 8: Run the tests to verify they fail**

Run: `cargo test transfer`
Expected: FAIL to compile with `error[E0599]: no method named 'transfer' found`.

- [ ] **Step 9: Implement transfer**

Add inside `impl Accounts`:

```rust
    /// §8.2/§8.5: moves money between accounts, or errs with NO state change.
    pub fn transfer(
        &mut self,
        from: AgentId,
        to: AgentId,
        amount: Money,
    ) -> Result<(), MoneyError> {
        if amount == Money::ZERO {
            return Ok(()); // no-op by contract: creates no account entry
        }
        let from_balance = self.balance_of(from);
        if from_balance < amount {
            return Err(MoneyError::InsufficientFunds); // §8.5 — nothing applied
        }
        if from == to {
            return Ok(()); // funds verified; debit + credit would cancel out
        }
        self.balances.insert(from, from_balance.minus(amount));
        let to_balance = self.balance_of(to);
        self.balances.insert(to, to_balance.plus(amount));
        Ok(())
    }
```

- [ ] **Step 10: Run the tests to verify they pass**

Run: `cargo test transfer`
Expected: PASS, `4 passed`.

- [ ] **Step 11: Commit**

```bash
git add src/money.rs
git commit -m "feat: add atomic no-overdraft transfer to Accounts"
```

- [ ] **Step 12: Write the failing tests for burn (TDD cycle 3)**

Add to `mod tests`:

```rust
    #[test]
    fn burn_debits_and_logs() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Money::new(100));
        accounts.burn(a(), Money::new(40)).unwrap();
        assert_eq!(accounts.balance_of(a()), Money::new(60));
        assert_eq!(accounts.total_burned(), Money::new(40));
        accounts.audit();
    }

    #[test]
    fn burn_insufficient_funds_is_atomic() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Money::new(10));
        assert_eq!(
            accounts.burn(a(), Money::new(20)),
            Err(MoneyError::InsufficientFunds)
        );
        assert_eq!(accounts.balance_of(a()), Money::new(10));
        assert_eq!(accounts.total_burned(), Money::ZERO);
    }
```

- [ ] **Step 13: Run the tests to verify they fail**

Run: `cargo test burn`
Expected: FAIL to compile with `error[E0599]: no method named 'burn' found`.

- [ ] **Step 14: Implement burn**

Add inside `impl Accounts`:

```rust
    /// §8.4: the ONLY way money is destroyed. Same atomicity rules as
    /// transfer (§8.5): errs with no state change, zero is a no-op.
    pub fn burn(&mut self, from: AgentId, amount: Money) -> Result<(), MoneyError> {
        if amount == Money::ZERO {
            return Ok(());
        }
        let balance = self.balance_of(from);
        if balance < amount {
            return Err(MoneyError::InsufficientFunds); // §8.5 — nothing applied
        }
        self.balances.insert(from, balance.minus(amount));
        self.total_burned = self.total_burned.plus(amount);
        Ok(())
    }
```

- [ ] **Step 15: Run the tests to verify they pass**

Run: `cargo test burn`
Expected: PASS, `2 passed`.

- [ ] **Step 16: Commit**

```bash
git add src/money.rs
git commit -m "feat: add burn as the sole money sink in Accounts"
```

- [ ] **Step 17: Write the failing audit-hardening tests (TDD cycle 4)**

Add to `mod tests`:

```rust
    #[test]
    fn audit_passes_after_op_sequence() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Money::new(100));
        accounts.audit();
        accounts.transfer(a(), b(), Money::new(30)).unwrap();
        accounts.audit();
        // failed ops must leave the books balanced too
        assert!(accounts.transfer(b(), a(), Money::new(999)).is_err());
        accounts.audit();
        accounts.burn(a(), Money::new(20)).unwrap();
        accounts.audit();
        assert!(accounts.burn(b(), Money::new(999)).is_err());
        accounts.audit();
        accounts.mint(b(), Money::new(5));
        accounts.audit();
    }

    #[test]
    #[should_panic]
    fn audit_panics_on_imbalance() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Money::new(100));
        accounts.set_balance_for_test(a(), Money::new(150));
        accounts.audit();
    }

    #[test]
    fn total_money_includes_external() {
        // External is just an id from Accounts' perspective; 1 is its
        // reserved value (World reserves it properly in Task 2).
        let external = AgentId(1);
        let mut accounts = Accounts::new();
        accounts.mint(a(), Money::new(100));
        accounts.transfer(a(), external, Money::new(60)).unwrap();
        // out of circulation but still counted by the audit
        assert_eq!(accounts.total_money(), Money::new(100));
        accounts.audit();
    }
```

- [ ] **Step 18: Run the tests to verify they fail**

Run: `cargo test audit`
Expected: `audit_panics_on_imbalance` FAILS to compile with `error[E0599]: no method named 'set_balance_for_test' found`.

- [ ] **Step 19: Implement the test-only balance mutator**

Add inside `impl Accounts`:

```rust
    /// The SANCTIONED §8.2 exception: exists solely so tests can force an
    /// imbalance and prove the audit panics. Never compiled into the sim.
    #[cfg(test)]
    pub fn set_balance_for_test(&mut self, id: AgentId, amount: Money) {
        self.balances.insert(id, amount);
    }
```

- [ ] **Step 20: Run the full money suite to verify all pass**

Run: `cargo test money::`
Expected: PASS, `10 passed` (all ten acceptance tests from the spec).

- [ ] **Step 21: Commit**

```bash
git add src/money.rs
git commit -m "feat: harden conservation audit with sequence, panic, and External tests"
```

---

### Task 2: World (`world.rs`) — reserved ids, spawning, derived occupancy

**Files:**
- Create: `src/world.rs`
- Modify: `src/main.rs` (add `mod world;`)
- Test: in-module `#[cfg(test)] mod tests` at the bottom of `src/world.rs`

**Interfaces:**
- Consumes: `agent::{Agent, AgentId}`, `housing::{House, HouseId}`, `money::Accounts` (Task 1 signatures).
- Produces (later tasks rely on these exact names):
  - `world::World { pub agents: Vec<Agent>, pub houses: Vec<House>, pub accounts: Accounts, pub mint_id: AgentId, pub external_id: AgentId }` (+ private id counters)
  - `World::new() -> World` (and `Default`)
  - `World::spawn_agent(&mut self, name: &str, home: Option<HouseId>, workplace: Option<HouseId>) -> AgentId`
  - `World::add_house(&mut self, address: &str, owners: Vec<AgentId>) -> HouseId`
  - `World::occupants_of(&self, house: HouseId) -> Vec<AgentId>` (contract-bound: always derived, never stored)
  - `World::agent_mut(&mut self, id: AgentId) -> Option<&mut Agent>`

- [ ] **Step 1: Write the failing tests**

Create `src/world.rs` with only the doc header, imports, and tests (types come in Step 3), and add `mod world;` to `src/main.rs` (alphabetical: after `mod money;`):

```rust
//! The world: all agents, houses, and the money accounts in one value.
//! Mint and External are reserved as plain accounts here — no structs,
//! no behavior — so phase contracts can name them and ids never get
//! reassigned.

use crate::agent::{Agent, AgentId};
use crate::housing::{House, HouseId};
use crate::money::Accounts;

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
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test world::`
Expected: FAIL to compile with `error[E0433]: failed to resolve: use of undeclared type 'World'`.

- [ ] **Step 3: Implement World**

Insert between the imports and `mod tests`:

```rust
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

    pub fn agent_mut(&mut self, id: AgentId) -> Option<&mut Agent> {
        self.agents.iter_mut().find(|agent| agent.id == id)
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test world::`
Expected: PASS, `3 passed` (13 total across the crate if you run bare `cargo test`).

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/world.rs
git commit -m "feat: add World with reserved Mint/External ids and derived occupancy"
```

---

### Task 3: Tick loop (`sim.rs`) — 9 fixed phases + decide/apply template

**Files:**
- Create: `src/sim.rs`
- Modify: `src/main.rs` (add `mod sim;`)
- Test: in-module `#[cfg(test)] mod tests` at the bottom of `src/sim.rs`

**Interfaces:**
- Consumes: `world::World` (with `pub accounts`), `Accounts::audit()`, `#[cfg(test)] Accounts::set_balance_for_test`, `agent::Agent`.
- Produces (Task 4 relies on): `sim::tick(world: &mut World)`. Also `sim::Intent` (empty enum) — the seam future mechanics extend; nothing else calls it yet.

- [ ] **Step 1: Write the failing tests**

Create `src/sim.rs` with the doc header, imports, and tests only, and add `mod sim;` to `src/main.rs` (alphabetical: after `mod money;`, before `mod world;`):

```rust
//! The fixed 9-phase tick (parent doc §6). A new mechanic lands INSIDE its
//! phase; adding or reordering phases requires amending the spec's phase
//! contract table. The conservation audit (§8.3) is unconditionally last.

use crate::agent::Agent;
use crate::world::World;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentId;
    use crate::money::Money;

    #[test]
    fn n_ticks_run_clean() {
        let mut world = World::new();
        for _ in 0..100 {
            tick(&mut world);
        }
        // nothing mints yet, so the money supply must still be zero
        assert_eq!(world.accounts.total_money(), Money::ZERO);
    }

    #[test]
    #[should_panic]
    fn tick_runs_audit_last() {
        let mut world = World::new();
        // corrupt the books via the sanctioned test hook; if any path
        // through tick skipped the audit, this would NOT panic
        world.accounts.set_balance_for_test(AgentId(7), Money::new(999));
        tick(&mut world);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test sim::`
Expected: FAIL to compile with `error[E0425]: cannot find function 'tick'`.

- [ ] **Step 3: Implement tick, the eight phase stubs, and the decide/apply template**

Insert between the imports and `mod tests`:

```rust
/// What an agent wants to do, decided in a pure pass and executed in an
/// apply pass. Mechanics add variants; the skeleton has none.
pub enum Intent {}

/// Runs one tick: phases 1–8 in exactly the spec table's order, then the
/// conservation audit — unconditionally last, no early return skips it.
pub fn tick(world: &mut World) {
    labor_market(world);
    produce(world);
    pay_wages(world);
    goods_market(world);
    consume(world);
    invest(world);
    sinks(world);
    mint_phase(world);
    // Phase 9: audit (§8.3) — read-only, never gains behavior.
    world.accounts.audit();
}

/// Phase 1: match hires, adjust wage offers. Money ops allowed: none.
fn labor_market(_world: &mut World) {
    // TODO: firms + labor market land here.
}

/// Phase 2: labor + inputs → goods. Money ops allowed: none.
fn produce(_world: &mut World) {
    // TODO: firm production lands here.
}

/// Phase 3: firms pay agreed wages. Money ops allowed: transfer only.
fn pay_wages(_world: &mut World) {
    // TODO: wages land here (needs firms).
}

/// Phase 4: agents buy goods, prices adjust. Money ops allowed: transfer
/// only. This phase is the WORKED decide→apply TEMPLATE — every behavior
/// phase copies this two-pass shape.
fn goods_market(world: &mut World) {
    // Decide (pure): each agent reads the tick-start snapshot and returns
    // what it WANTS to do. No `&mut` anywhere — unit-testable and free of
    // iteration-order effects.
    let intents: Vec<Intent> = world.agents.iter().flat_map(decide_goods).collect();

    // Apply: the ONLY place this phase moves money. Unaffordable intents
    // fail cleanly (transfer errs) — wanting is unconstrained, paying is not.
    for intent in intents {
        apply_goods_intent(world, intent);
    }
}

/// TODO: needs-driven purchasing lands here. Stays pure.
fn decide_goods(_agent: &Agent) -> Vec<Intent> {
    Vec::new()
}

fn apply_goods_intent(_world: &mut World, intent: Intent) {
    // Exhaustive over zero variants: adding an Intent variant forces every
    // apply fn to handle it at compile time.
    match intent {}
}

/// Phase 5: goods consumed toward needs. Money ops allowed: none.
fn consume(_world: &mut World) {
    // TODO: needs fulfillment lands here.
}

/// Phase 6: expand capacity / take profit. Money ops allowed: transfer only.
fn invest(_world: &mut World) {
    // TODO: firm investment lands here.
}

/// Phase 7: degradation, imports. Money ops allowed: burn, transfer→External.
fn sinks(_world: &mut World) {
    // TODO: demurrage and external purchases land here.
}

/// Phase 8: new money from reserve. Money ops allowed: mint only.
fn mint_phase(_world: &mut World) {
    // TODO: the mint job (and later the gold-backing cap) lands here.
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test sim::`
Expected: PASS, `2 passed` (the `should_panic` test passes *because* it panics).

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/sim.rs
git commit -m "feat: add 9-phase tick with audit last and decide/apply template"
```

---

### Task 4: Interactive shell — slim `game_loop.rs`, delete the factory scaffold

**Files:**
- Modify: `src/engine/game_loop.rs` (full rewrite — old content is replaced entirely)
- Modify: `src/engine/mod.rs` (drop `pub mod game_state;`)
- Delete: `src/engine/game_state.rs`
- Modify: `src/world.rs` (add three read helpers)
- Modify: `CLAUDE.md` ("Current code state vs. target layout" section — required by that section's own instruction)
- Test: no unit tests (interactive I/O); verified by piping input through `cargo run` plus the full gate

**Interfaces:**
- Consumes: `sim::tick`, `World::{new, spawn_agent, add_house, agent_mut, occupants_of, accounts, agents, houses, mint_id, external_id}`, `Accounts::{balance_of, total_money, total_minted, total_burned}`, `Money: Display`.
- Produces: `engine::game_loop::run()` (unchanged entry point, called from `main`); new `World` helpers `agent(&self, id: AgentId) -> Option<&Agent>`, `agent_by_name(&self, name: &str) -> Option<&Agent>`, `house(&self, id: HouseId) -> Option<&House>`.

- [ ] **Step 1: Add the three read helpers to `src/world.rs`**

Inside `impl World`, directly above `agent_mut`:

```rust
    pub fn agent(&self, id: AgentId) -> Option<&Agent> {
        self.agents.iter().find(|agent| agent.id == id)
    }

    pub fn agent_by_name(&self, name: &str) -> Option<&Agent> {
        self.agents.iter().find(|agent| agent.name == name)
    }

    pub fn house(&self, id: HouseId) -> Option<&House> {
        self.houses.iter().find(|house| house.id == id)
    }
```

- [ ] **Step 2: Rewrite `src/engine/game_loop.rs`**

Replace the entire file with:

```rust
//! The interactive shell: clear, render, read a command, tick. All
//! simulation behavior lives in `sim::tick` — this file only draws frames
//! and reads input. Loop mechanics are unchanged: Enter advances, q quits;
//! typing an agent's name inspects it.

use std::io::{self, Write};

use crate::housing::HouseId;
use crate::sim;
use crate::world::World;

/// One parsed line of user input at the tick prompt.
enum Command {
    Advance,
    Quit,
    Inspect(String),
}

pub fn run() {
    let mut world = template_world();
    let mut tick_count: u64 = 0;

    loop {
        // Redraw the frame in place so the display doesn't scroll downward.
        clear_screen();
        render(&world, tick_count);

        match read_command(tick_count) {
            Command::Quit => break,
            Command::Advance => {
                sim::tick(&mut world);
                tick_count += 1;
            }
            Command::Inspect(name) => inspect(&world, &name),
        }
    }
}

/// A hand-seeded template world to step through: three people, two houses.
/// Nothing mints money yet, so every balance stays 0 until a faucet exists
/// (the mint job — money only ever enters through earned paths, no genesis).
fn template_world() -> World {
    let mut world = World::new();
    let alice = world.spawn_agent("alice", None, None);
    let bob = world.spawn_agent("bob", None, None);
    let carol = world.spawn_agent("carol", None, None);
    let mill = world.add_house("1 Mill Lane", vec![alice]);
    let kiln = world.add_house("2 Kiln Row", vec![bob]);
    world.agent_mut(alice).expect("just spawned").home = Some(mill);
    world.agent_mut(bob).expect("just spawned").home = Some(kiln);
    world.agent_mut(carol).expect("just spawned").home = Some(kiln);
    world
}

/// Clears the terminal and parks the cursor at the top-left, so each frame
/// redraws in place instead of scrolling. `\x1b[2J` erases the screen and
/// `\x1b[H` homes the cursor; we flush so it lands before anything prints.
fn clear_screen() {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();
}

/// Draws one stable frame: the money summary, then houses, then agents.
fn render(world: &World, tick_count: u64) {
    println!("=== CityProj — tick {tick_count} ===");
    println!(
        "money: total={} minted={} burned={}",
        world.accounts.total_money(),
        world.accounts.total_minted(),
        world.accounts.total_burned(),
    );
    println!(
        "reserved: mint balance {} · external balance {}",
        world.accounts.balance_of(world.mint_id),
        world.accounts.balance_of(world.external_id),
    );

    println!("houses:");
    for house in &world.houses {
        let owners = names_of(world, &house.owners);
        let occupants = names_of(world, &world.occupants_of(house.id));
        println!(
            "  {} — owners: {} · occupants: {}",
            house.address,
            or_none(&owners),
            or_none(&occupants),
        );
    }

    println!("agents:");
    for agent in &world.agents {
        println!(
            "  {} — balance {} · home {}",
            agent.name,
            world.accounts.balance_of(agent.id),
            describe_house(world, agent.home),
        );
    }
}

/// Resolves a list of agent ids to their names (unknown ids are skipped).
fn names_of(world: &World, ids: &[crate::agent::AgentId]) -> Vec<String> {
    ids.iter()
        .filter_map(|id| world.agent(*id))
        .map(|agent| agent.name.clone())
        .collect()
}

fn or_none(names: &[String]) -> String {
    if names.is_empty() {
        "(none)".to_string()
    } else {
        names.join(", ")
    }
}

fn describe_house(world: &World, id: Option<HouseId>) -> String {
    id.and_then(|house_id| world.house(house_id))
        .map(|house| house.address.clone())
        .unwrap_or_else(|| "none".to_string())
}

/// Blocks until the user enters a command. EOF (e.g. Ctrl-D) and read
/// errors quit cleanly, same as before.
fn read_command(tick_count: u64) -> Command {
    print!("[tick {tick_count}] Enter = advance · <agent name> = inspect · q = quit > ");
    // stdout is line-buffered; flush so the prompt shows before we block.
    let _ = io::stdout().flush();

    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => Command::Quit,
        Ok(_) => match line.trim() {
            "" => Command::Advance,
            quit if quit.eq_ignore_ascii_case("q") => Command::Quit,
            name => Command::Inspect(name.to_string()),
        },
    }
}

/// Prints one agent's details, then waits for Enter so the next clear-screen
/// doesn't wipe them before they're read.
fn inspect(world: &World, name: &str) {
    match world.agent_by_name(name) {
        Some(agent) => {
            println!("{}:", agent.name);
            println!("  balance   {}", world.accounts.balance_of(agent.id));
            println!("  home      {}", describe_house(world, agent.home));
            println!("  workplace {}", describe_house(world, agent.workplace));
        }
        None => println!("no agent named '{name}'"),
    }
    print!("press Enter to continue... ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
}
```

- [ ] **Step 3: Delete the factory scaffold**

```bash
git rm src/engine/game_state.rs
```

And replace the whole of `src/engine/mod.rs` with:

```rust
//! The interactive front-end. Simulation logic lives in the top-level
//! modules (`sim`, `world`, `money`, …) — this layer only presents it.

pub mod game_loop;
```

- [ ] **Step 4: Run the full verification gate**

Run each and confirm:

```bash
cargo check    # expected: clean, NO dead_code warnings left outside money.rs
cargo clippy   # expected: no warnings
cargo test     # expected: 15 passed (10 money + 3 world + 2 sim), 0 failed
```

If `cargo check` still reports a dead-code warning, something rendered/used in this task was missed — fix the usage (or the code), don't add new `#[allow]`s.

- [ ] **Step 5: Smoke-test the interactive shell non-interactively**

Run: `printf '\n\nalice\n\nq\n' | cargo run`
Expected: exits 0; output contains `=== CityProj — tick 2 ===` (two Enters advanced two ticks), the `alice:` inspect block with `balance   0` and `home      1 Mill Lane`, and no panic — the audit ran clean on every tick.

Then run `cargo run` interactively yourself if a terminal is available: Enter advances, `bob` inspects, `q` quits.

- [ ] **Step 6: Update CLAUDE.md's "Current code state vs. target layout" section**

Replace that section's body (keep the heading) with:

```markdown
The economy skeleton from
[`docs/superpowers/specs/2026-07-02-money-gameloop-skeleton-design.md`](docs/superpowers/specs/2026-07-02-money-gameloop-skeleton-design.md)
is in place — that spec's ⟨REF⟩ sections are the standing contract for hooking
new mechanics into the loop and money:

- `src/money.rs` — `Money`, `MoneyError`, `Accounts` (the §8 trusted core:
  `transfer`/`mint`/`burn` movers, `audit` panics on imbalance).
- `src/agent.rs`, `src/housing.rs` — `Agent` (person) and `House` data types.
- `src/world.rs` — `World`: agents + houses + accounts; reserves the Mint and
  External account ids; occupancy is derived, never stored.
- `src/sim.rs` — `tick()`: the fixed 9-phase order, audit unconditionally
  last; `goods_market` holds the worked decide→apply template; `Intent` is
  the (empty) enum mechanics extend.
- `src/engine/game_loop.rs` — interactive shell only: Enter advances a tick,
  an agent name inspects it, q quits.

No economic behavior runs yet: phases 1–8 are TODO stubs, `Accounts` starts
empty, and nothing mints — money enters only through earned paths once the
mint job exists. Firms are deferred (TODO markers in `sim.rs`). If you change
structure, update this section.
```

- [ ] **Step 7: Commit**

```bash
git add src/engine/game_loop.rs src/engine/mod.rs src/world.rs CLAUDE.md
git commit -m "feat: slim game loop to interactive shell over World and 9-phase tick"
```

(The `git rm` from Step 3 is already staged; this commit includes the deletion.)

---

## Verification (whole plan)

- `cargo check` — clean (only `money.rs` carries its documented `#![allow(dead_code)]`).
- `cargo clippy` — no warnings.
- `cargo test` — **15 passed**: the spec's 10 money + 3 world + 2 sim acceptance tests, names matching the spec's list.
- `printf '\n\nalice\n\nq\n' | cargo run` — steps ticks, inspects, quits, no panic.

## Spec-coverage checklist (self-review, done at plan-writing time)

- Money contract (type, checked ops) → Task 1 Step 2. ✓
- transfer / mint / burn / audit / read-query contracts → Task 1 cycles 1–4, signatures verbatim. ✓
- All 10 money acceptance tests → Task 1. ✓
- occupants_of contract + 3 world tests + reserved ids → Task 2. ✓
- sim::tick contract (fixed order, audit last, no skip) + 2 sim tests → Task 3. ✓
- Intent empty enum + decide/apply worked template in one phase → Task 3 (`goods_market`). ✓
- Slimmed shell (clear/render/Enter/q), inspect ("agent → balance, home"), template world, factory scaffold removed → Task 4. ✓
- Firms TODO markers only, no genesis, no pricing, no needs — respected everywhere (phases are stubs; nothing mints). ✓
- CLAUDE.md structure section update (required by CLAUDE.md itself) → Task 4 Step 6. ✓
