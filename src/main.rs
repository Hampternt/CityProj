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
//! - [`housing`] — [`housing::House`], first-class places agents live and
//!   (later) work in.
//! - [`role`] — [`role::Role`], the closed set of job roles.
//! - [`world`] — [`world::World`] bundles agents, houses, and accounts, and
//!   reserves the Mint and External account ids.
//! - [`sim`] — [`sim::tick`], the fixed 9-phase tick; mechanics land inside
//!   phases, never around them.
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
mod engine;
mod housing;
mod money;
mod role;
mod sim;
mod world;

/// Entry point: hands control to the interactive shell in
/// [`engine::game_loop`].
fn main() {
    engine::game_loop::run();
}
