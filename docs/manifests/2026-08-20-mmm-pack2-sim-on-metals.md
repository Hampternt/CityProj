# Multi-metal money — Pack 2: the sim runs on metals

**Status:** ACTIVE 2026-08-20 — item list and D1–D3 approved by the user,
strawmen taken on all three; go given the same day.
**Container:** `2026-08-15-multi-metal-money.md`
**Branch:** `claude/multimetal-plan-project-f85924` (the session's worktree
branch, already cut from `main` at `6346526`).

Observable (container's sentence, reused verbatim): the seeded town runs ticks
as before, with the header showing gold, silver and copper separately instead
of one total, and the audit green every tick.

## State on arrival (measured 2026-08-20)

- Suite green at **103 passed** — `verify.sh` re-run twice during the
  2026-08-20 independent review of pack 1 (`VERIFY OK`, quoted in pack 1's
  ledger).
- The migration grep — with the `metal.rs` exclusion pack 1's appendix added —
  prints exactly **65** lines (13 production, 52 test) in `world.rs`,
  `sim.rs`, `engine/game_loop.rs`:

  ```
  grep -rn 'Metal::Gold' --include=*.rs src/ | grep -v '^src/money.rs' | grep -v '^src/metal.rs'
  ```

- One finding handed over by that review: the `mint_phase` doc comment
  (`sim.rs:275–279`) still reads `total_money()` zero-arg — stale since pack
  1's item 2, on lines pack 1 never touched. Item 4 owns it.
- Pack 1's appendix supersedes the spec's Migration impact (`spec:51–57`);
  this pack is written against the grep and that table, not those bullets.

## Decisions

Carried fixed from the container and spec, not relitigated: `Money(u64)`
currency-agnostic; `transfer` never crosses metals; no exchange rates, no
reference currency, no seigniorage; `RoleSlot.wage`, `price` and `owed_to`
keep type `Money` (which metal a wage is in is out of scope for this pack —
the pay call carries it).

What this pack must decide, and neither document does:

- **Fixed by the spec's own scope — gold is the only trading metal.** With
  exchange rates and a reference currency out of scope, nothing can *price*
  in silver or copper, so every wage, purchase and transfer the running sim
  makes stays gold — now written at each call site as an explicit choice
  rather than pack 1's forced literal. Silver and copper circulate only as
  seeded holdings until the market layer learns to price them. This is why
  the semantic sweep is mostly *affirmation*, not change, and the manifest
  says so up front rather than letting the sweep look like a no-op.
- **D1 (user) — which metals worldgen seeds.** Strawman: keep the gold
  economy exactly as seeded today, and additionally give each household a
  small inert silver and copper holding (numbers to taste at implementation,
  e.g. 10 silver / 20 copper per agent). Nothing spends them, so behavior is
  untouched — but the header shows three live totals, and the per-metal audit
  pins three nonzero supplies in production instead of two zeros.
  Alternative: seed gold only and let silver/copper display zeros — minimal
  and honest, but the Observable's three-metal header shows two dead rows and
  the production audit never exercises a nonzero non-gold ledger.
  Recommendation: the strawman.
- **D2 (user) — the money summary's layout** (spec:56 leaves it to the plan).
  Strawman: one line per metal in `Metal::ALL` order —
  `gold    total=419  minted=419  burned=0` — replacing today's single
  `total=… minted=… burned=…` line; mint/external reserve lines likewise per
  metal. The old single-total line cannot survive: a cross-metal total is
  exactly the number the core now refuses to compute.
- **D3 (user) — per-agent and per-business balance display.** Strawman: the
  compact form `g:35 s:10 c:20` everywhere a single balance prints today
  (roster lines, inspect view), all three metals always shown — visible
  zeros beat rows that appear and vanish. Alternative: print nonzero metals
  only.
- **Expected suite arithmetic: 103 → 105.** Item 1 adds
  `pay_moves_only_the_named_metal`; item 2 adds
  `template_world_seeds_the_decided_metals`. The pack gate quotes a number,
  not "the suite passes."

## Items

Boxes unticked; nothing starts without a go. Items 1–2 are independent; 3
needs 2 (it renders what worldgen seeds); 4 sweeps after 1–3 stop moving the
grep; 5 closes.

- [x] **1. `World::pay` gains its metal.** The signature becomes
  `pay(&mut self, from: AgentId, to: AgentId, metal: Metal, amount: Money)
  -> Result<(), WorldError>` — the parameter the spec fixes (`spec:57`), in
  `transfer`'s argument order, forwarding `metal` and deleting the literal at
  `world.rs:184`. All 11 `.pay(` sites (regenerate:
  `grep -rn '\.pay(' --include=*.rs src/`) pass `Metal::Gold` explicitly —
  production callers because wages and prices are gold by the fixed Decision,
  test callers affirming gold as the metal under test. `WorldError::Money`
  carries `InsufficientFunds` unchanged. Plus the new world-level acceptance
  test `pay_moves_only_the_named_metal`: fund an agent in gold and silver,
  pay in silver, assert gold's balance and totals untouched.
  Done: `./scripts/check.sh` prints `CHECK OK`; `cargo test money::`,
  `cargo test world::`, `cargo test sim::` green; suite **104 passed**; the
  `pay` signature matches this item character for character.
  Touches: `src/world.rs` · `src/sim.rs` · `src/engine/game_loop.rs`
- [ ] **2. Worldgen chooses the seed metals.** `template_world` seeds per D1,
  each seed mint naming its metal as a real choice; the worldgen comments say
  what is seeded and why the non-gold holdings are inert. Plus
  `template_world_seeds_the_decided_metals`: build the template world and
  assert each metal's `total_money`/`total_minted` against the decided
  amounts.
  Done: `./scripts/check.sh` prints `CHECK OK`; suite **105 passed**; the new
  test names each metal's expected total explicitly.
  Touches: `src/engine/game_loop.rs`
- [ ] **3. The shell reports per metal.** The money summary — one `println!`
  across `game_loop.rs:133–135`, one statement, rewritten as a unit per pack
  1's appendix caveat — becomes D2's per-metal lines; mint/external reserve
  lines and every agent/business balance line become D3's form. This is the
  pack's behavior-visible surface: byte-identity with pack 1 deliberately
  ends here, replaced by item 5's walkthrough.
  Done: `./scripts/check.sh` prints `CHECK OK`; suite still **105**; a manual
  `cargo run` over one tick shows the D2 header and D3 balances.
  Touches: `src/engine/game_loop.rs`
- [ ] **4. The semantic sweep, affirmed site by site.** Regenerate the grep;
  walk every remaining `Metal::Gold` and either change it (none expected —
  say so if true) or affirm it as a real choice; record the closing count and
  the per-file split in the appendix beside pack 1's 65. Fix the stale
  `mint_phase` doc comment (`sim.rs:275–279`) to per-metal phrasing — the
  handed-over review finding. Update `CLAUDE.md`'s structure section (money
  summary now per metal; pack 2 landed) and `docs/INVENTORY.md:92–94`'s
  in-transit footer.
  Done: the appendix carries the post-pack-2 column with any divergence
  explained; `grep -n 'total_money()' src/` returns nothing;
  `./scripts/check.sh` prints `CHECK OK`.
  Touches: `src/sim.rs` · `docs/manifests/2026-08-20-mmm-pack2-sim-on-metals.md` · `CLAUDE.md` · `docs/INVENTORY.md`
- [ ] **5. Walkthrough, pack gate, container close.** The terminal
  walkthrough (this repo's "real browser" — the shell *is* the feature):
  `cargo run` over at least 3 ticks in a real terminal, header showing the
  three metals separately, audit green every tick; the captured output goes
  in the Ledger. `./scripts/verify.sh` quoted with its test count. One
  `/code-review` pass over the pack recorded. The container's 🚧 pointer in
  `docs/INVENTORY.md:49–51` folds into a real entry — this is the
  container's last pack, so its definition of merged is discharged here —
  and the container's status and ledger close.
  Done: `VERIFY OK` + **105 passed** quoted verbatim; walkthrough transcript
  in the Ledger; 🚧 folded; container marked DONE.
  Touches: `docs/INVENTORY.md` · `docs/manifests/2026-08-15-multi-metal-money.md` · (verification otherwise)

## Not in this pack unless you say so

- **Any non-gold *trade*.** No price, wage, or offer in silver or copper;
  no conversion between metals anywhere. Blocked on the reference-currency
  question, which stays open.
- **`RoleSlot.wage` growing a metal.** Wages are paid via
  `pay(.., Metal::Gold, ..)`; the wage *type* stays `Money` per the carried
  Decision.
- **Metal-aware `market.rs`.** `plan_purchases`, `Offer` and `adjust_price`
  are untouched; the goods market clears in gold via the pay call.
- **The Mint business, seigniorage, raw-metal goods, exchange rates** — the
  spec's own out-of-scope list, unchanged.

## Reference files

- `docs/superpowers/specs/2026-07-12-multi-metal-money-design.md` — the
  approved spec; `spec:57` is this pack's `pay` sentence, `spec:56` the
  render one. Its Migration impact stays superseded by pack 1's appendix.
- `docs/manifests/2026-08-17-mmm-pack1-metal-core.md` — pack 1: the
  appendix's counting rule and 65-site table this pack sweeps against, and
  the 2026-08-20 review entry this draft inherits its one finding from.
- `src/engine/game_loop.rs` — worldgen (`template_world`) and the entire
  render surface; no test module today, so item 2's test founds one (or
  lives beside `template_world` — implementer's call, named here so it is
  not invented silently).
- `src/world.rs:181–186` — `pay`'s body, the one signature this pack changes.

## Ledger

- **2026-08-20** — drafted, from the container's pack-2 sentence, the spec's
  two pack-2 bullets (`spec:56–57`), pack 1's appendix (the 65-site list and
  its regenerating grep), and the 2026-08-20 independent review's handover
  (the stale `mint_phase` comment). Decisions D1–D3 are strawmen needing the
  user; the item list is the checkpoint the workflow puts at their level.
  Suite measured green at 103; grep measured at exactly 65. Nothing
  implemented, no branch cut, no box ticked. Awaiting approval of the item
  list and D1–D3, then a go.
