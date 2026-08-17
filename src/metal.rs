//! The coinage metals (07-12 spec). [`Metal`] is the orthogonal key that
//! scopes every balance, conservation total, and audit assertion in the
//! trusted money core — [`crate::money::Money`] itself stays one
//! currency-agnostic scalar (§8.1) and never learns which metal it
//! denominates.

use std::fmt;

/// One of the three coinage metals. Keys `Accounts`' books as
/// `(AgentId, Metal)`. Adding a metal later means extending this enum and
/// [`Metal::ALL`], then letting the compiler find every match arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Metal {
    Gold,
    Silver,
    Copper,
}

impl Metal {
    /// Every metal, hand-written — no enum-iteration crate, per the
    /// zero-dep convention. The audit iterates this; adding a metal here
    /// puts it under conservation automatically.
    pub const ALL: [Metal; 3] = [Metal::Gold, Metal::Silver, Metal::Copper];
}

impl fmt::Display for Metal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Metal::Gold => "gold",
            Metal::Silver => "silver",
            Metal::Copper => "copper",
        };
        write!(f, "{name}")
    }
}
