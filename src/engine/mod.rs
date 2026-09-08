//! The interactive front-end. Simulation logic lives in the top-level
//! modules (`sim`, `world`, `money`, …) — this layer only presents it.

pub mod game_loop;
/// The frozen pre-cure soak baseline — test-only, and deliberately a
/// separate file: it must never be edited to track a re-cut cured soak.
#[cfg(test)]
pub mod null_baseline;
pub mod worldgen;
