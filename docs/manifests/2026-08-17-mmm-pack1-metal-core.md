# Multi-metal money — Pack 1: the metal-keyed core

**Status:** IN PROGRESS 2026-08-17 — go received; items executing in order on
`main` from base `7555b73`. The boundary correction below is **accepted**
(user, 2026-08-17) and the container is amended to match.
**Container:** `2026-08-15-multi-metal-money.md`
**Branch:** `main`

Observable: the test suite proves the metals are separate ledgers — moving and
burning gold leaves silver's balance and totals untouched, an untouched pair
reads zero, and corrupting only silver still panics naming silver. Nothing a
person can see changes: the shell prints the same bytes before and after.

## The container's boundary does not hold — measured, not argued

The container gives pack 1 the metal-keyed `Accounts` and gives pack 2 "every
call site migrated." Measured on a scratch copy today, that boundary produces a
pack whose every commit is red:

- Adding one parameter to `mint` alone — one method of the eight that change —
  gives **2 errors under `cargo build`** and **29 under
  `cargo clippy --all-targets -- -D warnings`**. `./scripts/check.sh` runs
  exactly those two commands, so the *item* gate fails on pack 1's first item,
  never mind the pack gate.
- The broken sites include production code: `world.rs:183` inside `World::pay`,
  `sim.rs:114` and `:159`, and the whole `game_loop.rs:132–169` render block.
  `cargo test` compiles `world.rs` and `sim.rs` too, so pack 1's own Observable
  is unreachable as drawn.

The obvious escapes do not exist. Rust has no overloading, so there is no
`mint(to, amount)` *and* `mint(to, metal, amount)`; a shim needs a new name,
and either the shim or the core then carries a name the spec's Contracts do not
fix — which `CLAUDE.md` forbids ("copy them verbatim, never rename"). A
parallel `MetalAccounts` fares worse: the spec says `Accounts` is *modified*,
and the eleven ported tests would exercise a type the sim does not use.

**The correction, accepted by the user 2026-08-17.** Move the boundary, and
move it as little as possible:

- **Pack 1 absorbs the *arity* sweep.** It changes only what would otherwise
  not compile, writing the literal `Metal::Gold` at each site. Measured forced
  set: **65 lines — 13 production, 52 test — in exactly three files**
  (`src/world.rs` 15, `src/sim.rs` 40, `src/engine/game_loop.rs` 10). Behaviour
  is unchanged throughout; the shell's output stays byte-identical.
- **Pack 2 becomes the *semantic* migration**: `World::pay` gains its `metal`
  parameter, worldgen seeds more than gold, each site chooses its real metal,
  the money summary is rebuilt per metal, and the walkthrough happens. That is
  still exactly the work the container's pack-2 sentence describes.
- **`World::pay` and `audit()` keep their signatures in pack 1**, which is why
  the forced set is 65 and not the 87 total money-API lines outside `money.rs`:
  `pay`'s body forwards at `world.rs:183` and pack 1 writes `Metal::Gold`
  *there*, leaving all 11 `.pay(` sites and all 11 `.audit()` sites untouched.
  The container's "`World::pay` gains a metal parameter" stays wholly in pack 2.

**What the correction costs, and how it is paid back.** The container's
Decisions lean on the compiler for future safety — "extending the enum and
letting the compiler find every match arm." The sweep spends that: once 65
lines compile with a hardcoded `Metal::Gold`, nothing fails if pack 2 misses a
site. The replacement is mechanical and cannot go stale: **every site pack 2
owns carries a literal `Metal::Gold`**, so

```
grep -rn 'Metal::Gold' --include=*.rs src/ | grep -v '^src/money.rs'
```

regenerates the migration list on demand. That is a better artifact than the
static inventory the signed gate asked for, and item 7 writes both down.

This was a change to an approved container, so it was put to the user rather
than absorbed. Accepted 2026-08-17; `2026-08-15-multi-metal-money.md` now
carries the amended pack-1 and pack-2 sentences and a ledger entry recording
why. The alternative was not a redrafted manifest — a rejection would have left
pack 1 with no gateable item list at all, and the container needing a different
pack 1.

## State on arrival (measured 2026-08-17)

| what | measured | the document says |
| --- | --- | --- |
| suite baseline | `cargo test` → **99 passed, 0 failed** | — |
| `#[test]` fns in `money.rs` | **14** — 11 touch `Accounts`, 3 touch only `Money` | container and spec both say "ten" |
| money-API lines outside `money.rs` | **87** — 16 production, 71 test, in 3 files | spec's Migration impact names **3** |
| lines pack 1 is forced to touch | **65** — 13 production, 52 test | container assigns all of them to pack 2 |

The three `Money`-only tests are `times_scales_a_unit_price`,
`times_panics_on_overflow` and `divided_by_floors`. By the fixed §8.1 Decision
they must **not** be ported; their staying unchanged is the positive evidence
that the Decision held. "Ten" was wrong at authorship, not stale — the same
eleven `Accounts` tests were present at the last commit touching `money.rs`
before the spec's date. The likely origin is family-grouping (mint 1 +
transfer 4 + burn 3 + audit 2 = 10), which drops `total_money_includes_external`.

Two gate hazards, both probed today rather than predicted:

- A standalone `metal.rs` item **fails `check.sh`** with exactly
  `error: enum Metal is never used` and
  `error: associated constant ALL is never used`, on the bin *and* test
  targets — this is a binary crate, so `pub` exempts nothing. But
  `impl fmt::Display for Metal` needs no allow: trait impls are exempt, and I
  verified it compiles clean with no caller.
- The **gold-pinned rekey is green**: storage moved to
  `HashMap<(AgentId, Metal), Money>` and two `HashMap<Metal, Money>` totals with
  every public signature still 1-D and `Metal::Gold` pinned inside the bodies →
  `cargo clippy --all-targets -- -D warnings` clean, `cargo test` **99 passed**,
  and not one line outside `money.rs` changed. That measurement is what makes
  item 1 possible and item-sized.

## Decisions

The container's five fixed Decisions carry over unchanged and are not
relitigated here (`Money(u64)` untouched; audit as N independent assertions;
`transfer` never crosses metals; `World::pay` gains a metal in pack 2; the
shell summary is behaviour-visible pack-2 work). So do the spec's Contracts —
every signature this pack writes is copied verbatim from them, never renamed.
What follows is only what this pack must decide and neither document does.

- **The key is flat: `balances: HashMap<(AgentId, Metal), Money>`,** with both
  totals as `HashMap<Metal, Money>` and every read still
  `.copied().unwrap_or(Money::ZERO)`. Not because the spec is ambiguous — it is
  not — but because `transfer_zero_is_noop` (`money.rs:258`) and
  `burn_zero_is_noop` (`:296`) assert `accounts.balances.is_empty()` on the
  private field. A flat key keeps both assertions valid verbatim and leaves
  `new()` genuinely empty; any per-metal sub-map pre-populated at `new()` breaks
  them. It is also what makes `unknown_metal_pair_reads_zero` pass for free.
- **No `Accounts` method may return a collection or iterator of per-metal
  `Money`** — no `balances_of(id) -> HashMap<Metal, Money>`, no
  `impl Iterator<Item = Money>`. This is the standing general form of the
  Contracts' four one-metal readers. Say plainly why it is needed: because
  `Money` stays one currency-agnostic scalar, `gold.plus(silver)` still
  typechecks after this pack. **The refusal is signature discipline, not type
  safety** — the type system cannot enforce it, so the review needs a criterion
  and this is it.
- **The audit's panic message format is pinned**, because `#[should_panic]`
  matches one substring and the tests that witness §8.3 assert on the text.
  Accumulate every failing metal — *both* failure kinds, the burned > minted
  one and the circulating ≠ minted − burned one — then panic once:
  `conservation audit failed for gold, silver (§8.3)`, the failing metals joined
  with `", "` in `Metal::ALL` order via `Display` (lowercase — `{:?}` would give
  `Silver` and the `expected =` string must match what ships). Per-metal detail
  may follow on later lines; the prefix through `(§8.3)` is the pinned part.
  **A single-metal assertion must never be pinned on the first metal in `ALL`
  order**: `"failed for gold"` is a substring of `"failed for gold, silver"`,
  so a gold-pinned test would not separate the two cases and would quietly
  reintroduce the masking bug. `"failed for silver"` is not a substring of it,
  which is why silver is the corruption target.
- **The shell's output stays byte-identical through pack 1.**
  `game_loop.rs:132–134` must become `total_money(Metal::Gold)` and friends to
  compile, but the printed string does not change: nothing mints anything but
  gold in production yet, so gold-only *is* the total and the existing label is
  not yet false. This is what gives pack 1 zero behaviour-visible surface — and
  therefore no terminal walkthrough and no 🚧 fold. Relabelling "total" → "gold"
  for honesty would import a walkthrough obligation into a test-only pack;
  refuse it, and read `game_loop.rs` in pack 1's diff as arity, not bleed.
- **Explicitly not this pack, so the review has something to check against:**
  `MoneyError` does **not** gain a metal (`WorldError::Money` carries
  `InsufficientFunds` unchanged); `audit()`'s signature does **not** change and
  never becomes a `Result`; `mint` stays bare and uncapped (the reserve cap is
  per-metal now but still deferred). All three are natural "improvements" an
  implementer makes while metal-ifying everything else.
- **The port is eleven tests, not ten**, and the count correction is recorded
  rather than silently adopted — otherwise pack 2 re-imports "10" from the
  spec's Migration impact. Eight of the eleven are pure argument-adds; three are
  not: `audit_panics_on_imbalance` (needs a target metal and a real
  `expected =`) and the two `*_zero_is_noop` private-field tests above.
- **`audit_checks_every_metal` cannot be one function**, so the Contract name
  goes on the *passing* half and the reason is written down here. `#[should_panic]`
  is function-level, and the spec asks one name to both pass and panic. Worse,
  the spec's own case does not discriminate: with `ALL = [Gold, Silver, Copper]`,
  corrupting only silver panics under a stop-at-first implementation *and* a
  collect-all one, because gold passes and silver is reached either way. It
  witnesses **naming**, never **independence**. So the pack ships **four** new
  test functions, not three: `audit_checks_every_metal` (both metals correct →
  passes), `audit_names_every_broken_metal` (corrupt gold **and** silver, both
  named — the only case that discriminates), plus the spec's
  `metals_are_independently_conserved` and `unknown_metal_pair_reads_zero`. The
  spec's "corrupt only silver → panics naming silver" is served by the ported
  `audit_panics_on_imbalance`, retargeted to silver and given its `expected =`.
- **Expected suite arithmetic: 99 → 103.** The eleven ports change no count and
  the sweep adds none, so the pack gate quotes a number, not "the suite passes."

## Items

The boundary correction above is accepted, so items 2–4 — the arity sweep —
are this pack's to carry. Nothing below is started; every box is unticked
because no go exists.

- [ ] **1. `Metal`, and the books re-keyed behind unchanged signatures.** New
  `src/metal.rs` with `pub enum Metal { Gold, Silver, Copper }`, a hand-written
  `pub const ALL: [Metal; 3]` (no crate — zero-dep convention), `Copy + Eq +
  Hash` for the map key, and a `Display` impl; `mod metal;` added to
  `main.rs`'s module list. `Accounts`'s storage becomes
  `HashMap<(AgentId, Metal), Money>` plus two `HashMap<Metal, Money>` totals,
  with `Metal::Gold` pinned inside every body so **no public signature changes
  and no file outside `money.rs`/`metal.rs`/`main.rs` is touched**. `ALL` alone
  carries `#[allow(dead_code)]` with a comment naming item 5 as its remover, in
  the style of `money.rs:187`; the enum itself is live via `Metal::Gold` and
  `Display` needs no allow.
  Done: `./scripts/check.sh` prints `CHECK OK — fmt, clippy, build clean.`,
  `cargo test` prints **99 passed** with no test renamed, and
  `git diff --name-only` lists only those three files. (Measured on a scratch
  copy today — this item is known to be reachable, not predicted.)
  Touches: `src/metal.rs` (new) · `src/money.rs` · `src/main.rs`
- [ ] **2. Lift the four read queries to take a metal.** `balance_of(id, metal)`,
  `total_money(metal)`, `total_minted(metal)`, `total_burned(metal)` — verbatim
  from the Contracts — and the **44 lines** they break (10 production, 34 test:
  `game_loop.rs` 8, `sim.rs` 2+24, `world.rs` 0+10) get `Metal::Gold`. The
  `money.rs` tests that call a reader are ported in this commit. No no-arg
  aggregate survives anywhere.
  Done: `./scripts/check.sh` prints `CHECK OK`, `cargo test money::` is green and
  the suite is still **99 passed**, the four signatures match the spec's
  Contracts character for character, and no method on `Accounts` returns
  `HashMap<Metal, Money>`, `Vec<Money>` or `impl Iterator<Item = Money>`.
  Touches: `src/money.rs` · `src/engine/game_loop.rs` · `src/sim.rs` · `src/world.rs`
- [ ] **3. Lift the three mutators to take a metal.** `transfer(from, to, metal,
  amount)`, `mint(to, metal, amount)`, `burn(from, metal, amount)` — Contracts
  verbatim, same semantics scoped to one metal, `transfer` still never moving
  value between metals — and the **20 lines** they break (3 production, 17 test:
  `world.rs:183` inside `pay`, `game_loop.rs:102` and `:110`, plus 17 test
  lines) get `Metal::Gold`. `World::pay`'s own signature is **not** touched.
  Done: `./scripts/check.sh` prints `CHECK OK`, `cargo test money::` green, suite
  still **99 passed**, and `git diff src/world.rs` shows the `pub fn pay` line
  unchanged.
  Touches: `src/money.rs` · `src/world.rs` · `src/engine/game_loop.rs` · `src/sim.rs`
- [ ] **4. Lift `set_balance_for_test`.** `set_balance_for_test(id, metal,
  amount)`, writing only that metal's cell and still touching neither totals map
  — leaving the logs alone is the entire mechanism by which an imbalance
  appears. Its one caller outside `money.rs` is `sim.rs:388`
  (`tick_runs_audit_last`), which `--all-targets` compiles, so pack 1's blast
  radius reaches outside `money.rs` here by design rather than by accident.
  Done: `./scripts/check.sh` prints `CHECK OK`, `cargo test money::` and
  `cargo test sim::` both green, suite still **99 passed**.
  Touches: `src/money.rs` · `src/sim.rs`
- [ ] **5. ⚠ Rewrite `audit` as N independent assertions, and witness it.**
  `pub fn audit(&self)` keeps its signature and never becomes a `Result`. It
  collects a verdict for every `Metal::ALL` entry — accumulating **both** the
  burned > minted case (today an `.expect()` at `money.rs:142` that aborts
  before any comparison) and the circulating ≠ minted − burned case (the
  `assert_eq!` at `:143`) — and panics **once** at the end, naming every failing
  metal in the pinned format. Ships with its two witnesses:
  `audit_checks_every_metal` (gold and silver both correct → passes) and
  `audit_names_every_broken_metal` (corrupt gold *and* silver via the item-4
  hook → panics naming both), and retargets `audit_panics_on_imbalance` to
  silver with `#[should_panic(expected = "failed for silver")]` in place of
  today's bare `#[should_panic]`. Item 1's `#[allow(dead_code)]` on `ALL` comes
  off here, since the audit iterates it and is production-reachable from
  `sim.rs:48`. ⚠ A first-failure-masks-the-rest bug passes every test the spec
  named, so this item takes its own `/code-review` pass at the item, not folded
  into the pack's.
  Done: `./scripts/check.sh` prints `CHECK OK` **with the allow removed** — this
  is the one item whose gate risk is clippy, not tests — `cargo test money::`
  shows `audit_names_every_broken_metal` passing against
  `expected = "failed for gold, silver"` and `audit_panics_on_imbalance` against
  `expected = "failed for silver"`, the suite is **101 passed**, the `audit` body
  contains no `?`, early `return` or `.expect()` that can leave the loop, and
  `grep -n 'allow(dead_code)' src/metal.rs` returns nothing.
  Touches: `src/money.rs` · `src/metal.rs`
- [ ] **6. The two remaining acceptance tests, by their Contract names.**
  `metals_are_independently_conserved` — mint gold to A and silver to B, then
  transfer and burn on one metal and assert the other's balance *and* both its
  totals are untouched; `unknown_metal_pair_reads_zero` — an untouched
  `(agent, metal)` reads `Money::ZERO`.
  Done: `./scripts/check.sh` prints `CHECK OK`, `cargo test money::` lists both
  by name passing, and the suite is **103 passed**.
  Touches: `src/money.rs`
- [ ] **7. Write down the money call-site inventory the signed gate demanded.**
  The appendix below already carries the **pre-pack baseline**, measured today;
  what this item adds is the **post-sweep** half, which cannot exist until items
  2–4 land: a per-file `Metal::Gold` count, the distinct-call-expression count
  beside the line count (`game_loop.rs:132–134` is one `println!` across three
  lines, so the two figures differ and pack 2 needs both), and the explicit
  sentence that this table **supersedes the spec's Migration impact**
  (`2026-07-12-multi-metal-money-design.md:51–57`), which names 3 sites against a
  measured 87 — so pack 2 is written against the table and not those five bullets.
  Done: the appendix carries a post-sweep column beside the baseline one, the
  supersession sentence naming `spec:51–57` is in it, and running
  `grep -rn 'Metal::Gold' --include=*.rs src/ | grep -v '^src/money.rs' | wc -l`
  prints the post-sweep total (predicted **65**), with any divergence from 65
  explained in the table rather than silently adopted.
  Touches: `docs/manifests/2026-08-17-mmm-pack1-metal-core.md`
- [ ] **8. Pack gate and review.**
  Done: `./scripts/verify.sh` prints `VERIFY OK — fmt, clippy, build, tests all
  clean.` with **103 passed** quoted verbatim; `cargo test money::` reports
  **18 passed** (today's 14 minus none plus the four new ones — the number that
  pins "eleven ported, not ten" and the four new tests in one figure); `git diff
  main -- src/money.rs` shows **no change inside `impl Money`** and none to
  `times_scales_a_unit_price`, `times_panics_on_overflow` or `divided_by_floors`,
  which is §8.1 checked mechanically rather than asserted; one `/code-review`
  pass over the pack is recorded in the Ledger; and item 5's separate ⚠ pass is
  recorded as its own line.
  Touches: (none — verification only)

## Not in this pack unless you say so

- **`World::pay`'s signature.** It stays `pay(from, to, amount)` through pack 1;
  only its body's forward at `world.rs:183` gains `Metal::Gold`. This is the
  load-bearing reason all 11 `.pay(` call sites are untouched and the forced set
  is 65 rather than 87. The container puts the parameter in pack 2; leave it there.
- **Any behaviour change in the shell.** `game_loop.rs` appears in pack 1's diff
  as arity only and its output stays byte-identical. Rebuilding the money
  summary per metal, and the terminal walkthrough that proves it, are pack 2's.
- **Choosing a real metal anywhere.** Every site outside `money.rs` says
  `Metal::Gold` after this pack — including worldgen's two seed mints, which are
  the whole money supply of a run. Which metals a world is seeded with is pack
  2's first decision, and until it is made silver and copper are permanently
  zero at runtime. That is expected, not a defect.
- **The 🚧 pointer fold.** `docs/INVENTORY.md:49–51`'s pointer is the
  *container's*, and what it promises — "balances and wages denominated per
  metal" — only becomes true after pack 2. The container folds it then. Pack 1
  states this rather than failing its own definition-of-merged check.
- **`RoleSlot.wage`, `price`, `owed_to`.** They keep type `Money`;
  `business.rs:13–17` already flags the wage question and the spec defers it to
  the market layer. Which metal a wage is denominated in is out of scope for
  packs 1 and 2 both.
- **Anything the spec put out of scope**: the seigniorage-gated minting formula,
  raw metal as a tradeable good, exchange rates and a reference currency.

## Reference files

- `docs/superpowers/specs/2026-07-12-multi-metal-money-design.md` — the approved
  spec. Its **Contracts** section is the source of truth for every signature and
  test name this pack writes; copy verbatim, never rename. Its **Migration
  impact** (`:51–57`) is the one part item 7 supersedes.
- `docs/manifests/2026-08-15-multi-metal-money.md` — the container. Its
  Decisions are fixed and carried by reference; its pack-1 sentence is what the
  correction above proposes amending, and its `2026-08-17` ledger entry is the
  gate condition item 7 discharges.
- `src/money.rs` (358 lines) — the whole of pack 1's subject. `Accounts`'s ten
  public items at `:88–207`, the twice-short-circuiting `audit` at `:137–148`,
  the sanctioned test hook at `:201–206`, the two private-field assertions at
  `:258` and `:296`, and the `#[allow(dead_code)]` precedent at `:187`.
- `src/metal.rs` — does not exist yet; item 1 creates it. `src/main.rs:45–58` is
  the module list it must join, and `main.rs:11–13`'s module map names the money
  core's mutators, so it gets a line there too.
- `src/world.rs`, `src/sim.rs`, `src/engine/game_loop.rs` — the only three files
  outside `money.rs` with a money call site, and therefore the entire surface of
  both the pack-1 sweep and the pack-2 migration. `world.rs:26` is why:
  `pub accounts: Accounts` is a public field, so `World::pay` is validation, not
  encapsulation, and everything else reaches `world.accounts.*` directly.
- `scripts/check.sh` and `scripts/verify.sh` — the two gates and the two exact
  strings every `Done:` above quotes. `check.sh`'s own header says what it
  cannot vouch for ("logic") and names `cargo test money::`; this pack is
  entirely money, so every item runs the module tests too.
- `docs/manifests/2026-08-15-terrain-playground-merge.md` — the only prior pack
  manifest in this repo and the format this one follows.
- `docs/INVENTORY.md:30, :49–51, :92–93` — where the container's 🚧 pointer
  lives and the footer that still reads "proposed".

## Appendix — the money call-site inventory

**Pre-pack baseline, measured 2026-08-17 against `HEAD`.** Item 7 appends the
post-sweep column beside it; until then this table describes the tree as it
stands, not as the pack leaves it.

Counting rule, so pack 2 can re-derive it rather than trust it: **lines** in
`src/` outside `src/money.rs`, excluding comment-only lines, matching

```
grep -rnE '\.(transfer|mint|burn|balance_of|total_money|total_minted|total_burned|set_balance_for_test|audit|pay)\(' --include=*.rs src/ | grep -v '^src/money.rs' | grep -vE ':[[:space:]]*//'
```

split at each file's `mod tests` line (`world.rs:306`, `sim.rs:273`;
`game_loop.rs` has no test module, so all of it is production).

| file | production | test | total (baseline) | forced in pack 1 | post-sweep `Metal::Gold` |
| --- | --- | --- | --- | --- | --- |
| `src/world.rs` | 1 | 26 | 27 | 15 | 15 |
| `src/sim.rs` | 5 | 45 | 50 | 40 | 40 |
| `src/engine/game_loop.rs` | 10 | 0 | 10 | 10 | 10 |
| **total** | **16** | **71** | **87** | **65** | **65** |

By method: `balance_of` 35, `mint` 19, `audit` 11, `pay` 11, `total_money` 5,
`total_minted` 3, `total_burned` 1, `transfer` 1, `set_balance_for_test` 1,
`burn` 0. Zero sites in `agent.rs`, `business.rs`, `goods.rs`, `housing.rs`,
`main.rs`, `market.rs`, `role.rs`, `terrain.rs`.

The 22-line gap between 87 and 65 is exactly the `audit()` and `.pay(` calls,
whose signatures pack 1 does not change — verified, not inferred: re-running the
grep restricted to `audit|pay` and subtracting any line that also matches a
signature-changing method gives **22**, and the overlap set is empty, so the two
halves partition the 87 with nothing double-counted. The forced 65 splits by
item: readers 44 (item 2), mutators 20 (item 3), `set_balance_for_test` 1
(item 4).

**Post-sweep, measured 2026-08-17 after items 2–4 landed.** The migration
grep prints **67**, not the predicted 65, and the divergence is explained,
not adopted: **2** of the 67 are in `src/metal.rs` — the `ALL` constant's own
definition and the `Display` match arm — which are the enum defining itself,
not call sites choosing a metal. `metal.rs` did not exist when the counting
rule was written; pack 2 should extend the grep's exclusion to it
(`grep -v '^src/metal.rs'`), which leaves exactly the predicted **65** in the
three files above, split 13 production / 52 test as forced.

Lines and distinct call expressions **coincide at 65**: after `cargo fmt`,
every migrated call expression sits on its own line and none shares a line.
The statement-level caveat survives one level up: the shell's money summary
(`game_loop.rs:133–135`) is three call expressions inside **one** `println!`
statement, so pack 2 rewrites it as a unit, not as three independent sites.

This table **supersedes the spec's Migration impact**
(`2026-07-12-multi-metal-money-design.md:51–57`), which names 3 sites against
the measured 87. Pack 2 is written against this table and the regenerating
grep, not those five bullets.

## Ledger

- **2026-08-17** — pack manifest drafted. Inherited from the container as
  approved: the goal, the five fixed Decisions, the pack-1/pack-2 split, and
  the Observable, which is reused verbatim rather than reworded. Inherited from
  the spec's signed approval gate: the Contracts, which fix every signature and
  the three new test names, and **one condition** — pack 1's done-condition must
  produce the inventory of money call sites, because "every call site migrated"
  is pack 2's only unbounded phrase. Item 7 is that condition; the appendix is
  where it lands. Measurements taken before drafting, all on `HEAD`: suite green
  at **99 passed**; `money.rs` holds **14** `#[test]` fns, not ten — 11 on
  `Accounts`, 3 on `Money` arithmetic that §8.1 forbids porting; **87** money-API
  lines outside `money.rs` against the spec's three; a single method's arity
  change gives **2** `cargo build` errors and **29** under
  `cargo clippy --all-targets -- -D warnings`; a bare `metal.rs` fails the item
  gate with two dead-code errors while a gold-pinned storage rekey is fully
  green at 99 tests with nothing outside `money.rs` touched. **Corrections this
  pack records rather than absorbs:** the container's and the spec's "ten
  existing money tests" is eleven, so pack 2 must not re-import "10" from
  `spec:53`; the spec's Migration impact undercounts by roughly an order of
  magnitude and item 7 supersedes it; the spec's `audit_checks_every_metal` does
  not discriminate a stop-at-first audit from a collect-all one, so the pack
  ships four new test functions against the spec's three names, with the
  Contract name kept on the passing half. **Open questions this pack does not
  answer**, all carried from the spec and none blocking this substrate: the
  reference/unit-of-account currency, the seigniorage-gated minting formula, and
  whether `RoleSlot.wage` is one metal or a bundle. It also does not answer
  which metals worldgen seeds — that is pack 2's first decision. **Not decided
  here, and needing the user:** the boundary correction at the top of this file.
  It amends an approved container's pack-1 sentence, so it is proposed, not
  taken. Three lines elsewhere go stale the moment a go arrives and should ship
  with the first commit: the container's "Nothing is active — the container is
  PROPOSED" (`2026-08-15-multi-metal-money.md:45`, already contradicted by its
  own `PLANNED` status line), its `### Pack 1` heading, which needs a state
  marker and an `Item manifest:` line to resolve the container↔pack link, and
  `docs/INVENTORY.md:92–93`'s "*In transit: multi-metal money, proposed*".
  Nothing is implemented, no branch cut, no box ticked. Awaiting go.

- **2026-08-17** — **boundary correction accepted by the user.** The entry above
  left it open as the one thing needing them; it is now taken. The container
  `2026-08-15-multi-metal-money.md` carries the amended pack-1 and pack-2
  sentences, a `### Pack 1` heading that names this file, and a ledger entry
  recording what the amendment costs (the compiler no longer finds pack 2's
  sites) and what repays it (every one carries a literal `Metal::Gold`, so a
  grep regenerates the list). Items 2–4 — the arity sweep — are therefore this
  pack's to carry, and the conditional that opened the item list is resolved.

  Two of the three stale lines the entry above flagged are fixed with it: the
  container's "Nothing is active — the container is PROPOSED" and its `### Pack
  1` heading, which now resolves the container↔pack link. The third,
  `docs/INVENTORY.md:92–93`'s "*In transit: multi-metal money, proposed*", is
  left deliberately — "proposed" stops being true when work starts, not when a
  plan is agreed, so it ships with the first commit rather than now.

  Still a plan. Nothing implemented, no branch cut, no box ticked. Awaiting the
  go on item 1.
