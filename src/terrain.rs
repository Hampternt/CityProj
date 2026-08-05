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
