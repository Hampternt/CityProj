# Multi-metal money (container)

**Status:** ACTIVE 2026-08-17 — pack 1 in progress on `main`; its item
manifest is `2026-08-17-mmm-pack1-metal-core.md`.
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

Only the active pack gets an item manifest; the list below is one level deep.
Pack 1 is written and not started; pack 2 is proposed shape only.

The boundary between the two was **amended 2026-08-17** — see the Ledger. The
original split gave pack 1 the metal-keyed `Accounts` and pack 2 "every call
site migrated", which measurement showed cannot compile: one method's arity
change alone yields 2 build errors and 29 clippy errors, and `check.sh` is
exactly those two commands, so pack 1's first item would fail the item gate.
The line moved to *arity versus semantics*.

### Pack 1 — the metal-keyed core

Item manifest: `docs/manifests/2026-08-17-mmm-pack1-metal-core.md` (IN
PROGRESS 2026-08-17).

New `metal.rs`; `Accounts` keyed by `(AgentId, Metal)` with per-metal
`transfer`/`mint`/`burn`, read queries and audit; the **eleven** existing
`Accounts` tests ported (not ten — the spec's count drops
`total_money_includes_external`), plus **four** new test functions against the
spec's three names. Plus, by the amendment, the **arity sweep**: the 65 lines
in `world.rs`, `sim.rs` and `engine/game_loop.rs` that would otherwise not
compile, each written `Metal::Gold`. Behaviour is unchanged throughout and the
shell's output stays byte-identical.

Observable: the test suite proves the metals are separate ledgers — moving
and burning gold leaves silver's balance and totals untouched, an untouched
pair reads zero, and corrupting only silver still panics naming silver.

### Pack 2 — the sim runs on metals

The **semantic** migration, which is what this pack always described: `World::pay`
gains its metal parameter, worldgen decides which metals a world is seeded with,
each call site chooses its real metal in place of pack 1's `Metal::Gold`, and the
shell's money summary is rebuilt to report per metal.

Its migration list is not a static inventory but a command — every site pack 2
owns carries a literal `Metal::Gold`, so
`grep -rn 'Metal::Gold' --include=*.rs src/ | grep -v '^src/money.rs'`
regenerates it on demand. That is the replacement for the compiler errors the
amendment spends.

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

- **2026-08-17** — pack 1's manifest written
  (`2026-08-17-mmm-pack1-metal-core.md`, eight items, one ⚠), and with it a
  proposed amendment to this container's pack boundary, **accepted by the user
  the same day**. The original split was not a judgement call that went the
  other way; it was unbuildable. Changing the arity of `mint` alone gives 2
  errors under `cargo build` and 29 under `cargo clippy --all-targets -- -D
  warnings`, which are exactly the two commands `./scripts/check.sh` runs, so
  pack 1's first item failed the item gate before the pack gate was in sight.
  Rust has no overloading, so no compatibility shim exists that does not invent
  a name the spec's Contracts do not fix — and `CLAUDE.md` forbids renaming
  them. The boundary is now arity (pack 1) versus semantics (pack 2), which
  keeps pack 2's own sentence intact.

  **What the amendment costs.** This container's Decisions lean on the compiler
  for future safety — "extending the enum and letting the compiler find every
  match arm". The sweep spends that: once 65 lines compile with a hardcoded
  `Metal::Gold`, nothing fails if pack 2 misses a site. The replacement is the
  `Metal::Gold` grep recorded under Pack 2, which cannot go stale the way a
  written inventory can.

  **Three counts corrected against this container and the spec**, recorded
  rather than silently adopted: eleven `Accounts` tests, not ten; 87 money-API
  lines outside `money.rs`, against the spec's Migration impact naming three;
  and four new test functions, not three, because `#[should_panic]` is
  function-level and the spec's corrupt-only-silver case witnesses naming
  rather than independence. Pack 1's item 7 supersedes the spec's Migration
  impact for pack 2's benefit.

  Still nothing implemented, no branch cut, no box ticked. Awaiting the go on
  pack 1.
