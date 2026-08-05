# Mechanic ideas — theorycraft parking lot (non-binding)

Nothing here is designed, approved, or scheduled. Each idea starts its own
brainstorming session when it is picked up; these notes record *intent and
decisions worth inheriting* — never interfaces or signatures (a consumer-less
API guesses wrong; a recorded decision stays useful). Constraints on current
code, if any, are called out explicitly per idea.

All of the ideas below assume the shared world-coordinate space and terrain
from the 2026-07-28 terrain spec
([`docs/superpowers/specs/2026-07-28-terrain-coordinates-design.md`](superpowers/specs/2026-07-28-terrain-coordinates-design.md)).

## Resource fields — collapse-style ore prospecting (recorded 2026-08-05)

Ores and other in-ground resources are neither pre-placed veins nor
seed-predetermined hidden data. Until prospected, a region has *no fact*
about ore at all — it carries only a probability law: a pure formula giving,
for a prospect attempt over an **area** (a claim region, not a single
coordinate), the chance of a find and the distribution of its quality and
quantity. The law may be conditioned on geology the terrain makes queryable
(elevation, slope), on the area searched, and on the ledger of prior
outcomes. Prospecting is *measurement*: when an agent invests in an attempt,
the sim draws from the distribution and the outcome **collapses into
permanent stored state** — a vein with now-fixed location, quality, and
quantity, or a recorded dry attempt. Only collapsed outcomes (finds, dry
holes, claims, depletion) are ever sim state; untouched regions store
nothing.

Prior collapses condition later ones, hump-shaped over nearby find density: a
few nearby strikes first *raise* the chance (working mines teach prospectors
where to look), then growing density *suppresses* it toward zero (the
region's easy ore is found and drained), so fresh veins get progressively
harder to find in exploited areas.

Draws come from the sim's seeded RNG stream, so replays of the same event
history collapse identically. Because randomness lives at measurement time
(not worldgen), the law's formula can be retuned mid-game — it only governs
future draws; everything already found is ledger fact.

Open when this gets designed: the law's shape and tuning, prospecting
economics, and whether a dry hole permanently fixes "nothing here" or merely
lowers the odds of trying again.

Constraint on current code: none.

## Roads & paths (recorded 2026-08-05)

Roads live in the shared coordinate space, not on the terrain grid: a road is
free-form geometry — a polyline (or curve) of waypoints with a **width** —
so curviness is just waypoint density, never tile shapes. Grade along each
segment comes from the terrain; per-segment `travel_time` is the natural
seam (road travel = sum of segment times, with road quality plausibly a
speed multiplier on `SpeedProfile`). Incline/decline could constrain routing
or pricing: steep segments cost more to build/maintain, or cap wagon loads.
Roads are probably built and maintained by businesses, connecting to the
hauler/logistics thread in CLAUDE.md's roadmap.

Constraint on current code: none — `travel_time` is already single-segment,
which is exactly the primitive a polyline sums over.

## Zones (recorded 2026-08-05)

Assignable areas: polygonal regions in world coordinates designating use —
residential, market grounds, farmland, and mining **claims** (the ore idea
above is probably the first concrete zone consumer). Needs area/region math
(point-in-region, area, overlap) that intentionally does not exist yet;
design it when the first consumer spec arrives so the representation
(polygon vs. rect vs. cell-set) is chosen by real needs.

Constraint on current code: none.

## Water — rivers, waterways, lakes (recorded 2026-08-05)

Water interacts with the heightmap rather than being authored: lakes fill
basins, rivers follow descending elevation — so water is plausibly *derived*
from terrain (which means the generator's valleys matter beyond looks).
Effects pull in two directions: a barrier to land movement (fords, bridges —
a roads interaction) and a cheap transport medium (boats/barges as an
alternative `SpeedProfile`), plus location value (mills, irrigation, ports —
ties into zones). The terrain spec lists water as out of scope; nothing in
the heightmap design blocks adding it later.

Constraint on current code: none.
