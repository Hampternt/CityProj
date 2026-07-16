# World Agent Commands (07-03) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the `World` command layer from `docs/superpowers/specs/2026-07-03-world-agent-commands-design.md`: `WorldError`, `pay`, `assign_home`/`vacate_home`, `assign_workplace`/`vacate_workplace`.

**Architecture:** All new code lives in `src/world.rs` — one new error enum and one new `impl World` block (the "command layer"). Every command validates ids FIRST, then either rewrites one agent-side link field or forwards to the §8.2 money chokepoint (`Accounts::transfer`). `Err` always means nothing changed.

**Tech Stack:** Rust edition 2024, std only. No new dependencies.

## Global Constraints

- Spec contracts are verbatim — never rename `WorldError`, `pay`, `assign_home`, `vacate_home`, `assign_workplace`, `vacate_workplace`, or their error variants (`UnknownAgent`, `UnknownHouse`, `Money`).
- §8.2: commands never touch `Accounts.balances` — `pay` only validates and forwards to `Accounts::transfer`.
- §8.5 surfaces unchanged: insufficient funds → `Err(WorldError::Money(MoneyError::InsufficientFunds))`.
- Layer property: validate before any state change; `Err` means the world is untouched.
- Link rule: occupancy stays derived; assign/vacate only rewrite `Agent.home` / `Agent.workplace`.
- This ships **single-metal**, deliberately: the multi-metal spec's Migration impact section owns adding a `Metal` param to `pay` later. Do not add one now.
- Tests fund accounts via `world.accounts.mint(...)` directly — sanctioned by the spec (no `grant` wrapper exists on purpose).
- Verification is `cargo check && cargo clippy && cargo test` with real output. No warnings expected at any commit point.

---

### Task 1: `WorldError` + `World::pay`

**Files:**
- Modify: `src/world.rs` (imports at ~line 8, new enum + impl block after the existing `impl World`, tests in the existing `mod tests`)

**Interfaces:**
- Consumes: `Accounts::transfer(from: AgentId, to: AgentId, amount: Money) -> Result<(), MoneyError>` (src/money.rs:146); `World::agent(&self, id: AgentId) -> Option<&Agent>`; `world.mint_id` / `world.external_id`.
- Produces (later tasks and the refactor plan rely on these, exact):
  - `pub enum WorldError { UnknownAgent(AgentId), UnknownHouse(HouseId), Money(MoneyError) }` deriving `Debug, PartialEq, Eq`
  - `pub fn pay(&mut self, from: AgentId, to: AgentId, amount: Money) -> Result<(), WorldError>`
  - `fn is_known_account(&self, id: AgentId) -> bool` (private helper; the refactor plan's Amendment-14 task extends it with business ids)

- [ ] **Step 1: Write the failing tests**

In `src/world.rs`, replace the test module's `use crate::money::Money;` with:

```rust
    use crate::money::{Money, MoneyError};
```

Then add to `mod tests`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test pay_ 2>&1 | tail -20`
Expected: compile error — `cannot find type 'WorldError' in this scope` and/or `no method named 'pay' found for struct 'World'`.

- [ ] **Step 3: Write the implementation**

In `src/world.rs`, change the money import (line 8) to:

```rust
use crate::money::{Accounts, Money, MoneyError};
```

After the existing `impl World { ... }` block (after line 111), add:

```rust
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
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test pay_ 2>&1 | tail -10`
Expected: `test result: ok. 4 passed` (filter matches the 4 new tests).

- [ ] **Step 5: Full verification**

Run: `cargo check && cargo clippy && cargo test 2>&1 | tail -5`
Expected: no warnings, no errors; all tests pass (the crate's 16 pre-existing tests plus the 4 new `pay_` tests).

- [ ] **Step 6: Commit**

```bash
git add src/world.rs
git commit -m "feat: WorldError + World::pay — validated money movement (07-03)"
```

---

### Task 2: `assign_home` / `vacate_home`

**Files:**
- Modify: `src/world.rs` (methods inside the command-layer `impl World` block from Task 1; tests in `mod tests`)

**Interfaces:**
- Consumes: `WorldError` (Task 1); `World::agent` / `World::agent_mut` / `World::house` (existing).
- Produces:
  - `pub fn assign_home(&mut self, agent: AgentId, house: HouseId) -> Result<(), WorldError>`
  - `pub fn vacate_home(&mut self, agent: AgentId) -> Result<(), WorldError>`

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `src/world.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test _home 2>&1 | tail -10`
Expected: compile error — `no method named 'assign_home' found for struct 'World'`.

- [ ] **Step 3: Write the implementation**

Add inside the command-layer `impl World` block (after `pay`):

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test _home 2>&1 | tail -5`
Expected: `test result: ok. 3 passed` (plus the pre-existing `occupants_derived_from_agent_homes` may match the filter — 4 passed is also correct; zero failures is the requirement).

- [ ] **Step 5: Full verification**

Run: `cargo check && cargo clippy && cargo test 2>&1 | tail -5`
Expected: no warnings; all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/world.rs
git commit -m "feat: assign_home/vacate_home commands (07-03)"
```

---

### Task 3: `assign_workplace` / `vacate_workplace`

Identical contract to Task 2 on the `workplace` field (spec states them verbatim-identical) — but the code is repeated in full here; do not improvise a shared helper (two near-identical 6-line methods beat one macro or closure indirection at this size).

**Files:**
- Modify: `src/world.rs` (methods inside the command-layer `impl World` block; tests in `mod tests`)

**Interfaces:**
- Consumes: `WorldError` (Task 1); existing lookups.
- Produces:
  - `pub fn assign_workplace(&mut self, agent: AgentId, house: HouseId) -> Result<(), WorldError>`
  - `pub fn vacate_workplace(&mut self, agent: AgentId) -> Result<(), WorldError>`

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `src/world.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test _workplace 2>&1 | tail -10`
Expected: compile error — `no method named 'assign_workplace' found for struct 'World'`.

- [ ] **Step 3: Write the implementation**

Add inside the command-layer `impl World` block (after `vacate_home`):

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test _workplace 2>&1 | tail -5`
Expected: `test result: ok. 3 passed`.

- [ ] **Step 5: Full verification**

Run: `cargo check && cargo clippy && cargo test 2>&1 | tail -5`
Expected: no warnings; every test in the crate passes.

- [ ] **Step 6: Update CLAUDE.md's code-state section**

In `CLAUDE.md`, change the `src/world.rs` bullet:

```markdown
- `src/world.rs` — `World`: agents + houses + accounts; reserves the Mint and
  External account ids; occupancy is derived, never stored; the 07-03 command
  layer (`pay`, assign/vacate home/workplace) validates ids before forwarding
  to the §8.2 chokepoint.
```

And replace the "Next up" list (the numbered 1./2. items, keeping the "Pending approval" paragraph as-is) with:

```markdown
Next up (spec approved, not yet implemented):
[`docs/superpowers/specs/2026-07-13-housing-agent-business-refactor-design.md`](docs/superpowers/specs/2026-07-13-housing-agent-business-refactor-design.md)
— `Business`/`Role`/`RoleSlot` structs and agent specialization fields,
struct-only, no behavior. Amends the implemented 07-03 `pay` to recognize
business ids (its Amendment 14).
```

- [ ] **Step 7: Commit**

```bash
git add src/world.rs CLAUDE.md
git commit -m "feat: assign_workplace/vacate_workplace commands — completes 07-03"
```

---

## Self-review record

- Spec coverage: `WorldError` (Task 1), `pay` + all three contract edges (Task 1), `assign_home`/`vacate_home` (Task 2), `assign_workplace`/`vacate_workplace` verbatim-identical (Task 3), `From<MoneyError>` mechanical (Task 1). Spec's "internal id-existence checks" = `is_known_account` + existing lookups. No gaps.
- Single-metal confirmed everywhere; no `Metal` anywhere in this plan.
- Type consistency: `WorldError` variants and method signatures match the spec Contracts verbatim.
