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
