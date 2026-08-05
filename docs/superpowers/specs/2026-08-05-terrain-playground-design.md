# Terrain Playground — Design Spec

## Design (decisions — approve before reading on)

Grow `tools/map_viewer.html` into a terrain playground: generate random maps
in the browser, tweak generation parameters, and save/reselect named maps —
purely for looking at terrain, not wired into the sim. The value-noise
generator is ported to JS and kept bit-identical with Rust at the shipped
constants; a pinned-value canary on both sides catches drift. The existing
render path (isometric projection, hillshade, drag/zoom) and the
`map.json` file input are untouched.

- **Fits into:** @tools/map_viewer.html (all UI + JS generation);
  @src/terrain.rs (one new test only — no behavior change).
- **Invariants in play:** none of §8 (no money, no sim state). The standing
  contract honored is the viewer ↔ `Terrain::to_json` pairing: the input
  format and in-memory map shape stay exactly as shipped.
- **Out of scope (YAGNI):** feeding a selected map into worldgen; exporting
  playground maps back to files; server components; touching Rust
  `generate`'s signature or constants; saving elevation arrays (parameters
  only — generation is deterministic).

### Controls (top-left panel over the canvas)

- **Seed**: text input (u64 decimal) + **Random** button (rolls a fresh
  seed, regenerates immediately). Generate button applies a typed seed.
- **Size**: vertices X / vertices Y (default 64), cell size (default 50).
- **Shape**: max elevation (default 400) and three editable octave rows as
  (period, amplitude) pairs, defaults matching Rust's `NOISE_OCTAVES`:
  (16, 1.0), (8, 0.5), (4, 0.25).
- **Saved maps**: name field + Save, dropdown of saved maps, Delete.
  Selecting restores all parameters and regenerates. Saving under an
  existing name replaces it.
- Existing file input stays for shell-exported `map.json`.

### Error handling

Inputs clamp to sane ranges instead of erroring: dims 2–512, cell size ≥ 1,
period ≥ 1, amplitude > 0, max elevation ≥ 1, seed parsed as u64 (invalid →
0). Malformed JSON on file-load reports in the bar instead of failing
silently.

## Contracts (the hard reference the plan is written from)

### Unit: generateTerrain  (tools/map_viewer.html, JS)

Signature: `function generateTerrain({seed, verticesX, verticesY, cellSize, maxElevation, octaves}) -> {cell_size, vertices_x, vertices_y, elevations}`
Given:  `seed` a u64 (as BigInt or exactly-representable Number), dims ≥ 2,
        `cellSize` ≥ 1, `octaves` an array of `{period ≥ 1, amplitude > 0}`,
        `maxElevation` ≥ 1.
Then:   returns a map object in the exact in-memory shape the existing
        `draw()` consumes (row-major `elevations`, snake_case keys), with
        elevations in `[0, maxElevation]`. **Parity invariant:** at the
        default parameters (octaves (16,1)/(8,0.5)/(4,0.25), max elevation
        400) the elevations are integer-for-integer identical to Rust
        `Terrain::generate(seed, vx, vy, cell)` for every seed. The 64-bit
        hash uses BigInt with wrapping masks (bit-exact splitmix64-style
        avalanche, same constants as `lattice_value`); noise blending and
        rounding are plain f64 doubles mirroring Rust's operation order.
        Rounding is `Math.floor(v + 0.5)` — equal to Rust's
        half-away-from-zero `.round()` for the non-negative values produced
        here (JS `Math.round` differs on negative halves; not used).
Error:  none — callers clamp inputs first (playground tool, not a library).
Refs:   @src/terrain.rs `generate`, `value_noise`, `lattice_value`,
        `smoothstep`; @tools/map_viewer.html input contract comment.

### Unit: saved-map record  (localStorage)

Signature: `localStorage["cityproj.saved_maps"] = JSON of {name: record}`
Given:  a record is `{seed: string (u64 decimal), vertices_x, vertices_y,
        cell_size, max_elevation: number, octaves: [[period, amplitude], …]}`.
Then:   parameters only — never elevation arrays; restoring a record and
        regenerating reproduces the identical map (determinism). Seed is a
        string because u64 exceeds Number's exact-integer range.
Error:  missing/corrupt key on load → treated as empty (playground resets
        the store rather than crashing).
Refs:   @tools/map_viewer.html.

### Unit: parity canary  (both sides)

Signature: JS: startup self-check; Rust: `#[test] fn generate_matches_viewer_canary()`
Given:  seed 20260728, 64×64 vertices, cell size 50, default shape.
Then:   vertex (0, 0) → elevation **235**; vertex (33, 17) (world
        (1650, 850)) → elevation **183**. JS checks at page load and logs a
        console warning naming `src/terrain.rs` as the source of truth on
        mismatch; the Rust test asserts the same two values via
        `elevation_at` so drift is caught on both sides.
Error:  n/a (canary only warns in JS; fails the test in Rust).
Refs:   @src/terrain.rs tests, @tools/map_viewer.html.

### Mechanical / obvious (plan owns these — no contract needed)

Control-panel DOM/CSS, input clamping, dropdown wiring, random-seed rolling
(any 64-bit source; determinism only matters after the seed is fixed),
file-input error message, and the panel's layout. All trivial; no
interpretations to invent.

--- APPROVAL GATE — do not write the plan or any code above this line without sign-off ---

## Open questions

None.
