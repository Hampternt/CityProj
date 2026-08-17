//! CityProj — a noded market-economy simulation with strictly conserved money.
//!
//! Simulates an economy from the household level up. Money is integer-only,
//! moves solely through the trusted core in [`money`], and is audited for
//! conservation every tick. The architecture reference is
//! `docs/superpowers/specs/2026-06-20-economy-sim-design.md`; §-references in
//! doc comments point into it.
//!
//! # Module map
//!
//! - [`money`] — the trusted core (§8): [`money::Accounts`] holds every
//!   balance; `transfer` / `mint` / `burn` are the only mutators and `audit`
//!   panics on any conservation violation.
//! - [`agent`] — [`agent::Agent`], the person-level economic actor.
//! - [`business`] — [`business::Business`], a house's employer capability;
//!   account-only money-wise (no balance field).
//! - [`goods`] — [`goods::Good`], the closed set of consumables and their
//!   per-good constants.
//! - [`housing`] — [`housing::House`], first-class places agents live and
//!   (later) work in.
//! - [`market`] — the goods market's pure shopping logic (§8.6);
//!   [`market::plan_purchases`] turns wallet + inventory + offers into a
//!   purchase plan.
//! - [`metal`] — [`metal::Metal`], the coinage metals that key every
//!   balance and conservation total in the money core (07-12 spec).
//! - [`role`] — [`role::Role`], the closed set of job roles.
//! - [`world`] — [`world::World`] bundles agents, houses, and accounts, and
//!   reserves the Mint and External account ids.
//! - [`sim`] — [`sim::tick`], the fixed 9-phase tick; mechanics land inside
//!   phases, never around them.
//! - [`terrain`] — world coordinates and the ground heightmap; pure
//!   movement math (grade, travel time) with its tuning constants
//!   alongside, plus the deterministic terrain generator.
//! - [`engine`] — the interactive shell; presentation only, no simulation
//!   behavior.
//!
//! # Hard invariants (§8)
//!
//! Integer money only; a single mutation chokepoint in [`money::Accounts`];
//! a conservation audit every tick (`total == minted − burned`, panics on
//! imbalance); mint as the only faucet and burn the only sink; no overdraft.
//! Breaking one of these is a bug even if tests pass.
//!
//! Generate this documentation with `cargo doc --open` — as a binary crate,
//! private items are included automatically.

mod agent;
mod business;
mod engine;
mod goods;
mod housing;
mod market;
mod metal;
mod money;
mod role;
mod sim;
// A pure-math foundation with no in-sim consumer yet (07-28 spec): only the
// shell's `map` export calls into it. Drop the allow when mechanics do.
#[allow(dead_code)]
mod terrain;
mod world;

/// Entry point: hands control to the interactive shell in
/// [`engine::game_loop`].
fn main() {
    engine::game_loop::run();
}
