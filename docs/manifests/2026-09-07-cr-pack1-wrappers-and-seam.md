# CR pack 1 — the wrappers, the helper, and the null seam

**Container:** [`2026-09-07-conserved-recycle.md`](2026-09-07-conserved-recycle.md) ·
**Spec:** [`2026-09-07-conserved-recycle-design.md`](../superpowers/specs/2026-09-07-conserved-recycle-design.md)
(signed 2026-09-07) · **Status:** DONE 2026-09-07

## State on arrival

191 tests green at `8ee72d3`. Phase 7 (`sinks`) runs emigration only; phase 8
(`mint_phase`) is literally `fn mint_phase(_world: &mut World) {}`. `World::pay` is
the **only** command-layer wrapper over the §8.2 chokepoint — there is none for
`mint` or for `burn`, and `Accounts::burn` carries
`#[allow(dead_code)] // the sinks phase (7) lands later` with zero non-test callers.
`total_minted(Gold)` and `total_burned(Gold)` have not moved since genesis.

## Decisions

- **This pack changes no scenario behavior, and its gate proves that rather than
  asserting it.** Nothing is wired into a phase. `RECYCLE_PERMILLE` is introduced
  at **0** (the spec's stated trajectory: pack 2 item 2 freezes it at 20), and
  `tick_with_rate` takes the rate but no phase reads it yet.
- **`#[allow(dead_code)]` on the new items, retired by pack 2** — the repo's
  `assign_home` precedent (`// no caller until the labor market lands`). Landing
  code ahead of its consumer under `-D warnings` has no other honest option, and
  the comment names the pack that removes it.
- **`Accounts::burn` loses its `#[allow(dead_code)]` in this pack**, because
  `World::levy` is a non-test caller the moment it exists — even though nothing in
  a phase calls `levy` yet. That is the attribute's actual condition ("no caller"),
  not a proxy for "unused by the sim".
- **The narrow id rule is `self.agent(id).is_some()`, deliberately NOT
  `is_known_account`** — that rejection is recorded in the spec's `World::levy`
  contract and is a design decision, so the tests pin it in both directions rather
  than leaving it to look like an oversight.

## Items

- [x] **1. `WorldError::OverIssue { requested, remaining }`.** New variant beside
      the existing ones in `src/world.rs`, with the doc-comment style the enum
      already uses. No `Display` impl exists on `WorldError` today — check before
      writing one; if there is none, this item adds no formatting code.
- [x] **2. `World::levy`.** `pub fn levy(&mut self, from: AgentId, metal: Metal,
      amount: Money) -> Result<(), WorldError>`. Validates `from` is a spawned
      agent FIRST, then forwards to `Accounts::burn` unchanged. Retires
      `#[allow(dead_code)]` from `Accounts::burn` in the same edit.
- [x] **3. `World::disburse`.** `pub fn disburse(&mut self, to: AgentId, metal:
      Metal, amount: Money, pot: Money) -> Result<Money, WorldError>`. Validates
      `to` is a spawned agent FIRST, then `amount <= pot`, then credits via
      `Accounts::mint` and returns the remaining pot. `Money::ZERO` returns
      `Ok(pot)` **without calling `mint`** — `mint` has no zero guard and would
      insert a zero-balance entry.
- [x] **4. `sim::RECYCLE_PERMILLE` (at 0) and `sim::levy_amount`.** Pure, total,
      scalar-taking, beside its phase — the `draw_amount`/`insolvent_now`
      precedent, NOT `market.rs` (§8.6). The constant's doc comment carries the
      end-state value, the sweep justification, **the refuted share-vs-basket
      account**, and the n=1 note, so pack 2's freeze is an edit to one number
      rather than a re-derivation.
- [x] **5. `sim::tick_with_rate`, with `tick` delegating.** `tick` keeps its ⟨REF⟩
      signature verbatim. The rate parameter is threaded but read by no phase in
      this pack.
- [x] **6. The 🚧 pointer in `docs/INVENTORY.md`.**
- [x] **7. Unit tests (A13 and the `levy_amount` property table).** All outside the
      sim: the narrow id rule in both directions on both wrappers (Mint id,
      External id, a live business id, a ghost id — each refusing with the books
      provably untouched); the zero no-ops (neither wrapper creates an account
      entry, and `disburse` returns the pot unchanged); `levy`'s insufficient-funds
      atomicity; `disburse`'s `OverIssue` atomicity with `total_minted` unmoved;
      and `levy_amount(b, p) <= b` over a table including the 49/50 boundary at
      20‰, `p = 0` for every `b`, and `p = 1000`.
- [x] **8. Gate.** `./scripts/verify.sh` green with output quoted, **plus the
      bit-identical proof**: the three town soaks and the genesis assertions are
      not merely green, their event streams are diffed against the pre-pack build.
      A pack gated as "zero behavior" must prove it.

## Ledger

*(append-only; each entry quotes real gate output)*

- **2026-09-07 — pack 1 DONE, all eight items.** 191 → **201 tests**.

  ```
  VERIFY OK — fmt, clippy, build, tests all clean.
  test result: ok. 201 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```

  **Item 8's bit-identical proof, done as a diff rather than asserted.** A
  throwaway probe dumped, for each of 300 ticks of `town_world()`, every
  `Event` in the `TickReport`, the per-metal `total_money`/`total_minted`/
  `total_burned`, and every agent's gold wallet by name. The same probe was
  run in a worktree at the pre-pack commit `8ee72d3` and in this tree, and
  the two outputs diffed:

  ```
  28237 stream_pre.txt
  28237 stream_post.txt
  BIT-IDENTICAL: event streams, per-metal totals and wallets match over 300 ticks
  ```

  The probe was removed from both trees afterwards; `src/engine/worldgen.rs`
  is byte-identical to its pre-pack state (`git diff --stat` empty), so the
  scenario and its three soaks are untouched by this pack in the strongest
  available sense.

- **Two findings from doing it, recorded because they bear on pack 2.**
  1. **`tick_with_rate`'s parameter is `_permille` in this pack**, and that
     is load-bearing rather than cosmetic: it is *why* the bit-identity above
     is provable. `pack_one_ships_the_rate_at_zero_and_no_phase_reads_it`
     pins it from the other side — it runs 25 ticks at `permille = 999`
     against 25 plain `tick` calls and asserts the event streams and all
     three metals' totals match. **Pack 2 must delete that test's second half
     in the same commit that wires phase 7**, because the moment the rate is
     read the test becomes false by design. Named here so it is retired
     deliberately, not discovered red.
  2. **`Accounts::burn`'s `#[allow(dead_code)]` came off in this pack** even
     though no phase calls the levy yet — `World::levy` is a non-test caller
     the moment it exists, which is the attribute's actual condition. The two
     new wrappers and `levy_amount` carry their own
     `#[allow(dead_code)] // no phase calls it until pack 2 wires the …`,
     each naming the pack that removes it (the `assign_home` precedent).

- **Deviation from the manifest as written, item 1.** The item said to check
  whether `WorldError` has a `Display` impl before writing one. It does not —
  only `impl From<MoneyError> for WorldError` at @src/world.rs:202 — so
  `OverIssue` ships as a documented variant with no formatting code, exactly
  as the item's conditional anticipated. No `Display` was added, and none is
  owed until something renders a `WorldError` to a user.
