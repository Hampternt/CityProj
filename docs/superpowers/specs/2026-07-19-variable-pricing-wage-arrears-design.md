# Variable Goods Pricing + Wage Arrears — Design Spec

## Design (decisions — approve before reading on)

Two halves of one feedback loop, shipped together. (1) Businesses adjust their
posted price each tick from local sell-through — per-business Walrasian
tâtonnement ("sold out → raise, didn't sell → lower"), the simple end of the
adjustment-based family the parent doc requires (the BazaarBot/Doran-Parberry
bid-ask auction stays a possible later swap-in; it needs randomness and a
different market shape, rejected for now to keep the sim deterministic).
(2) Wages are paid from business coffers only: a business that can't cover its
wage bill drains its account to zero and carries the remainder as **wage
arrears** — a debt ledger on the business, repaid automatically when revenue
returns. The phase-8 unconditional wage-bill mint (the 07-19 "accepted broken
faucet") is deleted; worldgen's seed becomes the entire money supply and the
audit now pins `total_money() == seeded` forever. Money circulates instead of
being spawned.

- **Fits into:** @src/market.rs (adjust_price), @src/business.rs (`owed_to`
  ledger), @src/sim.rs (phases 3, 4, 8), @src/engine/game_loop.rs (summary
  lines), CLAUDE.md (state + new Roadmap section).
- **Invariants in play:** §8.2 single chokepoint (arrears are bookkeeping on
  the business, never negative balances; every payment is a normal
  `transfer`); §8.3 audit unchanged; §8.4 mint remains the only creation path
  — it is simply not called during ticks anymore; §8.5 no overdraft (partial
  wage payment is a full valid transfer of a smaller amount); §8.6 pricing
  logic stays in market.rs.
- **Out of scope (YAGNI):** wage adjustment (same tâtonnement shape, phase 1,
  future spec); employee happiness / job-switching (will read the arrears
  ledger); physical goods movement (see Roadmap); gold-backed mint business;
  multi-metal money; interest on arrears; business bankruptcy/closure.

### Tuning constants (market.rs, same status as the goods table)

| Constant | Value | Meaning |
|---|---|---|
| high sell-through | ≥ 9/10 of offered | raise price one step |
| low sell-through | < 1/2 of offered | lower price one step |
| step | `max(1, price / 10)` | proportional, integer-safe |
| price floor | `Money(1)` | price can always recover upward |

All ratio checks are integer cross-multiplication (`sold * 10 >= offered * 9`,
`sold * 2 < offered`) — no floats, per §8.1.

### Expected scenario behavior (not asserted, but the point)

The 07-19 farm overproduces (40 made, ~10 eaten per tick), so food's price
falls to the floor — that is the correct oversupply signal, not a bug. Market
*balance* is explicitly not a goal of this spec; production/manpower tuning
comes later. The observable win is: total money supply constant every tick,
arrears visible when a business runs dry, and arrears draining back to zero
when revenue returns.

## Contracts (the hard reference the plan is written from)

### Unit: adjust_price  (market.rs)

Signature: `pub fn adjust_price(price: Money, offered: u32, sold: u32) -> Money`
Given:  a business's posted `price`, its shelf stock at the phase-4 snapshot
        (`offered`), and units actually sold in the apply pass (`sold`;
        callers guarantee `sold <= offered`).
Then:   pure and deterministic. `offered == 0` → `price` unchanged (no
        signal). Sell-through ≥ 9/10 → `price + max(1, price/10)`.
        Sell-through < 1/2 → `price − max(1, price/10)`, saturating at the
        floor `Money(1)`. Otherwise unchanged. A floor-price seller with poor
        sales stays at `Money(1)`.
Error:  none — total function.
Refs:   @src/market.rs, §8.6 pricing stays in market.rs, §8.1 integer money.

### Unit: goods_market price write-back  (sim.rs, phase 4)

Given:  the existing decide→apply pass, with per-business `offered` taken
        from the same tick-start `Offer` snapshot the buyers planned against.
Then:   the apply pass counts units sold per business; after the last intent
        is applied, each business's `price` is replaced by
        `adjust_price(price, offered, sold)`. New prices take effect next
        tick — the decide pass never observes a mid-phase price change.
        Businesses with no offer in the snapshot are untouched.
Error:  none.
Refs:   @src/sim.rs `goods_market`, decide→apply template, §8.6.

### Unit: pay_wages with arrears  (sim.rs phase 3 + business.rs)

`Business` gains `pub owed_to: HashMap<AgentId, Money>` — wage debt per
worker. Arrears and the current wage share one pot; payment always drains the
pot, so "repay when profitable" needs no separate mechanic.

Given:  a staffed business whose worker has a slotted `employed_role` (the
        07-19 eligibility rules are unchanged).
Then:   per such business, in `World::businesses()` iteration order: (1) add this
        tick's `slot.wage` to `owed_to[worker]`; (2) pay
        `min(balance_of(business), owed_to[worker])` via the normal validated
        transfer (skip the call when zero); (3) subtract the amount paid from
        `owed_to[worker]`, removing the entry at zero. Coffers drain to
        exactly zero before any wage goes unpaid; a funded business is
        identical to today's behavior.
Error:  the transfer cannot err by construction (amount ≤ balance); if it
        does, skip cleanly per §8.5 — never partial-apply the ledger update
        without the transfer.
Refs:   @src/sim.rs `pay_wages`, @src/business.rs, §8.2, §8.5.

### Unit: mint_phase becomes a stub  (sim.rs, phase 8)

Given:  any world.
Then:   no money operation of any kind (TODO comment points at the future
        gold-backed mint business). Consequently `total_minted` changes only
        at worldgen and the §8.3 audit pins the supply at the seed forever.
Error:  none.
Refs:   @src/sim.rs `mint_phase`, §8.4.

### Mechanical / obvious (plan owns these — no contract needed)

- Sold-units counting inside `goods_market` (a `HashMap<AgentId, u32>`
  incremented in the apply pass) — trivial bookkeeping.
- Shell summary additions in game_loop.rs: per-business current price and
  outstanding arrears, plus the (now constant) total money supply — display
  only.
- `Business::wage_bill()` keeps existing callers/tests; phase 8 no longer
  reads it (doc comment update only).
- Doc updates: CLAUDE.md current-state section (phases 3/4/8 descriptions,
  price no longer "fixed at worldgen"), stale doc comments in business.rs,
  and the CLAUDE.md Roadmap section below.
- Test updates where assertions encoded the old faucet
  (`minimal_economy_feeds_the_worker_and_breaks_the_idle`,
  `mint_tops_up_staffed_businesses_by_one_wage_bill`).

## Roadmap notes this spec records (documentation deliverable, not design)

Added to CLAUDE.md as a short **Roadmap** section; future specs design them:

- **Physical goods movement.** Goods will physically travel: producer →
  market / wherever needed, moved by hauler/logistics-type businesses. Agents
  already have inventories; houses and market venues get their own storage.
  Agents will travel to a market to buy and back home to stow goods. Until
  then businesses sell from their own `stock`; the seam where "shelf" becomes
  "market venue inventory" is the `Offer` struct — don't bake in the
  assumption that stock lives only on businesses.
- **Money creation** stays as already designed (parent doc §2.1): a *literal
  staffed Mint business* that consumes precious-metal goods to mint coins
  (seigniorage formula deferred by the 07-12 multi-metal spec), plus trade
  with outside markets through `External`.
- **Wage market.** The `adjust_price` pattern applied to `RoleSlot.wage` in
  phase 1 (can't fill a slot → raise, queue of applicants → lower), plus
  employee happiness / job-switching driven by the arrears ledger.

--- APPROVAL GATE — do not write the plan or any code above this line without sign-off ---

## Open questions

None — thresholds, step, floor, ledger semantics, and faucet removal were all
decided in brainstorming (2026-07-19).
