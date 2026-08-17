# Multi-metal money (container)

**Status:** PLANNED 2026-08-17 — the spec's approval gate is signed. Pack 1 is
next and its item manifest is not written yet; nothing is active.
**Branch:** `main`
**Origin:** the design spec
`docs/superpowers/specs/2026-07-12-multi-metal-money-design.md`, written
2026-07-12 and pending approval since. `CLAUDE.md` has carried it as the
next body of work; this container is that intent written down during the
2026-08-15 workflow conversion, not a decision to proceed.

## Goal

Gold, silver and copper become independently spendable currencies rather
than raw material converted into one abstract unit. Every balance, wage and
conservation guarantee is scoped to a metal, and the trusted core is made
incapable of summing across metals — without a market rate, a cross-metal
total is a meaningless number, so the signatures refuse to produce one.

Deliberately not in this container, per the spec's own scope: the
seigniorage formula that would gate minting (`mint` stays uncapped), raw
metal as a tradeable good, exchange rates and a reference currency, and the
wage spec that stays on hold behind all of it.

## Decisions (fixed by the spec, 2026-07-12, amended 2026-07-15)

- **`Money(u64)` is unchanged and stays currency-agnostic.** `Metal` is a new
  orthogonal key, not a new numeric type — §8.1 survives untouched.
- **The audit becomes N independent assertions**, one per metal, and must not
  stop at the first failure; the panic names the metal that broke. Adding a
  metal later means extending the enum and letting the compiler find every
  match arm.
- **`transfer` never moves value between metals.** Conversion is a future
  market trade — two calls, not one.
- **`World::pay` gains a metal parameter.** The 07-03 command layer shipped
  single-metal knowingly; this container is the revision pass that was
  planned for it (decided 2026-07-15), not a discovered defect.
- **The shell's money summary is behavior-visible work, not a signature fix.**
  It currently prints one global total, which stops being a meaningful
  number the moment metals are independent.

## Packs

Only the active pack gets an item manifest; the list below is the proposed
shape, one level deep. Nothing is active — the container is PROPOSED.

### Pack 1 — the metal-keyed core

New `metal.rs`; `Accounts` keyed by `(AgentId, Metal)` with per-metal
`transfer`/`mint`/`burn`, read queries and audit; the ten existing money
tests ported, plus the spec's three new ones.

Observable: the test suite proves the metals are separate ledgers — moving
and burning gold leaves silver's balance and totals untouched, an untouched
pair reads zero, and corrupting only silver still panics naming silver.

### Pack 2 — the sim runs on metals

Every call site migrated: `World::pay`, the world and sim tests, and the
shell's money summary rebuilt to report per metal.

Observable: the seeded town runs ticks as before, with the header showing
gold, silver and copper separately instead of one total, and the audit green
every tick.

## Open questions (carried from the spec, blocking later work, not this)

- A reference currency for prices — one designated metal others trade
  against? Blocks the market layer and the wage spec.
- The seigniorage-gated minting formula: its own future spec, needing goods
  and market.
- Whether `RoleSlot.wage` is one metal or a bundle. Decide when unblocked.

## Ledger

- **2026-08-15** — container opened during the workflow conversion, PROPOSED.
  It records an intent that already existed in `CLAUDE.md`; the spec's
  approval gate remains unsigned, so the pack sequence is proposed shape
  only and no pack manifest exists.

- **2026-08-17** — gate signed; container moves to PLANNED. Cleared on the
  grounds that all three open questions belong to the market layer and none
  blocks this substrate. **One condition on pack 1:** its done-condition must
  produce the inventory of money call sites, because "every call site
  migrated" is pack 2's only unbounded phrase and pack 2 should be written
  against a number rather than a guess. No pack manifest written and no work
  started — signing the gate authorizes planning, not execution.
