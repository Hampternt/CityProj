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

Each tick redraws the town in place under a header showing the tick,
population, and employment counts, then narrates what just happened as an
event feed: hires and quits, wage and price moves, production, payrolls
(aggregated per business at town scale), sales, who went hungry, and who
arrived in or left town. The money summary sits above the feed; below it, every
house with its occupants and the business trading from it — what it sells,
at what price, its stock, its coffers and any wages it owes.

The seeded scenario is a town of thirty named residents across four
occupied residences and two spare cottages, working six multi-worker
businesses — two farms, two entertainment venues, two luxury workshops —
with sixteen employed at seed and the rest finding work through the labor
market in the opening ticks. Its constants were tuned against pinned soak
criteria and frozen; the manifests record every measured deviation.

`roster` lists every agent on a line — job, wallet, pantry. Typing an
agent's name inspects one person (balances, home, workplace, goods, hunger,
and the last three feed lines they starred in); typing a business's address
inspects the business. Inspection output waits for Enter, so it can
actually be read before the next redraw.

## What a tick does

The behaviors you can watch play out over successive ticks:

- **The labor market clears.** The unemployed apply to the best-paying open
  slot; racing applicants are settled by live headcount. Posted wages float
  — an unfilled slot raises its offer when the business can genuinely
  afford it (net of wage debts), a queue of surplus applicants lowers it.
  A worker owed too many ticks of back pay walks out, and never applies to
  an employer still owing them.
- **Businesses produce.** Each adds stock in proportion to its staff.
- **Wages are paid from the employer's own coffers.** A business that cannot
  cover its payroll pays what it can and carries the rest as arrears, shown
  against the worker until revenue returns and clears it.
- **Households shop for what they need.** Agents buy the goods they are
  running short of, at the seller's asking price, as far as their money goes.
- **Prices move.** A business that sold out raises its price; one that didn't
  sell lowers it. New prices take effect the following tick, so a shortage
  and its correction are visible a tick apart.
- **Goods are consumed**, which is what creates next tick's need — and an
  agent who cannot cover a day's food goes visibly hungry, day after day.
- **Population moves in both directions.** An agent worn down by hunger and
  too poor to buy food leaves town: any back wages are settled as far as
  the employer's coffer reaches, and everything they own — every metal —
  is swept out through the External account, so nothing orphans. When a
  job stands open long enough with a vacant residence to offer, a named
  newcomer arrives on a small External-funded grubstake and applies for
  work the next day. Immigration stalls by design when the fund drains or
  no residence stands empty.

- **Owners take profit.** Every business is founded by a named resident
  (the seeded venues by their founding worker), and each tick a business
  pays its gold above a retained safety buffer — three payrolls deep, plus
  every coin of back wages it still owes — to its owner, narrated in the
  feed. Creditors therefore always sit ahead of the owner, though a venue
  earning well enough to cover both still pays out. An owner who leaves
  town leaves their venue's draws suspended until closure lands.
  Coffers no longer pool without bound; the owner's wealth does, which is
  the capital the coming founding mechanic spends.

🚧 Firm lifecycle in flight
(`docs/manifests/2026-08-22-firm-lifecycle.md`): closure (a persistently
insolvent venue liquidates — back wages settled, staff laid off, the
residual to its owner, the freed house a landing pad) and founding (a
capitalized resident opens a replacement venue into measured scarcity)
land as packs 2–3; the profit draw above was pack 1.

Money is strictly conserved: each metal's total is fixed at whatever the
world was seeded with, and the sim halts rather than continue if a tick ever
fails to balance — arrivals and departures included. Business closure and
founding, degradation sinks beyond emigration, and coin minting are not
simulated yet — those ticks pass through untouched.

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

*Previous: town colony sim closed 2026-08-21
(`docs/manifests/2026-08-21-town-colony-sim.md`) — the narrated town
console, the phase-1 labor market, and migration through External, folded
into the entries above. Before that: multi-metal money, closed 2026-08-20
(`docs/manifests/2026-08-15-multi-metal-money.md`).*
