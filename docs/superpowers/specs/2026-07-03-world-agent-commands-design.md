# World agent commands — Design Spec

## Design (decisions — approve before reading on)

A small command layer on `World` that tick phases, worldgen/setup, and the
interactive shell all reuse: `pay` for money movement between known parties,
and assign/vacate pairs for the two agent↔house links (`home`, `workplace`).
`World` is the only type that sees agents, houses, and accounts together, so
it is where cross-reference validation lives. The layer adds referential
integrity the money core deliberately lacks: `Accounts` creates accounts
implicitly at first credit, so a raw `transfer` to a typo'd id silently parks
money on a phantom account — `pay` rejects unknown ids instead. All money
still moves through the §8.2 chokepoint; these methods only validate and
forward.

- **Fits into:** @src/world.rs (all new methods + `WorldError`); callers are
  future phase mechanics in @src/sim.rs, worldgen, and
  @src/engine/game_loop.rs.
- **Invariants in play:** §8.2 single chokepoint (`pay` forwards to
  `Accounts::transfer`, never touches balances itself); §8.5 atomicity
  (validation happens before any state change, so an `Err` means nothing
  moved); link rule (occupancy stays derived — assign/vacate only rewrite the
  agent-side field).
- **Out of scope (YAGNI):** no `grant`/`take` wrappers around `mint`/`burn` —
  money enters only through earned paths via the mint phase, and a "give
  agent free money" helper is how that erodes (tests use `accounts.mint`
  directly, as they already do). No house capacity checks (house traits are
  undesigned). No ownership transfer. No firm-side validation of
  `assign_workplace` (any existing house qualifies until firms land).

## Contracts (the hard reference the plan is written from)

### Unit: WorldError  (world.rs)

Signature: `pub enum WorldError { UnknownAgent(AgentId), UnknownHouse(HouseId), Money(MoneyError) }`
Given:  any command method refuses
Then:   the variant names the first failed check; `Money` wraps the §8.5
        refusal unchanged. Derives `Debug, PartialEq, Eq` like `MoneyError`.
Error:  —
Refs:   @src/world.rs, @src/money.rs §8.5 atomic refusal

### Unit: World::pay  (world.rs)

Signature: `pub fn pay(&mut self, from: AgentId, to: AgentId, amount: Money) -> Result<(), WorldError>`
Given:  `from` and `to` are each a spawned agent OR a reserved account id
        (`mint_id`, `external_id`)
Then:   forwards to `Accounts::transfer` — same semantics (zero is a no-op,
        self-pay of verified funds is a no-op), books unchanged on error
Error:  unknown id -> `Err(WorldError::UnknownAgent(id))` (checked `from`
        then `to`, before touching accounts)   // defends against phantom accounts
        insufficient funds -> `Err(WorldError::Money(InsufficientFunds))`   // §8.5
Refs:   @src/world.rs, @src/money.rs, §8.2 single chokepoint

### Unit: World::assign_home  (world.rs)

Signature: `pub fn assign_home(&mut self, agent: AgentId, house: HouseId) -> Result<(), WorldError>`
Given:  `agent` is spawned (reserved ids are NOT agents), `house` exists
Then:   `agent.home = Some(house)`; re-assigning an already-housed agent
        moves them (derived occupancy follows — link rule)
Error:  unknown agent -> `Err(WorldError::UnknownAgent(agent))`;
        unknown house -> `Err(WorldError::UnknownHouse(house))`   // agent checked first
Refs:   @src/world.rs link rule (occupancy derived, never stored)

### Unit: World::vacate_home  (world.rs)

Signature: `pub fn vacate_home(&mut self, agent: AgentId) -> Result<(), WorldError>`
Given:  `agent` is spawned; housed or not
Then:   `agent.home = None`; already-homeless is an Ok no-op
Error:  unknown agent -> `Err(WorldError::UnknownAgent(agent))`
Refs:   @src/world.rs link rule

### Unit: World::assign_workplace / World::vacate_workplace  (world.rs)

Signature: same shapes as the home pair, writing `agent.workplace`
Given/Then/Error: identical to `assign_home` / `vacate_home` with the
        `workplace` field; no firm-side checks in v1 (firms deferred)
Refs:   @src/world.rs, sim.rs firm TODOs

### Mechanical / obvious (plan owns these — no contract needed)

Internal id-existence checks (`agent`/`house` lookups already exist);
`From<MoneyError> for WorldError` if convenient; test helpers. Trivial —
implement as obvious, do not invent behavior beyond the contracts.

--- APPROVAL GATE — do not write the plan or any code above this line without sign-off ---

## Open questions

None — mint/burn exclusion, no capacity checks, and no firm validation were
decided above.
