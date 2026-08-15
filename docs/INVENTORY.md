# Inventory

The live map of what this repo **is**, at feature altitude: what exists and
what it does — never how it's implemented (that lives in `CLAUDE.md` and the
design specs). Work in flight appears as a 🚧 pointer to its manifest in the
section where it will land; converting that placeholder into a real entry is
part of the definition of merged. Manifests live in `docs/manifests/`.

---

## The simulated town — `cargo run`

A city economy simulated from the household up, advanced one tick at a time
from a terminal shell. Enter advances a tick, `q` quits.

Each tick redraws the town: the money supply and where it sits, every house
with its occupants and the business trading from it, and every agent with
their balance, home, workplace and the goods in their inventory. Typing an
agent's name inspects that one person instead.

The seeded scenario is a farm, a theater and a jeweler with the households
that work in them — enough to watch a small economy run rather than a blank
world.

## What a tick does

The behaviors you can watch play out over successive ticks:

- **Businesses produce.** Each turns its inputs into stock it can sell.
- **Wages are paid from the employer's own coffers.** A business that cannot
  cover its payroll pays what it can and carries the rest as arrears, shown
  against the worker until revenue returns and clears it.
- **Households shop for what they need.** Agents buy the goods they are
  running short of, at the seller's asking price, as far as their money goes.
- **Prices move.** A business that sold out raises its price; one that didn't
  sell lowers it. New prices take effect the following tick, so a shortage
  and its correction are visible a tick apart.
- **Goods are consumed**, which is what creates next tick's need.

Money is strictly conserved: the town's total is fixed at whatever the world
was seeded with, and the sim halts rather than continue if a tick ever fails
to balance. Labor allocation, investment, degradation sinks and coin minting
are not simulated yet — those ticks pass through untouched.

- 🚧 Coins of several metals rather than one abstract unit — balances and
  wages denominated per metal, and the conservation guarantee holding metal
  by metal. Container: `docs/manifests/2026-08-15-multi-metal-money.md`

## Terrain and the map viewer — `map` · `tools/map_viewer.html`

The town sits on a generated landscape: an integer heightmap over world
coordinates measured in decimetres, with ground elevation defined everywhere
between the sampled points, not just at them. Travel across it accounts for
slope — the same distance costs more uphill, at a speed profile that can be
swapped per traveller.

`map` in the shell writes the current landscape to `map.json`.
`tools/map_viewer.html` opens in any browser with no server or build step and
draws that file as a hillshaded isometric landscape; drag rotates, the wheel
zooms.

Nothing in the simulation reads the terrain yet — agents have no position and
buildings have no footprint. It exists to be looked at and built on.

- 🚧 Terrain playground — generating and tuning landscapes in the browser
  instead of only viewing exported ones, with named maps saved locally.
  Pack: `docs/manifests/2026-08-15-terrain-playground-merge.md`

## Infrastructure

A single Rust binary (edition 2024, no dependencies, no database, no engine);
the simulation state lives in memory for the life of a run and the only files
written are the map exports. Quality gates: `scripts/check.sh` (item) and
`scripts/verify.sh` (pack), the latter the acceptance line for every pack.

---

*In transit: the terrain playground merge
(`docs/manifests/2026-08-15-terrain-playground-merge.md`); multi-metal money
proposed (`docs/manifests/2026-08-15-multi-metal-money.md`).*
