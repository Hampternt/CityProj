# World Coordinates & Terrain Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `src/terrain.rs` — shared integer 3D coordinates, a triangulated heightmap with exact interpolated ground height, grade/travel-time movement math, a deterministic value-noise generator — plus a `map` shell command exporting `map.json` and a self-contained `tools/map_viewer.html` to eyeball it.

**Architecture:** One new pure-math module (`terrain.rs`) with no in-sim consumer; the shell holds a `Terrain` alongside (not inside) the economy `World` and only does I/O for the `map` command. All stored state is `i64`; f64 appears only transiently inside pure functions. Spec: `docs/superpowers/specs/2026-07-28-terrain-coordinates-design.md` — its Contracts section is verbatim law for names and signatures.

**Tech Stack:** Rust edition 2024, std only (no serde, no new dependencies). Viewer is one hand-written HTML file: vanilla JS + canvas, no libraries, no server.

**Worktree note (from project memory):** if executing in a fresh worktree created by EnterWorktree, it branches from a possibly-stale `origin/main` — fast-forward to local `main` before starting.

## Global Constraints

- World unit = **0.1 m**; `i64` everywhere state is stored; f64 transient only.
- Quantization f64 → i64 via **round-half-away-from-zero** (Rust's `.round()`).
- Cell triangulation: **fixed SW→NE diagonal** in every cell.
- Terrain domain: x ∈ [0, (vertices_x − 1)·cell_size], same for y (closed at both ends).
- Tuning constants (exact): `WAGON_FLAT_SPEED` = 50 units/tick, `UPHILL_DRAG` = 8.0, `DOWNHILL_BOOST` = 2.0, `DOWNHILL_CAP` = 1.25×. Effective speed: `flat / (1 + UPHILL_DRAG·g)` when g > 0; `min(flat·(1 + DOWNHILL_BOOST·|g|), DOWNHILL_CAP·flat)` when g < 0.
- `generate`: no transcendental functions (`sin`, `pow`, …) — integer/hash and f64 arithmetic only (libm varies across platforms). Elevations in [0, 400].
- JSON format (exact, hand-serialized): `{"unit_meters":0.1,"cell_size":C,"vertices_x":X,"vertices_y":Y,"elevations":[...]}` row-major.
- Spec signatures are copied verbatim — `grade` and `travel_time` are **free functions** taking `&Terrain`, not methods. Do not "fix" this.
- Verify each task with `cargo test terrain` (and the full suite before commits); `cargo fmt` before every commit.

---

### Task 1: Module scaffold — `TerrainError`, `Point3`, `distance`

**Files:**
- Create: `src/terrain.rs`
- Modify: `src/main.rs:42-51` (mod list) and `src/main.rs:9-30` (module-map doc comment)
- Test: in-file `#[cfg(test)] mod tests` in `src/terrain.rs` (project convention)

**Interfaces:**
- Consumes: nothing.
- Produces: `pub enum TerrainError { OutOfBounds, ZeroRun }`; `pub struct Point3 { pub x: i64, pub y: i64, pub z: i64 }`; `pub fn distance(a: Point3, b: Point3) -> f64`.

- [ ] **Step 1: Write the failing tests**

Create `src/terrain.rs`:

```rust
//! World geometry: the shared integer coordinate space, the ground-surface
//! heightmap, and movement math (grade, travel time). Pure math only — no
//! sim state, no I/O. Spec:
//! `docs/superpowers/specs/2026-07-28-terrain-coordinates-design.md`.
//!
//! Numeric discipline mirrors [`crate::money`]: everything *stored* is
//! integer (1 unit = 0.1 m); f64 exists only transiently inside pure
//! functions and is quantized back via round-half-away-from-zero.

/// Errors from terrain queries. Mirrors `MoneyError`'s shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainError {
    /// The queried (x, y) lies outside the terrain's domain.
    OutOfBounds,
    /// A grade/travel query where `from == to` — no horizontal run.
    ZeroRun,
}

/// A location in world coordinates (1 unit = 0.1 m; z is elevation, up
/// positive). Everything with a position shares this space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point3 {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}

/// Straight-line (Euclidean) distance between two points, in world units.
/// f64 is transient — callers quantize if they store the result.
pub fn distance(a: Point3, b: Point3) -> f64 {
    let dx = (b.x - a.x) as f64;
    let dy = (b.y - a.y) as f64;
    let dz = (b.z - a.z) as f64;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_is_euclidean_in_the_plane() {
        let a = Point3 { x: 0, y: 0, z: 0 };
        let b = Point3 { x: 3, y: 4, z: 0 };
        assert!((distance(a, b) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn distance_includes_elevation() {
        let a = Point3 { x: 0, y: 0, z: 0 };
        let b = Point3 { x: 2, y: 3, z: 6 };
        assert!((distance(a, b) - 7.0).abs() < 1e-12);
    }
}
```

(Scaffold and implementation land together here — `distance` is trivial and the spec marks it mechanical; the test-first cycle is the compile failure below.)

- [ ] **Step 2: Register the module and confirm tests fail first**

In `src/main.rs`, add to the mod list (alphabetical, between `sim` and `world`):

```rust
mod terrain;
```

and add to the module-map doc comment (after the `sim` bullet):

```rust
//! - [`terrain`] — world coordinates and the ground heightmap; pure
//!   movement math (grade, travel time) with its tuning constants
//!   alongside, plus the deterministic terrain generator.
```

Before writing the file, running `cargo test terrain` fails to find the module; after Step 1 + this step it compiles.

- [ ] **Step 3: Run the tests**

Run: `cargo test terrain -- --nocapture`
Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
cargo fmt && cargo test
git add src/terrain.rs src/main.rs
git commit -m "feat: terrain module scaffold — Point3, distance, TerrainError"
```

---

### Task 2: `Terrain` struct, constructor, `elevation_at`

**Files:**
- Modify: `src/terrain.rs`
- Test: same file's `mod tests`

**Interfaces:**
- Consumes: `TerrainError` (Task 1).
- Produces: `pub struct Terrain { vertices_x: u32, vertices_y: u32, cell_size: i64, elevations: Vec<i64> }` (fields private); `pub fn new(vertices_x: u32, vertices_y: u32, cell_size: i64, elevations: Vec<i64>) -> Terrain` (panics on invalid input); `pub fn elevation_at(&self, x: i64, y: i64) -> Result<i64, TerrainError>`.

- [ ] **Step 1: Write the failing tests**

Add to `mod tests`:

```rust
/// 2×2 vertices, cell_size 10 → domain [0,10]². Row-major elevations
/// [SW, SE, NW, NE] = [0, 10, 20, 40].
fn ramp() -> Terrain {
    Terrain::new(2, 2, 10, vec![0, 10, 20, 40])
}

#[test]
fn elevation_is_exact_at_all_four_vertices() {
    let t = ramp();
    assert_eq!(t.elevation_at(0, 0), Ok(0));
    assert_eq!(t.elevation_at(10, 0), Ok(10));
    assert_eq!(t.elevation_at(0, 10), Ok(20));
    // Far corner (max_x, max_y) exercises the closed-domain clamp.
    assert_eq!(t.elevation_at(10, 10), Ok(40));
}

#[test]
fn elevation_interpolates_in_the_southeast_triangle() {
    // fx=0.8, fy=0.2 (fx ≥ fy): 0 + 0.8·(10−0) + 0.2·(40−10) = 14.
    assert_eq!(ramp().elevation_at(8, 2), Ok(14));
}

#[test]
fn elevation_interpolates_in_the_northwest_triangle() {
    // fx=0.2, fy=0.8 (fx < fy): 0 + 0.2·(40−20) + 0.8·(20−0) = 20.
    assert_eq!(ramp().elevation_at(2, 8), Ok(20));
}

#[test]
fn elevation_agrees_on_the_diagonal() {
    // (5,5) lies on the SW→NE diagonal; both triangle planes give 20.
    assert_eq!(ramp().elevation_at(5, 5), Ok(20));
}

#[test]
fn elevation_rounds_half_away_from_zero() {
    let t = Terrain::new(2, 2, 10, vec![0, 5, 0, 5]);
    // fx=0.3: 0.3·5 = 1.5 → rounds to 2, not 1.
    assert_eq!(t.elevation_at(3, 0), Ok(2));
}

#[test]
fn elevation_rejects_out_of_bounds() {
    let t = ramp();
    assert_eq!(t.elevation_at(-1, 0), Err(TerrainError::OutOfBounds));
    assert_eq!(t.elevation_at(11, 0), Err(TerrainError::OutOfBounds));
    assert_eq!(t.elevation_at(0, 11), Err(TerrainError::OutOfBounds));
}

#[test]
#[should_panic(expected = "elevations")]
fn new_rejects_wrong_sample_count() {
    Terrain::new(2, 2, 10, vec![0, 10, 20]);
}

#[test]
#[should_panic(expected = "cell_size")]
fn new_rejects_nonpositive_cell_size() {
    Terrain::new(2, 2, 0, vec![0, 10, 20, 40]);
}

#[test]
#[should_panic(expected = "2x2")]
fn new_rejects_degenerate_grid() {
    Terrain::new(1, 2, 10, vec![0, 10]);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test terrain`
Expected: FAIL to compile — `Terrain` not defined.

- [ ] **Step 3: Implement `Terrain`, `new`, `elevation_at`**

Add to `src/terrain.rs` (above the tests):

```rust
/// A grid-sampled heightmap: integer elevation samples at regular grid
/// vertices, row-major. Each cell is split into two triangles along a
/// fixed SW→NE diagonal, so every in-domain (x, y) has an exact
/// interpolated ground height. The grid is only how the ground surface is
/// stored — positions are free `Point3` coordinates, never tile-locked.
pub struct Terrain {
    vertices_x: u32,
    vertices_y: u32,
    cell_size: i64,
    elevations: Vec<i64>,
}

impl Terrain {
    /// Builds a terrain from raw samples. Panics on invalid input —
    /// programmer error, matching `generate`'s style: the sample count
    /// must be `vertices_x · vertices_y`, dimensions at least 2×2, and
    /// `cell_size` positive.
    pub fn new(vertices_x: u32, vertices_y: u32, cell_size: i64, elevations: Vec<i64>) -> Terrain {
        assert!(
            vertices_x >= 2 && vertices_y >= 2,
            "terrain needs at least a 2x2 vertex grid"
        );
        assert!(cell_size > 0, "cell_size must be positive");
        assert_eq!(
            elevations.len(),
            vertices_x as usize * vertices_y as usize,
            "elevations must hold exactly vertices_x * vertices_y samples"
        );
        Terrain { vertices_x, vertices_y, cell_size, elevations }
    }

    /// Interpolated ground height at (x, y), in world units. Pure: locates
    /// the containing cell, picks the triangle via the fixed SW→NE
    /// diagonal, linearly interpolates the three vertex elevations, and
    /// quantizes with round-half-away-from-zero. At a grid vertex the
    /// stored sample is returned exactly.
    pub fn elevation_at(&self, x: i64, y: i64) -> Result<i64, TerrainError> {
        let max_x = (self.vertices_x as i64 - 1) * self.cell_size;
        let max_y = (self.vertices_y as i64 - 1) * self.cell_size;
        if x < 0 || y < 0 || x > max_x || y > max_y {
            return Err(TerrainError::OutOfBounds);
        }
        // The domain is closed at the far edges: clamp so max_x / max_y
        // fall inside the last column/row of cells.
        let cell_x = (x / self.cell_size).min(self.vertices_x as i64 - 2);
        let cell_y = (y / self.cell_size).min(self.vertices_y as i64 - 2);
        let fx = (x - cell_x * self.cell_size) as f64 / self.cell_size as f64;
        let fy = (y - cell_y * self.cell_size) as f64 / self.cell_size as f64;
        let sw = self.sample(cell_x, cell_y);
        let se = self.sample(cell_x + 1, cell_y);
        let nw = self.sample(cell_x, cell_y + 1);
        let ne = self.sample(cell_x + 1, cell_y + 1);
        // fx ≥ fy is the triangle south-east of the SW→NE diagonal
        // (vertices SW, SE, NE); fx < fy the north-west one (SW, NW, NE).
        // Each expression is the plane through its three vertices; on the
        // diagonal itself they agree, so the boundary is seamless.
        let z = if fx >= fy {
            sw + fx * (se - sw) + fy * (ne - se)
        } else {
            sw + fx * (ne - nw) + fy * (nw - sw)
        };
        Ok(z.round() as i64)
    }

    /// Stored sample at vertex (vx, vy), as f64 for interpolation.
    fn sample(&self, vx: i64, vy: i64) -> f64 {
        self.elevations[(vy * self.vertices_x as i64 + vx) as usize] as f64
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test terrain`
Expected: all pass (12 including Task 1's).

- [ ] **Step 5: Commit**

```bash
cargo fmt && cargo test
git add src/terrain.rs
git commit -m "feat: Terrain heightmap with triangulated elevation_at"
```

---

### Task 3: `grade`

**Files:**
- Modify: `src/terrain.rs`
- Test: same file's `mod tests`

**Interfaces:**
- Consumes: `Terrain::elevation_at`, `TerrainError` (Task 2).
- Produces: `pub fn grade(terrain: &Terrain, from: (i64, i64), to: (i64, i64)) -> Result<f64, TerrainError>`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn grade_is_positive_uphill_and_negative_downhill() {
    // Elevations [0, 10, 0, 10]: rises 10 over run 10 along +x.
    let t = Terrain::new(2, 2, 10, vec![0, 10, 0, 10]);
    let up = grade(&t, (0, 0), (10, 0)).unwrap();
    let down = grade(&t, (10, 0), (0, 0)).unwrap();
    assert!((up - 1.0).abs() < 1e-12);
    assert!((down + 1.0).abs() < 1e-12);
}

#[test]
fn grade_rejects_zero_run_and_out_of_bounds() {
    let t = ramp();
    assert_eq!(grade(&t, (5, 5), (5, 5)), Err(TerrainError::ZeroRun));
    assert_eq!(grade(&t, (0, 0), (99, 0)), Err(TerrainError::OutOfBounds));
    assert_eq!(grade(&t, (-1, 0), (5, 5)), Err(TerrainError::OutOfBounds));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test terrain`
Expected: FAIL to compile — `grade` not defined.

- [ ] **Step 3: Implement**

```rust
/// Signed grade (rise/run) of the straight ground segment `from → to`:
/// elevation difference over horizontal 2D distance. Uphill (to higher)
/// is positive. Pure; the f64 is transient — never stored. A free
/// function per the spec, like `travel_time` below.
pub fn grade(
    terrain: &Terrain,
    from: (i64, i64),
    to: (i64, i64),
) -> Result<f64, TerrainError> {
    // Checked before domain: a zero-run query is malformed regardless of
    // where the point lies.
    if from == to {
        return Err(TerrainError::ZeroRun);
    }
    let z_from = terrain.elevation_at(from.0, from.1)? as f64;
    let z_to = terrain.elevation_at(to.0, to.1)? as f64;
    let dx = (to.0 - from.0) as f64;
    let dy = (to.1 - from.1) as f64;
    let run = (dx * dx + dy * dy).sqrt();
    Ok((z_to - z_from) / run)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test terrain`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
cargo fmt && cargo test
git add src/terrain.rs
git commit -m "feat: signed terrain grade between ground points"
```

---

### Task 4: `SpeedProfile`, tuning constants, `travel_time`

**Files:**
- Modify: `src/terrain.rs`
- Test: same file's `mod tests`

**Interfaces:**
- Consumes: `grade`, `Terrain::elevation_at` (Tasks 2–3).
- Produces: `pub struct SpeedProfile { pub flat_speed: u32 }`; `pub const WAGON: SpeedProfile`; `pub const WAGON_FLAT_SPEED: u32 = 50`; `pub const UPHILL_DRAG: f64 = 8.0`; `pub const DOWNHILL_BOOST: f64 = 2.0`; `pub const DOWNHILL_CAP: f64 = 1.25`; `pub fn travel_time(terrain: &Terrain, from: (i64, i64), to: (i64, i64), profile: &SpeedProfile) -> Result<u64, TerrainError>`.

- [ ] **Step 1: Write the failing tests**

Segments are deliberately long (cell_size 1000) so tick counts don't collapse under `ceil` — the spec requires tests to show the asymmetry this way.

```rust
/// 10% grade along +x over a 1000-unit run; flat along y at x=0.
fn hill() -> Terrain {
    Terrain::new(2, 2, 1000, vec![0, 100, 0, 100])
}

#[test]
fn travel_time_uphill_flat_downhill_asymmetry() {
    let t = hill();
    // Uphill: g=0.1 → speed 50/1.8 ≈ 27.78; dist √(1000²+100²) ≈ 1004.99 → 37.
    let uphill = travel_time(&t, (0, 0), (1000, 0), &WAGON).unwrap();
    // Flat: 1000/50 = 20 exactly.
    let flat = travel_time(&t, (0, 0), (0, 1000), &WAGON).unwrap();
    // Downhill: speed min(50·1.2, 62.5) = 60 → ≈ 16.75 → 17.
    let downhill = travel_time(&t, (1000, 0), (0, 0), &WAGON).unwrap();
    assert_eq!(uphill, 37);
    assert_eq!(flat, 20);
    assert_eq!(downhill, 17);
    assert!(uphill > flat && flat > downhill);
}

#[test]
fn travel_time_caps_downhill_speed() {
    // 50% grade: uncapped boost would be 50·2 = 100; cap holds it at 62.5.
    let t = Terrain::new(2, 2, 1000, vec![0, 500, 0, 500]);
    // dist √(1000²+500²) ≈ 1118.03 → /62.5 ≈ 17.89 → 18 (uncapped would be 12).
    assert_eq!(travel_time(&t, (1000, 0), (0, 0), &WAGON), Ok(18));
    // Same pair uphill: speed 50/5 = 10 → 112.
    assert_eq!(travel_time(&t, (0, 0), (1000, 0), &WAGON), Ok(112));
}

#[test]
fn travel_time_is_at_least_one_tick() {
    let t = Terrain::new(2, 2, 1000, vec![0, 0, 0, 0]);
    assert_eq!(travel_time(&t, (0, 0), (1, 0), &WAGON), Ok(1));
}

#[test]
fn travel_time_propagates_errors() {
    let t = hill();
    assert_eq!(travel_time(&t, (5, 5), (5, 5), &WAGON), Err(TerrainError::ZeroRun));
    assert_eq!(
        travel_time(&t, (0, 0), (9999, 0), &WAGON),
        Err(TerrainError::OutOfBounds)
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test terrain`
Expected: FAIL to compile — `travel_time`, `WAGON` not defined.

- [ ] **Step 3: Implement**

```rust
// ── Travel tuning constants ─────────────────────────────────────────────
// Same status as the pricing table in `market.rs`: first-guess values,
// retuning is a constants edit. NOTE for the future hauler spec: whether
// these ticks are economy ticks or finer movement sub-steps is that
// spec's question — don't let the unit silently harden (spec, "Travel
// tuning constants").

/// Baseline wagon speed on flat ground, world units per tick.
pub const WAGON_FLAT_SPEED: u32 = 50;
/// Uphill speed divisor grows as 1 + UPHILL_DRAG·grade.
pub const UPHILL_DRAG: f64 = 8.0;
/// Downhill speed multiplier grows as 1 + DOWNHILL_BOOST·|grade|.
pub const DOWNHILL_BOOST: f64 = 2.0;
/// Downhill speed never exceeds this multiple of flat speed.
pub const DOWNHILL_CAP: f64 = 1.25;

/// How fast something moves on flat ground; the drag/boost/cap constants
/// above apply to every profile. The seam future hauler/wagon mechanics
/// extend (more fields as needed — load, mode, …).
pub struct SpeedProfile {
    /// Flat-ground speed in world units per tick.
    pub flat_speed: u32,
}

/// The standard wagon.
pub const WAGON: SpeedProfile = SpeedProfile { flat_speed: WAGON_FLAT_SPEED };

/// Ticks to travel the straight ground segment `from → to` (single
/// segment — no pathfinding): 3D distance between the ground points,
/// divided by grade-adjusted speed, `ceil`ed to whole ticks (≥ 1). The
/// asymmetry invariant holds on effective speed pre-quantization:
/// speed(uphill) < speed(flat) ≤ speed(downhill) ≤ cap·flat.
pub fn travel_time(
    terrain: &Terrain,
    from: (i64, i64),
    to: (i64, i64),
    profile: &SpeedProfile,
) -> Result<u64, TerrainError> {
    let g = grade(terrain, from, to)?;
    let z_from = terrain.elevation_at(from.0, from.1)? as f64;
    let z_to = terrain.elevation_at(to.0, to.1)? as f64;
    let dx = (to.0 - from.0) as f64;
    let dy = (to.1 - from.1) as f64;
    let dz = z_to - z_from;
    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
    let flat = profile.flat_speed as f64;
    let speed = if g > 0.0 {
        flat / (1.0 + UPHILL_DRAG * g)
    } else if g < 0.0 {
        (flat * (1.0 + DOWNHILL_BOOST * g.abs())).min(DOWNHILL_CAP * flat)
    } else {
        flat
    };
    Ok((dist / speed).ceil() as u64)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test terrain`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
cargo fmt && cargo test
git add src/terrain.rs
git commit -m "feat: grade-aware travel_time with wagon speed profile"
```

---

### Task 5: `Terrain::generate` — deterministic value noise

**Files:**
- Modify: `src/terrain.rs`
- Test: same file's `mod tests`

**Interfaces:**
- Consumes: `Terrain::new` (Task 2).
- Produces: `pub fn generate(seed: u64, vertices_x: u32, vertices_y: u32, cell_size: i64) -> Terrain` (associated function on `Terrain`). Internals (`value_noise`, `lattice_value`, `smoothstep`, `NOISE_OCTAVES`) are private.

- [ ] **Step 1: Write the failing tests**

Tests may read `Terrain`'s private fields directly — `mod tests` lives in the same module.

```rust
#[test]
fn generate_is_deterministic() {
    let a = Terrain::generate(20260728, 64, 64, 50);
    let b = Terrain::generate(20260728, 64, 64, 50);
    assert_eq!(a.elevations, b.elevations);
}

#[test]
fn generate_seeds_differ() {
    let a = Terrain::generate(1, 64, 64, 50);
    let b = Terrain::generate(2, 64, 64, 50);
    assert_ne!(a.elevations, b.elevations);
}

#[test]
fn generate_stays_within_elevation_bounds() {
    let t = Terrain::generate(20260728, 64, 64, 50);
    assert!(t.elevations.iter().all(|&e| (0..=400).contains(&e)));
}

#[test]
fn generate_has_visible_relief_at_shipped_size() {
    // "Rolling hills and at least one valley" is verified by eye in the
    // viewer; this pins the mechanical part — real vertical spread.
    let t = Terrain::generate(20260728, 64, 64, 50);
    let max = t.elevations.iter().max().unwrap();
    let min = t.elevations.iter().min().unwrap();
    assert!(max - min >= 100, "relief too flat: {min}..{max}");
}

#[test]
#[should_panic(expected = "2x2")]
fn generate_rejects_degenerate_dims() {
    Terrain::generate(0, 1, 64, 50);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test terrain`
Expected: FAIL to compile — `generate` not defined.

- [ ] **Step 3: Implement**

Add inside `impl Terrain`:

```rust
    /// Deterministic, dependency-free terrain: layered value noise from an
    /// integer hash lattice. Identical inputs give an identical `Terrain`
    /// on every platform — arithmetic and integer/hash ops only, no libm
    /// (spec, `generate` contract). Elevations land in [0, 400] units
    /// (0–40 m). Panics on dims < 2×2 or cell_size ≤ 0 — worldgen
    /// programmer error, matching `template_world`'s style.
    pub fn generate(seed: u64, vertices_x: u32, vertices_y: u32, cell_size: i64) -> Terrain {
        assert!(
            vertices_x >= 2 && vertices_y >= 2,
            "terrain needs at least a 2x2 vertex grid"
        );
        assert!(cell_size > 0, "cell_size must be positive");
        let total_amplitude: f64 = NOISE_OCTAVES.iter().map(|(_, a)| a).sum();
        let mut elevations = Vec::with_capacity(vertices_x as usize * vertices_y as usize);
        for vy in 0..vertices_y {
            for vx in 0..vertices_x {
                let mut n = 0.0;
                for (octave, (period, amplitude)) in NOISE_OCTAVES.iter().enumerate() {
                    n += amplitude * value_noise(seed, octave as u64, vx, vy, *period);
                }
                elevations.push((n / total_amplitude * MAX_ELEVATION).round() as i64);
            }
        }
        Terrain::new(vertices_x, vertices_y, cell_size, elevations)
    }
```

Add at module level (near the travel constants):

```rust
// ── Terrain generation ──────────────────────────────────────────────────

/// Highest generated elevation, world units (400 = 40 m).
const MAX_ELEVATION: f64 = 400.0;

/// Noise layers as (lattice period in vertices, amplitude). The base
/// 16-vertex period gives ~4 hills across the shipped 64-vertex map;
/// smaller octaves add detail. Tuning these reshapes the land.
const NOISE_OCTAVES: [(u32, f64); 3] = [(16, 1.0), (8, 0.5), (4, 0.25)];

/// One octave of value noise at vertex (vx, vy): hash the four
/// surrounding lattice corners to values in [0, 1) and blend bilinearly
/// with a smoothstep fade. Polynomials only — no libm.
fn value_noise(seed: u64, octave: u64, vx: u32, vy: u32, period: u32) -> f64 {
    let ix = vx / period;
    let iy = vy / period;
    let sx = smoothstep((vx % period) as f64 / period as f64);
    let sy = smoothstep((vy % period) as f64 / period as f64);
    let v00 = lattice_value(seed, octave, ix, iy);
    let v10 = lattice_value(seed, octave, ix + 1, iy);
    let v01 = lattice_value(seed, octave, ix, iy + 1);
    let v11 = lattice_value(seed, octave, ix + 1, iy + 1);
    let south = v00 + sx * (v10 - v00);
    let north = v01 + sx * (v11 - v01);
    south + sy * (north - south)
}

/// The classic cubic fade t²(3 − 2t): eases lattice blending so hills
/// roll instead of crease.
fn smoothstep(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

/// Hashes (seed, octave, lattice point) to a uniform value in [0, 1)
/// with a splitmix64-style avalanche. Pure integer ops, so identical on
/// every platform.
fn lattice_value(seed: u64, octave: u64, ix: u32, iy: u32) -> f64 {
    let mut h = seed
        ^ octave.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (ix as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ (iy as u64).wrapping_mul(0x1656_67B1_9E37_79F9);
    h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    (h >> 11) as f64 / (1u64 << 53) as f64
}
```

If `generate_has_visible_relief_at_shipped_size` fails, adjust `NOISE_OCTAVES` amplitudes/periods until it passes — that's inside the contract ("visible relief"), not a design change.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test terrain`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
cargo fmt && cargo test
git add src/terrain.rs
git commit -m "feat: deterministic value-noise terrain generator"
```

---

### Task 6: `to_json`

**Files:**
- Modify: `src/terrain.rs`
- Test: same file's `mod tests`

**Interfaces:**
- Consumes: `Terrain` (Task 2).
- Produces: `pub fn to_json(&self) -> String` on `Terrain`. The output string is the viewer's input contract — viewer (Task 8) and this serializer change together.

- [ ] **Step 1: Write the failing golden test**

```rust
#[test]
fn to_json_matches_the_viewer_contract_exactly() {
    let t = Terrain::new(2, 2, 10, vec![0, 10, 20, 40]);
    assert_eq!(
        t.to_json(),
        r#"{"unit_meters":0.1,"cell_size":10,"vertices_x":2,"vertices_y":2,"elevations":[0,10,20,40]}"#
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test terrain`
Expected: FAIL to compile — `to_json` not defined.

- [ ] **Step 3: Implement**

Add inside `impl Terrain`:

```rust
    /// Hand-serialized flat JSON for `tools/map_viewer.html` — its input
    /// contract; the viewer and this format change together. No serde by
    /// design (no new dependencies).
    pub fn to_json(&self) -> String {
        let elevations = self
            .elevations
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"unit_meters\":0.1,\"cell_size\":{},\"vertices_x\":{},\"vertices_y\":{},\"elevations\":[{}]}}",
            self.cell_size, self.vertices_x, self.vertices_y, elevations
        )
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test terrain`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
cargo fmt && cargo test
git add src/terrain.rs
git commit -m "feat: hand-serialized terrain JSON export"
```

---

### Task 7: the `map` shell command

**Files:**
- Modify: `src/engine/game_loop.rs:18-26` (`Command`), `:30-48` (`run`), `:199-213` (`read_command`), plus a new `export_map` fn.
- Test: none (interactive I/O — the presentation layer has no unit tests, per project convention); scripted smoke check below.

**Interfaces:**
- Consumes: `Terrain::generate`, `Terrain::to_json` (Tasks 5–6).
- Produces: shell command `map`; constants `MAP_SEED: u64 = 20260728`, `MAP_VERTICES: u32 = 64`, `MAP_CELL_SIZE: i64 = 50` in `game_loop.rs`.

- [ ] **Step 1: Wire the command**

Add to the imports in `src/engine/game_loop.rs`:

```rust
use crate::terrain::Terrain;
```

Add constants after the imports:

```rust
/// The shell's display terrain: generated once at startup, held alongside
/// — not inside — the economy `World` (no sim consumer yet). Fixed seed
/// so every run shows the same land. 64×64 vertices at cell 50 is a
/// ~320 m square sampled every 5 m (spec, `generate` contract).
const MAP_SEED: u64 = 20260728;
const MAP_VERTICES: u32 = 64;
const MAP_CELL_SIZE: i64 = 50;
```

Extend `Command`:

```rust
    /// `map` (any case): export the terrain to map.json.
    Map,
```

In `run()`, generate the terrain before the loop and handle the command:

```rust
    let mut world = template_world();
    let terrain = Terrain::generate(MAP_SEED, MAP_VERTICES, MAP_VERTICES, MAP_CELL_SIZE);
    let mut tick_count: u64 = 0;
```

and in the `match`:

```rust
            Command::Map => export_map(&terrain),
```

In `read_command`, add an arm above the `name =>` fallback (mirroring the `q` arm; note an agent named "map" becomes uninspectable — acceptable for a debug command):

```rust
            map if map.eq_ignore_ascii_case("map") => Command::Map,
```

and update the prompt line:

```rust
    print!("[tick {tick_count}] Enter = advance · <agent name> = inspect · map = export map.json · q = quit > ");
```

Add the handler (modeled on `inspect`'s wait-for-Enter so the next clear-screen doesn't wipe the message):

```rust
/// Writes the terrain to map.json in the working directory. A write
/// failure prints an error and the sim continues (spec, map export
/// contract).
fn export_map(terrain: &Terrain) {
    match std::fs::write("map.json", terrain.to_json()) {
        Ok(()) => println!("wrote map.json — open tools/map_viewer.html and load it"),
        Err(error) => println!("could not write map.json: {error}"),
    }
    print!("press Enter to continue... ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
}
```

- [ ] **Step 2: Scripted smoke check**

Run:

```bash
rm -f map.json
printf 'map\n\nq\n' | cargo run
head -c 60 map.json && echo
```

Expected: `map.json` exists and starts with `{"unit_meters":0.1,"cell_size":50,"vertices_x":64,`.

- [ ] **Step 3: Full test suite**

Run: `cargo test`
Expected: all pass (nothing in the sim changed).

- [ ] **Step 4: Commit**

```bash
cargo fmt && cargo test
git add src/engine/game_loop.rs
git commit -m "feat: map shell command exports terrain to map.json"
```

---

### Task 8: `tools/map_viewer.html`

**Files:**
- Create: `tools/map_viewer.html`
- Test: none (verified by opening in a browser, per spec).

**Interfaces:**
- Consumes: the exact JSON format from Task 6.
- Produces: a self-contained viewer page — human verification tool only; nothing in the codebase depends on it.

- [ ] **Step 1: Write the file**

Create `tools/map_viewer.html` with exactly this content (single file, no external resources; hillshaded isometric canvas, drag to rotate, wheel to zoom):

```html
<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>CityProj map viewer</title>
<style>
  body { margin: 0; font-family: system-ui, sans-serif; background: #14141a; color: #ccc; overflow: hidden; }
  #bar { padding: 8px 12px; position: absolute; z-index: 1; }
  canvas { display: block; cursor: grab; }
</style>
</head>
<body>
<div id="bar">
  <input type="file" id="file" accept=".json,application/json">
  <span>load map.json · drag = rotate · wheel = zoom</span>
</div>
<canvas id="view"></canvas>
<script>
"use strict";
// Input contract: {"unit_meters":0.1,"cell_size":C,"vertices_x":X,
// "vertices_y":Y,"elevations":[...]} row-major — produced by
// Terrain::to_json; this file and that serializer change together.
const canvas = document.getElementById("view");
const ctx = canvas.getContext("2d");
let map = null;
let angle = Math.PI / 4;   // rotation of the map around the vertical axis
let zoom = 1;
const Z_EXAGGERATION = 2;  // display-only vertical stretch so relief reads
const LIGHT = normalize([-0.5, -0.7, 0.8]); // fixed in world space

function normalize(v) {
  const len = Math.hypot(v[0], v[1], v[2]);
  return [v[0] / len, v[1] / len, v[2] / len];
}

document.getElementById("file").addEventListener("change", (event) => {
  const file = event.target.files[0];
  if (!file) return;
  file.text().then((text) => {
    map = JSON.parse(text);
    fitZoom();
    draw();
  });
});

function elevation(vx, vy) {
  return map.elevations[vy * map.vertices_x + vx];
}

// Map-centered world coordinates of a vertex, rotated by `angle`.
function rotated(vx, vy) {
  const x = (vx - (map.vertices_x - 1) / 2) * map.cell_size;
  const y = (vy - (map.vertices_y - 1) / 2) * map.cell_size;
  const cos = Math.cos(angle), sin = Math.sin(angle);
  return [x * cos - y * sin, x * sin + y * cos];
}

// Isometric projection: screen x from the rotated difference, screen y
// from the rotated sum minus exaggerated elevation.
function project(r, z) {
  return [
    (r[0] - r[1]) * 0.866 * zoom + canvas.width / 2,
    (r[0] + r[1]) * 0.5 * zoom - z * Z_EXAGGERATION * zoom + canvas.height / 2,
  ];
}

function fitZoom() {
  const span = Math.max(map.vertices_x, map.vertices_y) * map.cell_size;
  zoom = Math.min(canvas.width, canvas.height) / (span * 2.2);
}

// Lowland green → highland grey-brown, darkened by hillshade.
function cellColor(z, shade) {
  const t = Math.min(z / 400, 1);
  const r = 70 + t * 110, g = 110 + t * 40, b = 60 + t * 60;
  const s = 0.35 + 0.65 * shade;
  return `rgb(${Math.round(r * s)},${Math.round(g * s)},${Math.round(b * s)})`;
}

function draw() {
  ctx.fillStyle = "#14141a";
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  if (!map) return;

  // Painter's algorithm: sort cells far-to-near. Depth in this projection
  // is the rotated coordinate sum (larger = nearer the viewer).
  const cells = [];
  for (let vy = 0; vy < map.vertices_y - 1; vy++) {
    for (let vx = 0; vx < map.vertices_x - 1; vx++) {
      const r = rotated(vx + 0.5, vy + 0.5);
      cells.push({ vx, vy, depth: r[0] + r[1] });
    }
  }
  cells.sort((a, b) => a.depth - b.depth);

  for (const { vx, vy } of cells) {
    const z00 = elevation(vx, vy), z10 = elevation(vx + 1, vy);
    const z01 = elevation(vx, vy + 1), z11 = elevation(vx + 1, vy + 1);
    // Hillshade from the cell's average gradient, in unrotated world
    // space so the sun stays fixed to the terrain while the view spins.
    const dzdx = (z10 + z11 - z00 - z01) / (2 * map.cell_size);
    const dzdy = (z01 + z11 - z00 - z10) / (2 * map.cell_size);
    const n = normalize([-dzdx, -dzdy, 1]);
    const shade = Math.max(0, n[0] * LIGHT[0] + n[1] * LIGHT[1] + n[2] * LIGHT[2]);

    const p00 = project(rotated(vx, vy), z00);
    const p10 = project(rotated(vx + 1, vy), z10);
    const p11 = project(rotated(vx + 1, vy + 1), z11);
    const p01 = project(rotated(vx, vy + 1), z01);
    ctx.beginPath();
    ctx.moveTo(p00[0], p00[1]);
    ctx.lineTo(p10[0], p10[1]);
    ctx.lineTo(p11[0], p11[1]);
    ctx.lineTo(p01[0], p01[1]);
    ctx.closePath();
    ctx.fillStyle = cellColor((z00 + z10 + z01 + z11) / 4, shade);
    ctx.fill();
  }
}

let dragging = false, lastX = 0;
canvas.addEventListener("mousedown", (e) => { dragging = true; lastX = e.clientX; });
window.addEventListener("mouseup", () => { dragging = false; });
window.addEventListener("mousemove", (e) => {
  if (!dragging) return;
  angle += (e.clientX - lastX) * 0.01;
  lastX = e.clientX;
  draw();
});
canvas.addEventListener("wheel", (e) => {
  e.preventDefault();
  zoom *= Math.exp(-e.deltaY * 0.001);
  draw();
}, { passive: false });

function resize() {
  canvas.width = window.innerWidth;
  canvas.height = window.innerHeight;
  if (map) fitZoom();
  draw();
}
window.addEventListener("resize", resize);
resize();
</script>
</body>
</html>
```

- [ ] **Step 2: Human verification checkpoint (real browser — do not skip)**

Export a fresh `map.json` (Task 7's smoke command), open `tools/map_viewer.html` in a browser, load the file, and confirm: hillshaded terrain with rolling hills and at least one visible valley; dragging rotates; the wheel zooms. If running headless, stop and ask the human to verify — a type-check is not feature verification for UI.

- [ ] **Step 3: Commit**

```bash
git add tools/map_viewer.html
git commit -m "feat: self-contained isometric terrain viewer"
```

---

### Task 9: Docs and final verification

**Files:**
- Modify: `CLAUDE.md` (state section + Roadmap)
- Test: full verification sweep.

**Interfaces:**
- Consumes: everything above.
- Produces: docs matching shipped reality.

- [ ] **Step 1: Update CLAUDE.md**

In the "Current code state vs. target layout" bullet list, after the `src/sim.rs` entry, add:

```markdown
- `src/terrain.rs` — world coordinates (`Point3`, 1 unit = 0.1 m) and the
  triangulated integer heightmap (`Terrain`, `elevation_at`); pure movement
  math (`grade`, `travel_time` + `SpeedProfile`) with its tuning constants
  alongside; deterministic value-noise `generate`; `to_json` for the
  viewer. No in-sim consumer yet — the shell holds a display terrain and
  the `map` command exports `map.json` for `tools/map_viewer.html`
  (self-contained, open in a browser).
```

In the `src/engine/game_loop.rs` entry, extend the parenthetical shell description: change "interactive shell (Enter advances a tick, an agent name inspects it, q quits)" to "interactive shell (Enter advances a tick, an agent name inspects it, `map` exports map.json, q quits)".

In the Roadmap section, before the "Ideas parking lot" bullet, add:

```markdown
- **Building volumes.** Buildings will occupy 3D volumes (footprint +
  vertical extent, above or below ground) addressed by `Point3` — never
  tile-locked to the terrain grid, which only stores the ground surface.
```

- [ ] **Step 2: Full verification sweep — quote real output**

```bash
cargo fmt
cargo check
cargo clippy
cargo test
```

Expected: clean check, no clippy warnings, all tests pass. Then the Task 7 smoke command once more to confirm the shell still works end to end.

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: record terrain module and map viewer in CLAUDE.md"
```
