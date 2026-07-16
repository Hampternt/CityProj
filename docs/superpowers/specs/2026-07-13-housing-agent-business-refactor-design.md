# Housing, Business & Agent Refactor — Design Spec

**Status:** approved in conversation 2026-07-16 (design); spec under review ·
**Parent:** [2026-07-02-money-gameloop-skeleton-design.md](2026-07-02-money-gameloop-skeleton-design.md) (parent: [2026-06-20-economy-sim-design.md](2026-06-20-economy-sim-design.md))
**Supersedes:** 2026-07-12-business-wages-alpha-design.md — scope replaced, not deferred (Amendment 11). That spec is NOT in the repo (historical, superseded before check-in); everything from it that survives is restated in full here, so nothing below depends on reading it.
**Depends on:** [2026-07-03-world-agent-commands-design.md](2026-07-03-world-agent-commands-design.md) (`World::pay`, `assign_workplace`, `WorldError`) — implement first. This spec amends its `pay` contract (Amendment 14).
**Partially blocked on:** [2026-07-12-multi-metal-money-design.md](2026-07-12-multi-metal-money-design.md) (draft, pending approval) — only `RoleSlot.wage`'s final type needs it. `Role`, `Agent`'s new fields, and `Business` itself have no `Money` dependency; `wage` types against today's single-metal `Money` and is revised by multi-metal's migration pass (single metal vs. bundle is that spec's open question).

## Design (decisions — approve before reading on)

Refactor `House`/`Agent` to carry the data shape a future market economy needs — job roles, per-role wages, agent specialization/education — without wiring up any behavior. No hiring, no wage payment, no production. Struct-only pass; the payment/production logic from the superseded wages-alpha spec becomes future work built on top of this.

- **Fits into:** new `src/role.rs`; new `src/business.rs`; `src/housing.rs` (add field); `src/agent.rs` (add 3 fields + doc amendment, below); `src/world.rs` (`create_business`, `businesses`, + doc amendment, below).
- **Doc-comment ripple:** `agent.rs` (`AgentId` doc) and `world.rs` (`World.agents` doc) both state that only the reserved Mint/External accounts exist without an `Agent` struct. Business ids become the third member of that category — both comments must say so, or the next reader "fixes" a business id lookup returning `None`.
- **Invariants:** §8.2 (`Business` has no balance field — `AgentId` account like any agent).
- **Out of scope / decided out:**
  - All wage-payment/mint/alpha-sale logic (fully dropped, Amendment 11).
  - `inputs`/`outputs` on `Business` — **decided (2026-07-16): deferred to a future `goods.rs` spec.** No `Good` enum is stubbed here; a dead type with zero readers invites guessed semantics, and the closed-enum pattern makes adding it later a compile-guided change.
  - Hiring/matching logic that fills `employed_role` (`labor_market` phase stays a stub).
  - Validating `employed_role` against a business's `roles` map (no command layer does this yet — see Open questions).
  - What `education` numerically does (reserved field, no formula).

### Amendments to the parent design doc
*(continues 07-02's 1–3 and multi-metal's 7–9. Amendments 4–6 belonged to the superseded wages-alpha spec — historical, not in repo; what survives is restated in full below, and the numbers are never reused.)*

10. **Housing/workplace capability stays composition** (`House.business: Option<Business>`), never a `BuildingKind` enum. (Restates the superseded spec's decision in full — a house either hosts a business or doesn't; capability is data on the house, not a type split.)
11. **Wages are role-differentiated, not flat.** `Business` has no single `wage`/`revenue_per_tick` — the superseded spec's flat alpha-revenue stand-in is dropped entirely, not carried forward. Each business has `roles: HashMap<Role, RoleSlot>`, one wage per role it employs.
12. **Agents gain skill state**: `specialization` (trained-for role), `employed_role` (role currently filled — stored, not derived, same as `workplace`; nothing else records this), `education` (reserved scalar, no defined effect yet).
13. **Tick integration follows the existing 07-02 phase table — no per-entity-type resolve phases.** A business's behavior spans several phases with different money-permission contracts (hiring in `labor_market` allows no money ops; paying wages in `pay_wages` allows `transfer` only) — one `resolve_business` phase would blur that. Future phases pull in relevant entities via a shared query (`World::businesses`, below), not a new phase.
14. **`World::pay` (07-03) must recognize business ids.** 07-03's `pay` contract predates business ids: its known-id set is "spawned agent OR reserved account id", and this spec mints ids that are neither — as written, `pay(business_id, worker, wage)` fails validation and `pay_wages` can never move business money. Amended Given: `from`/`to` are each a spawned agent, a reserved account id (`mint_id`, `external_id`), **or the id of an existing business** (checkable via `World::businesses`). Everything else in the `pay` contract (validation order, error variants, no-op edges) is unchanged; an unknown id still errors `UnknownAgent`. If 07-03 is implemented before this refactor, its `pay` ships without the business arm and THIS spec's plan adds it; if after, 07-03's plan cites this amendment directly. Either way the acceptance test `pay_accepts_business_ids` (below) lands with whichever plan completes second.

## Contracts (the hard reference the plan is written from)

### Role  (role.rs, new)
`pub enum Role { Engineer, Labourer }` · `pub const ALL: [Role; 2]`
`Copy + Eq + Hash` for use as a `HashMap` key. Extensible: add a variant + extend `ALL`, compiler finds every match needing an update — same closed-enum pattern as multi-metal's `Metal`. Closed enum confirmed, not open-ended: impromptu/procedural business generation only needs to combine existing `Role` variants into a `HashMap<Role, RoleSlot>` passed to `create_business` — no new Rust type needed per business archetype.

### RoleSlot  (business.rs, new)
`pub struct RoleSlot { pub wage: Money, pub headcount: u32 }`
One wage figure and desired headcount per role a business employs. No behavior reads this yet. `wage` types against today's single-metal `Money`; multi-metal's migration pass revises it (that spec's Migration impact + its open "single metal or bundle" question) — do not design around the current type being final.

### Business  (business.rs, new)
`pub struct Business { pub id: AgentId, pub roles: HashMap<Role, RoleSlot> }`
`id` = account key in `Accounts` (like `Agent.id`); no balance field on the struct (§8.2). Given: attached to a house via `House.business`. `inputs`/`outputs` deliberately absent — deferred to a future `goods.rs` spec (decided, see Design).

### House.business  (housing.rs, modified)
`pub business: Option<Business>` — composition per Amendment 10. At most one business per house (v1).

### Agent  (agent.rs, modified — 3 new fields)
`pub specialization: Option<Role>` — role this agent is trained for. `None` = unspecialized/general labour.
`pub employed_role: Option<Role>` — role currently filled at `workplace`. Stored, not derived: unlike `occupants_of`, there is no other field to derive this from. Independent of `specialization` — an agent can work a role they aren't specialized in.
**Intended invariant (documented, not enforced in this pass):** `employed_role.is_some()` implies `workplace.is_some()` — a role without a workplace is meaningless. No command validates this yet; the future `assign_workplace` extension (see Open questions) is where enforcement belongs. State it in the field's doc comment so that spec enforces the right thing.
`pub education: u8` — reserved scalar, no defined effect. Range/meaning is an open question.

### World::create_business  (world.rs, new)
`pub fn create_business(&mut self, house: HouseId, roles: HashMap<Role, RoleSlot>) -> Result<AgentId, WorldError>`
Given: house exists, no existing business. Then: allocates `AgentId` from the same counter as `spawn_agent` (never a reserved id, never reused; no `Agent` struct is created — business ids join Mint/External as account-only ids); stores `Business { id, roles }` on the house; returns the id.
Error: unknown house → `Err(WorldError::UnknownHouse(house))`; already has business → `Err(WorldError::BusinessAlreadyExists(house))` (additive `WorldError` variant). Validates before touching any state — `Err` means nothing changed, matching 07-03's layer property.
Refs: @src/world.rs, 07-03 `WorldError`, §8.2

### World::businesses  (world.rs, new)
`pub fn businesses(&self) -> impl Iterator<Item = (&House, &Business)>`
Given: any world state. Then: yields every house with `business.is_some()`, paired with its business, in `houses` order. The shared way future phases (`labor_market`, `produce`, `pay_wages`, `invest`) find businesses — each phase still does its own thing with them, on its own turn, under its own money-permission contract (07-02 §6 phase table). One query, reused by many phases — not one phase owning all business behavior. Also the check backing Amendment 14's `pay` validation. A `businesses_mut` variant is future work, added only when a phase needs to mutate `Business` fields directly (none does yet).
Refs: @src/world.rs, Amendment 13 above

### Mechanical / obvious (plan owns these — no contract needed)
`Role` Debug/Display for the shell. `spawn_agent` keeps its 3-arg signature — the new `Agent` fields default (`None`/`None`/`0`); do not widen the constructor. `RoleSlot`/`Business` construction is plain struct literals — no builder. No validation logic connecting `employed_role` to a business's `roles` map — that's a future command, not this refactor.

## Acceptance tests (written from the contracts above)

- `create_business_allocates_id_from_agent_counter` — id comes from the shared counter: distinct from reserved ids, never colliding with a spawned agent's; `world.agent(business_id)` is `None`.
- `create_business_rejects_unknown_house` / `_rejects_duplicate` — the two error arms, world unchanged on `Err`.
- `agent_new_fields_default_to_none_and_zero` — a freshly spawned agent has `specialization: None`, `employed_role: None`, `education: 0`.
- `business_roles_map_holds_distinct_wages_per_role` — two roles on one business keep independent `wage`/`headcount`.
- `pay_accepts_business_ids` (Amendment 14; lands with whichever plan completes second — see amendment) — `pay` to and from a business id with funds succeeds; unknown non-business id still errors `UnknownAgent`.

--- APPROVAL GATE — do not write the plan or any code above this line without sign-off ---
*(Design approved in conversation 2026-07-16; gate cleared.)*

## Open questions

- **`education`'s type and meaning.** `u8` (0–255) is a placeholder. Scale, and what it eventually gates or multiplies, is undecided — decide in the spec that first reads it.
- **Validating `employed_role` against a business's `roles` map** — whether an agent can be marked as filling a role a business doesn't actually have open. No command enforces this yet; likely an extension of `assign_workplace` in a future spec (which also enforces the documented `employed_role ⇒ workplace` invariant).
