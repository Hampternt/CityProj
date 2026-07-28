# World Coordinates & Terrain Foundation — Design Spec

## Design (decisions — approve before reading on)

A pure math foundation for world geometry, with no in-sim consumer yet. One
new module `src/terrain.rs` provides: (1) a single integer 3D coordinate
space — `Point3 { x, y, z: i64 }` in world units, **1 unit = 0.1 m** — that
everything with a location (agents, buildings, market venues, future roads)
will share; (2) a grid-sampled heightmap `Terrain` — integer elevation
samples at regular grid vertices, each cell split into two triangles along a
fixed diagonal, so every in-bounds (x, y) has an exact interpolated ground
height; (3) movement math — distance, signed grade, and an elevation-aware
`travel_time` (uphill slower, downhill faster, capped) that future
hauler/wagon mechanics will call; (4) a tiny dependency-free deterministic
value-noise generator so there is terrain worth looking at; (5) a `map`
shell command that exports the terrain to `map.json`, rendered by a new
self-contained `tools/map_viewer.html` (hillshaded isometric view, rotate/
zoom, no libraries, no server) so the human can visually verify the system.

Numeric discipline follows the deterministic-sim standard (and the Money
precedent): all *stored* state is integer; f64 is allowed only transiently
inside pure math functions; anything returned into sim state (elevations,
travel times in ticks) is quantized back to integers. Rendering-side
performance tricks (LOD, chunking) are explicitly not this layer's concern.

**Roadmap note recorded now (not built now):** buildings will occupy 3D
*volumes* (footprint + vertical extent, above or below ground) addressed by
`Point3` coordinates in this space — never tile-locked to the terrain grid.
The terrain grid is only how the ground surface is stored, not how things
are located.

- **Fits into:** new @src/terrain.rs; @src/engine/game_loop.rs (the `map`
  shell command — I/O only); new tools/map_viewer.html; CLAUDE.md (state
  section + Roadmap).
- **Invariants in play:** no money moves, so §8.2–§8.5 are untouched; §8.1's
  spirit (integer state, deterministic) extends to geometry as described
  above; §8.6 by analogy — movement/terrain math lives in terrain.rs only,
  with its tuning constants alongside (the `adjust_price` pattern).
- **Out of scope (YAGNI):** positions on existing agents/houses/businesses;
  pathfinding; roads; wagons/haulers; building volumes; terrain modification;
  water/rivers; multi-node maps; serde or any new dependency.

### Coordinate conventions

| Decision | Value |
|---|---|
| World unit | 0.1 m (integer `i64` everywhere state is stored) |
| Axes | x, y horizontal; z = elevation (up positive) |
| Terrain domain | x ∈ [0, (vertices_x − 1)·cell_size], same for y |
| Cell triangulation | fixed SW→NE diagonal in every cell |
| Quantization | f64 → i64 via round-half-away-from-zero (`.round()`) |

### Travel tuning constants (terrain.rs, same status as the pricing table)

| Constant | Value | Meaning |
|---|---|---|
| `WAGON_FLAT_SPEED` | 50 units/tick | baseline speed on flat ground |
| `UPHILL_DRAG` | 8.0 | speed divisor grows as 1 + 8·grade (grade > 0) |
| `DOWNHILL_BOOST` | 2.0 | speed multiplier grows as 1 + 2·|grade| (grade < 0) |
| `DOWNHILL_CAP` | 1.25× | downhill speed never exceeds 1.25 × flat |

Effective speed: `flat / (1 + UPHILL_DRAG·g)` when g > 0;
`min(flat·(1 + DOWNHILL_BOOST·|g|), DOWNHILL_CAP·flat)` when g < 0. A 10%
uphill grade ≈ 0.56× flat speed. These are first-guess values; retuning them
is a constants edit, not a design change.

## Contracts (the hard reference the plan is written from)

### Unit: Terrain + elevation_at  (terrain.rs)

Signature: `pub fn elevation_at(&self, x: i64, y: i64) -> Result<i64, TerrainError>`
on `pub struct Terrain { vertices_x: u32, vertices_y: u32, cell_size: i64, elevations: Vec<i64> }`
(row-major, `vertices_x · vertices_y` samples; constructor validates length
and `cell_size > 0`).
Given:  an in-domain (x, y) in world units.
Then:   pure. Locates the containing cell, picks the triangle via the fixed
        SW→NE diagonal, barycentric-interpolates the three vertex
        elevations, quantizes per the conventions table. At a grid vertex
        the stored sample is returned exactly.
Error:  (x, y) outside the domain → `Err(TerrainError::OutOfBounds)`.
Refs:   @src/terrain.rs, coordinate conventions table.

### Unit: grade  (terrain.rs)

Signature: `pub fn grade(terrain: &Terrain, from: (i64, i64), to: (i64, i64)) -> Result<f64, TerrainError>`
Given:  two in-domain ground points with nonzero horizontal separation.
Then:   pure. Returns rise/run: (elevation_at(to) − elevation_at(from)) /
        horizontal 2D distance. Sign convention: uphill (to higher) is
        positive. f64 is transient — never stored.
Error:  either point out of domain → `OutOfBounds`; from == to →
        `Err(TerrainError::ZeroRun)`.
Refs:   @src/terrain.rs.

### Unit: travel_time  (terrain.rs — the future hauler seam)

Signature: `pub fn travel_time(terrain: &Terrain, from: (i64, i64), to: (i64, i64), profile: &SpeedProfile) -> Result<u64, TerrainError>`
with `pub struct SpeedProfile { pub flat_speed: u32 /* units/tick */ }` plus
the drag/boost/cap constants above; `pub const WAGON: SpeedProfile`.
Given:  two distinct in-domain ground points.
Then:   pure. 3D distance between the ground points (using their
        interpolated elevations) divided by effective speed from the tuning
        table, `ceil`ed to integer ticks; result ≥ 1. The asymmetry
        invariant holds on effective speed (pre-quantization): for nonzero
        grade, speed(uphill) < speed(flat) ≤ speed(downhill) ≤ cap·flat —
        tick counts only show it once segments are long enough to not
        collapse under `ceil`, which is what the tests must use.
        Single-segment straight line — no pathfinding.
Error:  out of domain → `OutOfBounds`; from == to → `ZeroRun`.
Refs:   @src/terrain.rs, travel tuning constants table.

### Unit: Terrain::generate  (terrain.rs)

Signature: `pub fn generate(seed: u64, vertices_x: u32, vertices_y: u32, cell_size: i64) -> Terrain`
Given:  any seed; dimensions ≥ 2×2; cell_size > 0.
Then:   deterministic and dependency-free: identical inputs produce an
        identical `Terrain` on every platform (integer/hash-based lattice
        noise — algorithm is the plan's to choose within this constraint).
        All elevations in [0, 400] units (0–40 m) with visible relief:
        rolling hills and at least one valley at the intended shipped size
        (64×64 vertices, cell_size 50 — a ~320 m square at 5 m spacing).
Error:  none for valid dims; dims < 2 or cell_size ≤ 0 panic (worldgen
        programmer error, matching template_world's style).
Refs:   @src/terrain.rs.

### Unit: map export  (game_loop.rs command → terrain.rs serializer)

Signature: `pub fn to_json(&self) -> String` on `Terrain`; shell command
`map` writes it to `map.json` in the working directory and prints the path.
Given:  any valid terrain (the shell holds one generated at startup with a
        fixed seed, alongside — not inside — the economy `World`).
Then:   hand-serialized flat JSON, no serde:
        `{"unit_meters":0.1,"cell_size":C,"vertices_x":X,"vertices_y":Y,"elevations":[...]}`
        row-major, same order as storage. This format is the viewer's input
        contract — viewer and exporter change together.
Error:  file-write failure prints an error line to the shell; the sim
        continues.
Refs:   @src/terrain.rs, @src/engine/game_loop.rs, tools/map_viewer.html.

### Mechanical / obvious (plan owns these — no contract needed)

- `Point3 { x, y, z: i64 }` and `distance(a: Point3, b: Point3) -> f64` —
  Euclidean, f64 transient; trivial.
- `TerrainError` enum (`OutOfBounds`, `ZeroRun`) — mirrors `MoneyError`'s
  shape.
- The noise algorithm internals (hash function, octave count, smoothing) —
  free choice within the `generate` contract.
- tools/map_viewer.html internals: single self-contained file, `<input
  type="file">` to load map.json, canvas hillshaded isometric render with
  drag-rotate and wheel-zoom, no external resources. Verified by opening in
  a browser, not by unit tests.
- Shell help text gains the `map` command; CLAUDE.md state section gains
  terrain.rs and the viewer; Roadmap gains the building-volumes note above.
- Tests: vertex-exact and midpoint interpolation, grade sign both
  directions, travel-time asymmetry (uphill > flat ≥ downhill, same pair),
  generator determinism (two calls, same seed, identical), out-of-bounds
  and zero-run errors, JSON golden sample for a 2×2 terrain.

--- APPROVAL GATE — do not write the plan or any code above this line without sign-off ---

## Open questions

None — representation, numerics, viewer approach, terrain source, and scope
were all decided in brainstorming (2026-07-28).
