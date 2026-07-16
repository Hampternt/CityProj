# Housing, Business & Agent Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `docs/superpowers/specs/2026-07-13-housing-agent-business-refactor-design.md`: `Role`/`RoleSlot`/`Business` structs, three new `Agent` fields, `World::create_business` / `World::businesses`, and Amendment 14 (`pay` recognizes business ids). Struct-only — no hiring, no wage payment, no production.

**Architecture:** Two new leaf modules (`role.rs`, `business.rs`) with no behavior; field additions to `House`/`Agent`; `World` gains one constructor-style command and one read-only query. Business ids come from the same counter as agent ids but get NO `Agent` struct — they join Mint/External as account-only ids, which is why `pay`'s known-id check must learn about them (Amendment 14).

**Tech Stack:** Rust edition 2024, std only. No new dependencies.

**Prerequisite:** The 07-03 plan (`2026-07-16-world-agent-commands.md`) is fully executed — `WorldError`, `pay`, and `is_known_account` exist in `src/world.rs`. Do not start this plan before that one is merged.

## Global Constraints

- Spec contracts are verbatim — never rename `Role`, `RoleSlot`, `Business`, `create_business`, `businesses`, `BusinessAlreadyExists`, `specialization`, `employed_role`, `education`.
- §8.2: `Business` has NO balance field, ever. Its `id` keys `Accounts` like any agent id.
- `RoleSlot.wage` types against today's single-metal `Money` — the multi-metal migration pass revises it later. Do not add a `Metal` anywhere.
- `spawn_agent` keeps its 3-arg signature; the new `Agent` fields default (`None`/`None`/`0`).
- No validation connecting `employed_role` to a business's `roles` map — future command, out of scope.
- New modules carry `#![allow(dead_code)]` with a removal comment, same rationale/pattern as `src/money.rs:17-19`.
- Verification is `cargo check && cargo clippy && cargo test` with real output. No warnings expected at any commit point.

---

### Task 1: `Role` enum (`src/role.rs`)

**Files:**
- Create: `src/role.rs`
- Modify: `src/main.rs:34-39` (module list) and its module-map doc comment

**Interfaces:**
- Consumes: nothing.
- Produces (Tasks 2, 4, 5 rely on these, exact):
  - `pub enum Role { Engineer, Labourer }` deriving `Debug, Clone, Copy, PartialEq, Eq, Hash`
  - `pub const ALL: [Role; 2]` (associated const on `Role`)
  - `impl fmt::Display for Role` — lowercase names: `"engineer"`, `"labourer"`

- [ ] **Step 1: Write the failing test**

Create `src/role.rs`:

```rust
//! Job roles: the closed set of work an agent can be trained for or
//! employed in (refactor spec Amendment 12). Same closed-enum pattern as
//! multi-metal's `Metal`: add a variant + extend `ALL`, and the compiler
//! finds every match needing an update. Business archetypes are DATA
//! (combinations of these variants), never new types.

// Struct-only refactor: nothing reads roles yet. Remove once the labor
// market lands. Same rationale as money.rs's crate allow.
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_lists_every_variant_exactly_once() {
        let mut seen = std::collections::HashSet::new();
        for role in Role::ALL {
            assert!(seen.insert(role), "duplicate in Role::ALL: {role:?}");
        }
        assert_eq!(seen.len(), 2);
    }

    #[test]
    fn display_is_lowercase_for_the_shell() {
        assert_eq!(Role::Engineer.to_string(), "engineer");
        assert_eq!(Role::Labourer.to_string(), "labourer");
    }
}
```

In `src/main.rs`, add to the module list (alphabetical — between `money` and `sim`):

```rust
mod role;
```

And add to the module-map doc comment (after the `housing` line):

```rust
//! - [`role`] — [`role::Role`], the closed set of job roles.
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test role:: 2>&1 | tail -10`
Expected: compile error — `cannot find type 'Role' in this scope`.

- [ ] **Step 3: Write the implementation**

In `src/role.rs`, between the `#![allow(dead_code)]` line and `#[cfg(test)]`:

```rust
use std::fmt;

/// One kind of job. `Copy + Eq + Hash` so it keys `HashMap<Role, RoleSlot>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Engineer,
    Labourer,
}

impl Role {
    /// Every variant, hand-enumerated — zero-dep convention, same as
    /// `Metal::ALL`. Extend this when adding a variant.
    pub const ALL: [Role; 2] = [Role::Engineer, Role::Labourer];
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Role::Engineer => "engineer",
            Role::Labourer => "labourer",
        };
        write!(f, "{name}")
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test role:: 2>&1 | tail -5`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 5: Full verification, then commit**

Run: `cargo check && cargo clippy && cargo test 2>&1 | tail -5`
Expected: no warnings; all tests pass.

```bash
git add src/role.rs src/main.rs
git commit -m "feat: Role enum — closed job-role set (refactor spec)"
```

---

### Task 2: `RoleSlot` + `Business` (`src/business.rs`)

**Files:**
- Create: `src/business.rs`
- Modify: `src/main.rs` (module list + module-map doc line)

**Interfaces:**
- Consumes: `Role` (Task 1), `Money` (`src/money.rs`), `AgentId` (`src/agent.rs`).
- Produces (Tasks 3, 5, 6 rely on these, exact):
  - `pub struct RoleSlot { pub wage: Money, pub headcount: u32 }` deriving `Debug`
  - `pub struct Business { pub id: AgentId, pub roles: HashMap<Role, RoleSlot> }` deriving `Debug`

- [ ] **Step 1: Write the failing test**

Create `src/business.rs`:

```rust
//! Businesses: a house's capability to employ, attached by composition
//! (`House::business`, Amendment 10 — never a `BuildingKind` enum).
//! Money-wise a business is only an account id (§8.2): balances live in
//! `Accounts`, never here. Wages are per-role (Amendment 11), never flat.

// Struct-only refactor: nothing reads these yet. Remove once hiring or
// wage phases land. Same rationale as money.rs's crate allow.
#![allow(dead_code)]

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentId;

    #[test]
    fn business_roles_map_holds_distinct_wages_per_role() {
        let mut roles = HashMap::new();
        roles.insert(
            Role::Engineer,
            RoleSlot {
                wage: Money::new(12),
                headcount: 2,
            },
        );
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(7),
                headcount: 5,
            },
        );
        let business = Business {
            id: AgentId(42),
            roles,
        };
        assert_eq!(business.roles[&Role::Engineer].wage, Money::new(12));
        assert_eq!(business.roles[&Role::Engineer].headcount, 2);
        assert_eq!(business.roles[&Role::Labourer].wage, Money::new(7));
        assert_eq!(business.roles[&Role::Labourer].headcount, 5);
    }
}
```

In `src/main.rs`, add to the module list (alphabetical — between `agent` and `engine`):

```rust
mod business;
```

And add to the module-map doc comment (after the `agent` line):

```rust
//! - [`business`] — [`business::Business`], a house's employer capability;
//!   account-only money-wise (no balance field).
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test business_roles 2>&1 | tail -10`
Expected: compile error — `cannot find type 'RoleSlot'`/`'Business' in this scope`.

- [ ] **Step 3: Write the implementation**

In `src/business.rs`, between the `#![allow(dead_code)]` line and `#[cfg(test)]`:

```rust
use std::collections::HashMap;

use crate::agent::AgentId;
use crate::money::Money;
use crate::role::Role;

/// One role a business employs: the wage it offers and how many workers it
/// wants. No behavior reads this yet. `wage` types against today's
/// single-metal `Money`; the multi-metal migration pass revises it (single
/// metal vs. bundle is that spec's open question) — don't design around the
/// current type being final.
#[derive(Debug)]
pub struct RoleSlot {
    pub wage: Money,
    pub headcount: u32,
}

/// A business attached to a house via `House::business`. `id` keys
/// `Accounts` like any agent id but has NO `Agent` struct behind it —
/// account-only, same category as the reserved Mint/External ids. No
/// balance field here, ever (§8.2). `inputs`/`outputs` deliberately absent
/// — deferred to a future `goods.rs` spec.
#[derive(Debug)]
pub struct Business {
    /// Account key in [`Accounts`](crate::money::Accounts), allocated by
    /// `World::create_business` from the shared agent-id counter.
    pub id: AgentId,
    /// The roles this business employs — one wage/headcount per role
    /// (Amendment 11: role-differentiated, never a flat figure).
    pub roles: HashMap<Role, RoleSlot>,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test business_roles 2>&1 | tail -5`
Expected: `test result: ok. 1 passed`.

- [ ] **Step 5: Full verification, then commit**

Run: `cargo check && cargo clippy && cargo test 2>&1 | tail -5`
Expected: no warnings; all tests pass.

```bash
git add src/business.rs src/main.rs
git commit -m "feat: RoleSlot + Business structs (refactor spec)"
```

---

### Task 3: `House.business` field

**Files:**
- Modify: `src/housing.rs` (add field), `src/world.rs` (`add_house` literal gains `business: None`; test in `mod tests`)

**Interfaces:**
- Consumes: `Business` (Task 2).
- Produces: `House.business: Option<Business>` (public field — Tasks 5 and 6 read/write it).

- [ ] **Step 1: Write the failing test**

Add to `mod tests` in `src/world.rs`:

```rust
    #[test]
    fn houses_start_without_business() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        assert!(world.house(house).unwrap().business.is_none());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test houses_start 2>&1 | tail -10`
Expected: compile error — `no field 'business' on type '&House'`.

- [ ] **Step 3: Write the implementation**

In `src/housing.rs`, add below the `use crate::agent::AgentId;` line:

```rust
use crate::business::Business;
```

Add the field to `House` (after `owners`, before the traits TODO comment):

```rust
    /// The business operating here, if any (composition, Amendment 10 — a
    /// house either hosts one or doesn't; capability is data, not a type
    /// split). At most one per house (v1). Attach via
    /// `World::create_business`.
    pub business: Option<Business>,
```

In `src/world.rs`, `add_house`'s struct literal gains the field:

```rust
        self.houses.push(House {
            id,
            address: address.to_string(),
            owners,
            business: None,
        });
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test houses_start 2>&1 | tail -5`
Expected: `test result: ok. 1 passed`.

- [ ] **Step 5: Full verification, then commit**

Run: `cargo check && cargo clippy && cargo test 2>&1 | tail -5`
Expected: no warnings; all tests pass.

```bash
git add src/housing.rs src/world.rs
git commit -m "feat: House.business — composition slot for a business (Am. 10)"
```

---

### Task 4: `Agent`'s three new fields + doc amendments

**Files:**
- Modify: `src/agent.rs` (3 fields + `AgentId` doc amendment), `src/world.rs` (`spawn_agent` literal + `World.agents` doc amendment; test in `mod tests`)

**Interfaces:**
- Consumes: `Role` (Task 1).
- Produces: `Agent.specialization: Option<Role>`, `Agent.employed_role: Option<Role>`, `Agent.education: u8` (public fields).

- [ ] **Step 1: Write the failing test**

Add to `mod tests` in `src/world.rs`:

```rust
    #[test]
    fn agent_new_fields_default_to_none_and_zero() {
        let mut world = World::new();
        let a = world.spawn_agent("a", None, None);
        let agent = world.agent(a).unwrap();
        assert_eq!(agent.specialization, None);
        assert_eq!(agent.employed_role, None);
        assert_eq!(agent.education, 0);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test agent_new_fields 2>&1 | tail -10`
Expected: compile error — `no field 'specialization' on type '&Agent'`.

- [ ] **Step 3: Write the implementation**

In `src/agent.rs`, add below `use crate::housing::HouseId;`:

```rust
use crate::role::Role;
```

Amend the `AgentId` doc comment (business ids join the account-only
category — without this line the next reader "fixes" a business id lookup
returning `None`):

```rust
/// Identifies one agent. Also keys `Accounts` balances — including the
/// reserved Mint/External accounts AND business ids, which have accounts
/// but no `Agent` struct.
```

Add the three fields to `Agent` (after `workplace`):

```rust
    /// Role this agent is trained for; `None` = unspecialized/general
    /// labour. Independent of `employed_role` — agents can work off-spec.
    pub specialization: Option<Role>,
    /// Role currently filled at `workplace`. Stored, not derived — nothing
    /// else records it. Intended invariant, documented but NOT yet
    /// enforced: `employed_role.is_some()` implies `workplace.is_some()`;
    /// enforcement belongs to the future `assign_workplace` extension.
    pub employed_role: Option<Role>,
    /// Reserved skill scalar with no defined effect yet — range and meaning
    /// are decided by the first spec that reads it.
    pub education: u8,
```

In `src/world.rs`, amend the `agents` field doc:

```rust
    /// Every person in the node. Reserved ids (Mint, External) and business
    /// ids have NO entry here — they are accounts only.
    pub agents: Vec<Agent>,
```

And `spawn_agent`'s struct literal gains the defaults (signature unchanged — do NOT widen the constructor):

```rust
        self.agents.push(Agent {
            id,
            name: name.to_string(),
            home,
            workplace,
            specialization: None,
            employed_role: None,
            education: 0,
        });
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test agent_new_fields 2>&1 | tail -5`
Expected: `test result: ok. 1 passed`.

- [ ] **Step 5: Full verification, then commit**

Run: `cargo check && cargo clippy && cargo test 2>&1 | tail -5`
Expected: no warnings; all tests pass.

```bash
git add src/agent.rs src/world.rs
git commit -m "feat: agent skill state — specialization, employed_role, education (Am. 12)"
```

---

### Task 5: `World::create_business` + `BusinessAlreadyExists`

**Files:**
- Modify: `src/world.rs` (new `WorldError` variant; `house_mut` helper in the ORIGINAL `impl World` block next to `agent_mut`; `create_business` in the command-layer block; tests in `mod tests`)

**Interfaces:**
- Consumes: `Business`/`RoleSlot` (Task 2), `House.business` (Task 3), `WorldError` (07-03 plan), `next_agent_id` counter.
- Produces (Tasks 6, 7 rely on these, exact):
  - `WorldError::BusinessAlreadyExists(HouseId)` (additive variant)
  - `pub fn create_business(&mut self, house: HouseId, roles: HashMap<Role, RoleSlot>) -> Result<AgentId, WorldError>`
  - `pub fn house_mut(&mut self, id: HouseId) -> Option<&mut House>` (mechanical mirror of `agent_mut`)

- [ ] **Step 1: Write the failing tests**

In `src/world.rs`'s `mod tests`, add below the existing `use` lines:

```rust
    use std::collections::HashMap;
```

Then add:

```rust
    #[test]
    fn create_business_allocates_id_from_agent_counter() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        let person = world.spawn_agent("a", None, None);
        let business = world.create_business(house, HashMap::new()).unwrap();
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
        let ghost = HouseId(99);
        assert_eq!(
            world.create_business(ghost, HashMap::new()),
            Err(WorldError::UnknownHouse(ghost))
        );
    }

    #[test]
    fn create_business_rejects_duplicate() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        let first = world.create_business(house, HashMap::new()).unwrap();
        assert_eq!(
            world.create_business(house, HashMap::new()),
            Err(WorldError::BusinessAlreadyExists(house))
        );
        // Err changed nothing: the original business is untouched
        assert_eq!(
            world.house(house).unwrap().business.as_ref().unwrap().id,
            first
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test create_business 2>&1 | tail -10`
Expected: compile error — `no method named 'create_business' found for struct 'World'`.

- [ ] **Step 3: Write the implementation**

In `src/world.rs`, extend the imports:

```rust
use crate::business::{Business, RoleSlot};
use crate::role::Role;
use std::collections::HashMap;
```

Add the variant to `WorldError` (after `UnknownHouse`):

```rust
    /// The house already hosts a business — at most one per house (v1).
    BusinessAlreadyExists(HouseId),
```

Add `house_mut` to the ORIGINAL `impl World` block, directly after `agent_mut` (it's a general lookup, not a command):

```rust
    /// Mutable lookup by id — mirrors [`agent_mut`](World::agent_mut).
    pub fn house_mut(&mut self, id: HouseId) -> Option<&mut House> {
        self.houses.iter_mut().find(|house| house.id == id)
    }
```

Add `create_business` to the command-layer `impl World` block (after `vacate_workplace`):

```rust
    /// Attaches a new business to `house`, allocating its account id from
    /// the same counter as `spawn_agent` — never a reserved id, never
    /// reused, and NO `Agent` struct is created (business ids are
    /// account-only, like Mint/External). Validates before touching state:
    /// `Err` means nothing changed.
    pub fn create_business(
        &mut self,
        house: HouseId,
        roles: HashMap<Role, RoleSlot>,
    ) -> Result<AgentId, WorldError> {
        match self.house(house) {
            None => return Err(WorldError::UnknownHouse(house)),
            Some(existing) if existing.business.is_some() => {
                return Err(WorldError::BusinessAlreadyExists(house));
            }
            Some(_) => {}
        }
        let id = AgentId(self.next_agent_id);
        self.next_agent_id += 1;
        self.house_mut(house).expect("existence checked above").business =
            Some(Business { id, roles });
        Ok(id)
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test create_business 2>&1 | tail -5`
Expected: `test result: ok. 3 passed`.

- [ ] **Step 5: Full verification, then commit**

Run: `cargo check && cargo clippy && cargo test 2>&1 | tail -5`
Expected: no warnings; all tests pass.

```bash
git add src/world.rs
git commit -m "feat: World::create_business — account-only business ids from the agent counter"
```

---

### Task 6: `World::businesses` query

**Files:**
- Modify: `src/world.rs` (method in the command-layer `impl World` block; test in `mod tests`)

**Interfaces:**
- Consumes: `House.business` (Task 3), `create_business` (Task 5).
- Produces (Task 7 and every future phase rely on this, exact):
  - `pub fn businesses(&self) -> impl Iterator<Item = (&House, &Business)>`

- [ ] **Step 1: Write the failing test**

Add to `mod tests` in `src/world.rs`:

```rust
    #[test]
    fn businesses_yields_only_hosting_houses_in_houses_order() {
        let mut world = World::new();
        let h1 = world.add_house("1 Mill Lane", vec![]);
        world.add_house("2 Kiln Row", vec![]); // hosts nothing — must be skipped
        let h3 = world.add_house("3 Forge Way", vec![]);
        // created out of order to prove iteration follows `houses`, not creation
        let b3 = world.create_business(h3, HashMap::new()).unwrap();
        let b1 = world.create_business(h1, HashMap::new()).unwrap();
        let found: Vec<_> = world
            .businesses()
            .map(|(house, business)| (house.id, business.id))
            .collect();
        assert_eq!(found, vec![(h1, b1), (h3, b3)]);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test businesses_yields 2>&1 | tail -10`
Expected: compile error — `no method named 'businesses' found for struct 'World'`.

- [ ] **Step 3: Write the implementation**

Add to the command-layer `impl World` block (after `create_business`):

```rust
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test businesses_yields 2>&1 | tail -5`
Expected: `test result: ok. 1 passed`.

- [ ] **Step 5: Full verification, then commit**

Run: `cargo check && cargo clippy && cargo test 2>&1 | tail -5`
Expected: no warnings; all tests pass.

```bash
git add src/world.rs
git commit -m "feat: World::businesses — shared read-only query for future phases (Am. 13)"
```

---

### Task 7: Amendment 14 — `pay` recognizes business ids

**Files:**
- Modify: `src/world.rs` (`is_known_account` body; test in `mod tests`)

**Interfaces:**
- Consumes: `is_known_account` (07-03 plan), `businesses()` (Task 6), `pay` (07-03 plan).
- Produces: `pay` accepting business ids in both positions — the seam the future `pay_wages` phase depends on.

- [ ] **Step 1: Write the failing test**

Add to `mod tests` in `src/world.rs`:

```rust
    #[test]
    fn pay_accepts_business_ids() {
        let mut world = World::new();
        let house = world.add_house("1 Mill Lane", vec![]);
        let worker = world.spawn_agent("a", None, None);
        let business = world.create_business(house, HashMap::new()).unwrap();
        world.accounts.mint(business, Money::new(100)); // sanctioned test funding
        // business → agent: the future pay_wages direction
        world.pay(business, worker, Money::new(40)).unwrap();
        // agent → business: the future goods-purchase direction
        world.pay(worker, business, Money::new(10)).unwrap();
        assert_eq!(world.accounts.balance_of(business), Money::new(70));
        assert_eq!(world.accounts.balance_of(worker), Money::new(30));
        // unknown non-business ids are still refused
        let ghost = AgentId(99);
        assert_eq!(
            world.pay(ghost, worker, Money::new(1)),
            Err(WorldError::UnknownAgent(ghost))
        );
        world.accounts.audit();
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test pay_accepts_business 2>&1 | tail -10`
Expected: FAIL (compiles, but panics) — `called 'Result::unwrap()' on an 'Err' value: UnknownAgent(AgentId(3))` (the business id — 3, allocated after the worker's 2 — is not yet in `pay`'s known set).

- [ ] **Step 3: Write the implementation**

In `src/world.rs`, extend `is_known_account` (the known-id set per Amendment 14: spawned ∪ reserved ∪ business ids). Update its doc comment too:

```rust
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test pay_accepts_business 2>&1 | tail -5`
Expected: `test result: ok. 1 passed`.

- [ ] **Step 5: Full verification**

Run: `cargo check && cargo clippy && cargo test 2>&1 | tail -5`
Expected: no warnings; every test in the crate passes (including all pre-existing `pay_` tests — the widened set must not loosen the unknown-id rejections).

- [ ] **Step 6: Update CLAUDE.md's code-state section**

In `CLAUDE.md`'s module bullet list, add after the `src/agent.rs, src/housing.rs` bullet:

```markdown
- `src/role.rs`, `src/business.rs` — `Role` (closed job-role enum) and
  `Business`/`RoleSlot` (per-role wages, account-only money) — struct-only,
  no behavior reads them yet.
```

Extend the `src/world.rs` bullet with:

```markdown
  `create_business` allocates account-only business ids from the agent
  counter; `businesses()` is the shared phase query; `pay` recognizes
  business ids (refactor Am. 14).
```

Replace the "Next up" paragraph with:

```markdown
Next up: nothing approved. Pending approval (no plan until signed off):
[`docs/superpowers/specs/2026-07-12-multi-metal-money-design.md`](docs/superpowers/specs/2026-07-12-multi-metal-money-design.md)
— `Accounts` keyed by `(AgentId, Metal)`; revises shipped `money.rs`,
`World::pay`, and `RoleSlot.wage` at its listed migration points. After
that: a wage-payment/hiring behavior spec built on `World::businesses()`.
```

- [ ] **Step 7: Commit**

```bash
git add src/world.rs CLAUDE.md
git commit -m "feat: pay recognizes business ids (Am. 14) — completes struct refactor"
```

---

## Self-review record

- Spec coverage: `Role`+`ALL`+Display (Task 1), `RoleSlot`/`Business` (Task 2), `House.business` (Task 3), `Agent`'s 3 fields + both doc-comment amendments + 3-arg `spawn_agent` (Task 4), `create_business` with both error arms (Task 5), `businesses()` (Task 6), Amendment 14 + `pay_accepts_business_ids` (Task 7). All five spec acceptance tests appear verbatim-or-stronger. No gaps.
- Discovered helper: `house_mut` (Task 5) — mechanical mirror of `agent_mut`, single-caller, not load-bearing; stays plan-owned per spec policy.
- No `Metal`, no `Good`, no validation of `employed_role`, no `businesses_mut` — all confirmed absent per spec.
- Type consistency: `HashMap<Role, RoleSlot>` and every signature matches Tasks 1–2 and the spec Contracts verbatim.
