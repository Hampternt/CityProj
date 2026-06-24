//! The game instance: the single value that holds the whole world state.
//!
//! Coming from JS, think of `GameState` as the object a "factory" would hand
//! back, and `GameState::new()` as that factory function. There is no class —
//! the struct is the data, the `impl` block holds the constructor.

use std::collections::HashMap;

/// A bag of goods keyed by good name, e.g. `{ "glass": 4, "mosaic": 3 }`.
///
/// Coming from JS this is the equivalent of a plain object used as a map:
/// the fields aren't fixed at compile time, so we use a `HashMap` instead of
/// named struct fields. Values are quantities (counts), not money.
pub type Goods = HashMap<String, u32>;

/// A single factory in the world. Placeholder test data for now.
///
/// Each factory turns `inputs` into `outputs` per unit, per tick. `count` is
/// how many copies of this factory exist, so its real yield is
/// `outputs × count`.
#[derive(Debug)]
pub struct Factory {
    pub name: String,
    pub count: u32,
    /// Goods consumed to run one factory for one tick. Empty for now, so
    /// nothing reads it yet — kept as the seam for input-driven production.
    #[allow(dead_code)]
    pub inputs: Goods,
    /// Goods produced by one factory in one tick, e.g. glass + mosaic.
    pub outputs: Goods,
}

/// The whole game state — the instance the game loop runs on.
#[derive(Debug)]
pub struct GameState {
    /// All factories in the world. Empty list to start filling in later.
    pub factories: Vec<Factory>,
}

impl GameState {
    /// Builds a fresh game instance with some placeholder test data.
    pub fn new() -> Self {
        Self {
            factories: vec![
                Factory {
                    name: "glass factory".to_string(),
                    count: 1,
                    inputs: Goods::new(), // none for now
                    // one glass factory makes 4 glass + 3 mosaic each tick
                    outputs: Goods::from([
                        ("glass".to_string(), 4),
                        ("mosaic".to_string(), 3),
                    ]),
                },
                Factory {
                    name: "steel factory".to_string(),
                    count: 2,
                    inputs: Goods::new(), // none for now
                    // each steel factory makes 5 steel; count: 2 → 10 per tick
                    outputs: Goods::from([("steel".to_string(), 5)]),
                },
            ],
        }
    }
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}
