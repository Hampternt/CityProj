//! The trusted money core (§8). All money movement goes through `Accounts`:
//! `transfer`, `mint`, and `burn` are the only mutators (§8.2), and `audit`
//! panics the sim on any conservation violation (§8.3).

// The full §8.2 API ships before any mechanic calls it; tests exercise it
// until the first mechanic does. Remove once the movers have real callers.
#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt;

use crate::agent::AgentId;

/// An amount of money in the smallest indivisible unit (§8.1 — never a
/// float). All arithmetic is checked; overflow panics explicitly rather
/// than wrapping silently.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Money(u64);

impl Money {
    pub const ZERO: Money = Money(0);

    pub const fn new(amount: u64) -> Self {
        Money(amount)
    }

    fn plus(self, other: Money) -> Money {
        Money(self.0.checked_add(other.0).expect("money overflow"))
    }

    fn minus(self, other: Money) -> Money {
        Money(self.0.checked_sub(other.0).expect("money underflow"))
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MoneyError {
    InsufficientFunds,
}

/// The single store of all balances (§8.2). `balances` is private; the only
/// public mutators are `transfer`, `mint`, and `burn`.
#[derive(Debug, Default)]
pub struct Accounts {
    balances: HashMap<AgentId, Money>,
    total_minted: Money,
    total_burned: Money,
}

impl Accounts {
    pub fn new() -> Self {
        Self::default()
    }

    /// Read-only. Unknown id reads as zero — accounts are created implicitly
    /// at first credit.
    pub fn balance_of(&self, id: AgentId) -> Money {
        self.balances.get(&id).copied().unwrap_or(Money::ZERO)
    }

    /// Sum of ALL balances, including External.
    pub fn total_money(&self) -> Money {
        self.balances.values().fold(Money::ZERO, |sum, &b| sum.plus(b))
    }

    pub fn total_minted(&self) -> Money {
        self.total_minted
    }

    pub fn total_burned(&self) -> Money {
        self.total_burned
    }

    /// §8.4: the ONLY way money is created. Gold-reserve cap deferred — spec
    /// amendment needed when the mint job arrives.
    pub fn mint(&mut self, to: AgentId, amount: Money) {
        let balance = self.balance_of(to);
        self.balances.insert(to, balance.plus(amount));
        self.total_minted = self.total_minted.plus(amount);
    }

    /// §8.3: asserts conservation, PANICS on imbalance — by design, never
    /// softened to a `Result`. Initial supply is zero (no genesis), so
    /// circulating money must equal minted − burned exactly.
    pub fn audit(&self) {
        let expected = self
            .total_minted
            .0
            .checked_sub(self.total_burned.0)
            .expect("audit failed: total_burned exceeds total_minted (§8.3)");
        assert_eq!(
            self.total_money(),
            Money(expected),
            "conservation audit failed: circulating money != minted - burned (§8.3)"
        );
    }

    /// §8.2/§8.5: moves money between accounts, or errs with NO state change.
    pub fn transfer(
        &mut self,
        from: AgentId,
        to: AgentId,
        amount: Money,
    ) -> Result<(), MoneyError> {
        if amount == Money::ZERO {
            return Ok(()); // no-op by contract: creates no account entry
        }
        let from_balance = self.balance_of(from);
        if from_balance < amount {
            return Err(MoneyError::InsufficientFunds); // §8.5 — nothing applied
        }
        if from == to {
            return Ok(()); // funds verified; debit + credit would cancel out
        }
        self.balances.insert(from, from_balance.minus(amount));
        let to_balance = self.balance_of(to);
        self.balances.insert(to, to_balance.plus(amount));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a() -> AgentId {
        AgentId(10)
    }

    fn b() -> AgentId {
        AgentId(11)
    }

    #[test]
    fn mint_credits_and_logs() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Money::new(100));
        assert_eq!(accounts.balance_of(a()), Money::new(100));
        assert_eq!(accounts.total_minted(), Money::new(100));
        assert_eq!(accounts.total_money(), Money::new(100));
        accounts.audit();
    }

    #[test]
    fn transfer_moves_exact_amount() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Money::new(100));
        accounts.transfer(a(), b(), Money::new(30)).unwrap();
        assert_eq!(accounts.balance_of(a()), Money::new(70));
        assert_eq!(accounts.balance_of(b()), Money::new(30));
        assert_eq!(accounts.total_money(), Money::new(100));
    }

    #[test]
    fn transfer_insufficient_funds_is_atomic() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Money::new(10));
        let result = accounts.transfer(a(), b(), Money::new(20));
        assert_eq!(result, Err(MoneyError::InsufficientFunds));
        // no partial application — nothing changed
        assert_eq!(accounts.balance_of(a()), Money::new(10));
        assert_eq!(accounts.balance_of(b()), Money::ZERO);
    }

    #[test]
    fn transfer_zero_is_noop() {
        let mut accounts = Accounts::new();
        accounts.transfer(a(), b(), Money::ZERO).unwrap();
        assert_eq!(accounts.total_money(), Money::ZERO);
        // creates no account entry (tests may touch private fields — same module)
        assert!(accounts.balances.is_empty());
    }

    #[test]
    fn transfer_to_self() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Money::new(50));
        accounts.transfer(a(), a(), Money::new(20)).unwrap();
        assert_eq!(accounts.balance_of(a()), Money::new(50));
    }
}
