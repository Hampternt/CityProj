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
        Terrain {
            vertices_x,
            vertices_y,
            cell_size,
            elevations,
        }
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
}

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
pub const WAGON: SpeedProfile = SpeedProfile {
    flat_speed: WAGON_FLAT_SPEED,
};

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

/// Signed grade (rise/run) of the straight ground segment `from → to`:
/// elevation difference over horizontal 2D distance. Uphill (to higher)
/// is positive. Pure; the f64 is transient — never stored. A free
/// function per the spec, like `travel_time` below.
pub fn grade(terrain: &Terrain, from: (i64, i64), to: (i64, i64)) -> Result<f64, TerrainError> {
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
        assert_eq!(
            travel_time(&t, (5, 5), (5, 5), &WAGON),
            Err(TerrainError::ZeroRun)
        );
        assert_eq!(
            travel_time(&t, (0, 0), (9999, 0), &WAGON),
            Err(TerrainError::OutOfBounds)
        );
    }

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
    fn generate_matches_viewer_canary() {
        // Pinned parity values shared with tools/map_viewer.html's startup
        // canary (spec, "parity canary" unit): if either side drifts from
        // the other, one of the two checks fails.
        let t = Terrain::generate(20260728, 64, 64, 50);
        // Vertex (0, 0) and vertex (33, 17) — world (1650, 850) at cell
        // size 50; elevation_at returns stored samples exactly at vertices.
        assert_eq!(t.elevation_at(0, 0), Ok(235));
        assert_eq!(t.elevation_at(1650, 850), Ok(183));
    }

    #[test]
    #[should_panic(expected = "2x2")]
    fn generate_rejects_degenerate_dims() {
        Terrain::generate(0, 1, 64, 50);
    }

    #[test]
    fn to_json_matches_the_viewer_contract_exactly() {
        let t = Terrain::new(2, 2, 10, vec![0, 10, 20, 40]);
        assert_eq!(
            t.to_json(),
            r#"{"unit_meters":0.1,"cell_size":10,"vertices_x":2,"vertices_y":2,"elevations":[0,10,20,40]}"#
        );
    }

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
