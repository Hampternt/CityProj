//! Job roles: the closed set of work an agent can be trained for or
//! employed in (refactor spec Amendment 12). Same closed-enum pattern as
//! multi-metal's `Metal`: add a variant + extend `ALL`, and the compiler
//! finds every match needing an update. Business archetypes are DATA
//! (combinations of these variants), never new types.

// Struct-only refactor: nothing reads roles yet. Remove once the labor
// market lands. Same rationale as money.rs's crate allow.
#![allow(dead_code)]

use std::fmt;

/// One kind of job. `Copy + Eq + Hash` so it keys `HashMap<Role, RoleSlot>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Engineer,
    Labourer,
}

impl Role {
    /// Every variant, hand-enumerated — zero-dep convention, same as
    /// `Metal::ALL`. Extend this when adding a variant.
    pub const ALL: [Role; 2] = [Role::Engineer, Role::Labourer];
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Role::Engineer => "engineer",
            Role::Labourer => "labourer",
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
        for role in Role::ALL {
            assert!(seen.insert(role), "duplicate in Role::ALL: {role:?}");
        }
        assert_eq!(seen.len(), 2);
    }

    #[test]
    fn display_is_lowercase_for_the_shell() {
        assert_eq!(Role::Engineer.to_string(), "engineer");
        assert_eq!(Role::Labourer.to_string(), "labourer");
    }
}
