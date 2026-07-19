//! Goods: the closed set of consumable products (07-19 minimal-needs
//! spec). Same closed-enum pattern as `Role`: add a variant + extend
//! `ALL` and the compiler finds every match needing an update. The
//! per-good constants are the scenario's tuning table — data on the
//! enum, tunable later. Unit *prices* are NOT here: price is a
//! `Business` field set at worldgen.

use std::fmt;

/// One kind of consumable. `Copy + Eq + Hash` so it keys inventories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Good {
    Food,
    Entertainment,
    Luxury,
}

impl Good {
    /// Every variant, hand-enumerated — same zero-dep convention as
    /// `Role::ALL`. Extend this when adding a variant.
    pub const ALL: [Good; 3] = [Good::Food, Good::Entertainment, Good::Luxury];

    /// Units one agent consumes per tick (phase 5).
    pub fn consumption_rate(self) -> u32 {
        match self {
            Good::Food => 10,
            Good::Entertainment => 5,
            Good::Luxury => 2,
        }
    }

    /// Shopping-priority weight: the market scores goods by
    /// `weight / (held + planned + 1)` (diminishing returns, §8.6).
    pub fn weight(self) -> u32 {
        match self {
            Good::Food => 100,
            Good::Entertainment => 30,
            Good::Luxury => 10,
        }
    }

    /// Stockpile target in days: agents stop buying a good at
    /// `target_days × consumption_rate` units.
    pub fn target_days(self) -> u32 {
        match self {
            Good::Food => 7,
            Good::Entertainment => 7,
            Good::Luxury => 7,
        }
    }

    /// Units a staffed producer adds to its stock per tick (phase 2).
    pub fn production_rate(self) -> u32 {
        match self {
            Good::Food => 40,
            Good::Entertainment => 20,
            Good::Luxury => 8,
        }
    }
}

impl fmt::Display for Good {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Good::Food => "food",
            Good::Entertainment => "entertainment",
            Good::Luxury => "luxury",
        };
        write!(f, "{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_lists_every_variant_exactly_once() {
        let mut seen = std::collections::HashSet::new();
        for good in Good::ALL {
            assert!(seen.insert(good), "duplicate in Good::ALL: {good:?}");
        }
        assert_eq!(seen.len(), 3);
    }

    #[test]
    fn constants_match_the_spec_table() {
        // 07-19 spec, "Per-good constants" table — change spec first, then this
        let table = [
            (Good::Food, 10, 100, 7, 40),
            (Good::Entertainment, 5, 30, 7, 20),
            (Good::Luxury, 2, 10, 7, 8),
        ];
        for (good, consumption, weight, days, production) in table {
            assert_eq!(good.consumption_rate(), consumption);
            assert_eq!(good.weight(), weight);
            assert_eq!(good.target_days(), days);
            assert_eq!(good.production_rate(), production);
        }
    }

    #[test]
    fn display_is_lowercase_for_the_shell() {
        assert_eq!(Good::Food.to_string(), "food");
        assert_eq!(Good::Entertainment.to_string(), "entertainment");
        assert_eq!(Good::Luxury.to_string(), "luxury");
    }
}
