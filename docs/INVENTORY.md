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

Each tick redraws the town in place: the money supply and where it sits,
every house with its occupants and the business trading from it — what it
sells, at what price, its stock, its coffers and any wages it owes — and
every agent with their balance, home and the goods in their inventory.

The seeded scenario is one household of four sharing a house, three of them
employed at a farm, a theater and a jeweler — enough to watch a small
economy run rather than a blank world.

Typing an agent's name is meant to inspect one person — their workplace as
well as their holdings — but the detail is wiped by the next redraw before
it can be read (observed 2026-08-15).

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

Money is strictly conserved: each metal's total is fixed at whatever the
world was seeded with, and the sim halts rather than continue if a tick ever
fails to balance. Labor allocation, investment, degradation sinks and coin minting
are not simulated yet — those ticks pass through untouched.

- **Coins of three metals** rather than one abstract unit: every balance is
  held per metal (the shell shows them as `g:.. s:.. c:..`) and the
  conservation guarantee holds metal by metal — the sim halts naming every
  metal whose books broke, and a total across metals deliberately does not
  exist. The town trades entirely in gold for now; agents also hold small
  silver and copper savings that sit inert until a future market can price
  them.

## Terrain and the map playground — `map` · `tools/map_viewer.html`

The town sits on a generated landscape: an integer heightmap over world
coordinates measured in decimetres, with ground elevation defined everywhere
between the sampled points, not just at them. Travel across it accounts for
slope — the same distance costs more uphill, at a speed profile that can be
swapped per traveller.

`tools/map_viewer.html` is a single self-contained page — no build step, no
external requests — that draws a landscape as a hillshaded isometric surface;
drag rotates, the wheel zooms. It is also where landscapes are made:

- Generate terrain in the page itself — a seed to type or roll at random,
  the map's size and cell spacing, its maximum elevation, and the three
  octaves that set how rugged or rolling it comes out. Out-of-range entries
  are clamped and written back, so the panel always shows what was used.
- Save a landscape under a name and pick it back off a list later, kept in
  the browser's storage for that page between visits. Records store the
  parameters, not the terrain, so reselecting one regenerates it exactly.
- Load a `map.json` written by the shell's `map` command to look at the
  landscape the sim is holding. A file that isn't one says so instead of
  breaking.

The browser's generator and the simulation's produce the same landscape from
the same seed, down to the individual elevation — a pinned test on each side
fails if they ever drift apart.

Nothing in the simulation reads the terrain yet — agents have no position and
buildings have no footprint. It exists to be looked at and built on.

## Infrastructure

A single Rust binary (edition 2024, no dependencies, no database, no engine);
the simulation state lives in memory for the life of a run and the only files
written are the map exports. Quality gates: `scripts/check.sh` (item) and
`scripts/verify.sh` (pack), the latter the acceptance line for every pack.

---

*Nothing in transit — multi-metal money closed 2026-08-20
(`docs/manifests/2026-08-15-multi-metal-money.md`).*
