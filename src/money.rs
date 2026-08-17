//! The trusted money core (§8). All money movement goes through [`Accounts`]:
//! [`Accounts::transfer`], [`Accounts::mint`], and [`Accounts::burn`] are the
//! only mutators (§8.2), and [`Accounts::audit`] panics the sim on any
//! conservation violation (§8.3).
//!
//! Nothing else in the crate may mutate balances — pricing, wages, and all
//! other economics live in their own modules and *call into* this one.
//!
//! ```ignore
//! let mut accounts = Accounts::new();
//! accounts.mint(alice, Metal::Gold, Money::new(100));          // the only faucet
//! accounts.transfer(alice, bob, Metal::Gold, Money::new(30))?; // errs, never overdrafts
//! accounts.burn(bob, Metal::Gold, Money::new(5))?;             // the only sink
//! accounts.audit();                                            // 95 == 100 − 5, or panic
//! ```

use std::collections::HashMap;
use std::fmt;

use crate::agent::AgentId;
use crate::metal::Metal;

/// An amount of money in the smallest indivisible unit (§8.1 — never a
/// float). All arithmetic is checked; overflow panics explicitly rather
/// than wrapping silently.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Money(u64);

impl Money {
    /// No money: the implicit balance of every account that has never been
    /// credited, and the amount that makes [`Accounts::transfer`] /
    /// [`Accounts::burn`] a no-op.
    pub const ZERO: Money = Money(0);

    /// Wraps an amount already expressed in the smallest unit. There is no
    /// conversion from floats or denominated units — by design (§8.1).
    pub const fn new(amount: u64) -> Self {
        Money(amount)
    }

    /// Checked addition; panics on `u64` overflow rather than wrapping.
    pub fn plus(self, other: Money) -> Money {
        Money(self.0.checked_add(other.0).expect("money overflow"))
    }

    /// Checked subtraction; panics on underflow — callers verify funds first.
    pub fn minus(self, other: Money) -> Money {
        Money(self.0.checked_sub(other.0).expect("money underflow"))
    }

    /// Checked multiplication by a unit count (`price × units`); panics
    /// on `u64` overflow rather than wrapping.
    pub fn times(self, count: u32) -> Money {
        Money(self.0.checked_mul(count as u64).expect("money overflow"))
    }

    /// Checked integer division, flooring — the proportional-step
    /// helper for pricing (§8.1: stays integer). Panics on a zero
    /// divisor; callers pass literal constants.
    pub fn divided_by(self, divisor: u64) -> Money {
        Money(self.0.checked_div(divisor).expect("money division by zero"))
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Why a money movement was refused. Refusal is always atomic: the books are
/// untouched when one of these is returned (§8.5).
#[derive(Debug, PartialEq, Eq)]
pub enum MoneyError {
    /// The debited account holds less than the requested amount. There is no
    /// overdraft in v1 (§8.5).
    InsufficientFunds,
}

/// The single store of all balances (§8.2). `balances` is private; the only
/// public mutators are `transfer`, `mint`, and `burn`.
#[derive(Debug, Default)]
pub struct Accounts {
    balances: HashMap<(AgentId, Metal), Money>,
    total_minted: HashMap<Metal, Money>,
    total_burned: HashMap<Metal, Money>,
}

impl Accounts {
    /// An empty book: no balances, nothing minted, nothing burned. There is
    /// no genesis supply — money only ever enters via [`Accounts::mint`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Read-only. Unknown id reads as zero — accounts are created implicitly
    /// at first credit.
    pub fn balance_of(&self, id: AgentId, metal: Metal) -> Money {
        self.balances
            .get(&(id, metal))
            .copied()
            .unwrap_or(Money::ZERO)
    }

    /// Sum of ALL of one metal's balances, including External. There is no
    /// cross-metal total by design: without a market rate it would be a
    /// meaningless number, so the signature refuses to produce one.
    pub fn total_money(&self, metal: Metal) -> Money {
        self.balances
            .iter()
            .filter(|((_, entry_metal), _)| *entry_metal == metal)
            .fold(Money::ZERO, |sum, (_, &b)| sum.plus(b))
    }

    /// Lifetime total of one metal ever created via [`Accounts::mint`]
    /// (§8.4 log). Never decreases.
    pub fn total_minted(&self, metal: Metal) -> Money {
        self.total_minted
            .get(&metal)
            .copied()
            .unwrap_or(Money::ZERO)
    }

    /// Lifetime total of one metal ever destroyed via [`Accounts::burn`]
    /// (§8.4 log). Never decreases.
    pub fn total_burned(&self, metal: Metal) -> Money {
        self.total_burned
            .get(&metal)
            .copied()
            .unwrap_or(Money::ZERO)
    }

    /// §8.4: the ONLY way money is created. Credits `to` and logs to
    /// [`total_minted`](Accounts::total_minted); cannot fail. Gold-reserve
    /// cap deferred — spec amendment needed when the mint job arrives.
    pub fn mint(&mut self, to: AgentId, metal: Metal, amount: Money) {
        let balance = self.balance_of(to, metal);
        self.balances.insert((to, metal), balance.plus(amount));
        let minted = self.total_minted(metal).plus(amount);
        self.total_minted.insert(metal, minted);
    }

    /// §8.3: asserts conservation. Initial supply is zero (no genesis), so
    /// circulating money must equal minted − burned exactly.
    ///
    /// # Panics
    ///
    /// Panics on any imbalance — by design, never softened to a `Result`. A
    /// failed audit means the §8.2 chokepoint was bypassed somewhere; the sim
    /// must not keep running on corrupt books.
    pub fn audit(&self) {
        let expected = self
            .total_minted(Metal::Gold)
            .0
            .checked_sub(self.total_burned(Metal::Gold).0)
            .expect("audit failed: total_burned exceeds total_minted (§8.3)");
        assert_eq!(
            self.total_money(Metal::Gold),
            Money(expected),
            "conservation audit failed: circulating money != minted - burned (§8.3)"
        );
    }

    /// §8.2/§8.5: moves money between accounts, or errs with NO state change.
    /// Zero-amount and self-transfers of verified funds are no-ops.
    ///
    /// # Errors
    ///
    /// [`MoneyError::InsufficientFunds`] if `from` holds less than `amount`
    /// — no overdraft (§8.5), nothing applied.
    pub fn transfer(
        &mut self,
        from: AgentId,
        to: AgentId,
        metal: Metal,
        amount: Money,
    ) -> Result<(), MoneyError> {
        if amount == Money::ZERO {
            return Ok(()); // no-op by contract: creates no account entry
        }
        let from_balance = self.balance_of(from, metal);
        if from_balance < amount {
            return Err(MoneyError::InsufficientFunds); // §8.5 — nothing applied
        }
        if from == to {
            return Ok(()); // funds verified; debit + credit would cancel out
        }
        self.balances
            .insert((from, metal), from_balance.minus(amount));
        let to_balance = self.balance_of(to, metal);
        self.balances.insert((to, metal), to_balance.plus(amount));
        Ok(())
    }

    /// §8.4: the ONLY way money is destroyed. Debits `from` and logs to
    /// [`total_burned`](Accounts::total_burned). Same atomicity rules as
    /// [`transfer`](Accounts::transfer) (§8.5): zero is a no-op.
    ///
    /// # Errors
    ///
    /// [`MoneyError::InsufficientFunds`] if `from` holds less than `amount`
    /// — nothing applied.
    #[allow(dead_code)] // the sinks phase (7) lands later
    pub fn burn(&mut self, from: AgentId, metal: Metal, amount: Money) -> Result<(), MoneyError> {
        if amount == Money::ZERO {
            return Ok(());
        }
        let balance = self.balance_of(from, metal);
        if balance < amount {
            return Err(MoneyError::InsufficientFunds); // §8.5 — nothing applied
        }
        self.balances.insert((from, metal), balance.minus(amount));
        let burned = self.total_burned(metal).plus(amount);
        self.total_burned.insert(metal, burned);
        Ok(())
    }

    /// The SANCTIONED §8.2 exception: exists solely so tests can force an
    /// imbalance and prove the audit panics. Never compiled into the sim.
    #[cfg(test)]
    pub fn set_balance_for_test(&mut self, id: AgentId, metal: Metal, amount: Money) {
        self.balances.insert((id, metal), amount);
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
        accounts.mint(a(), Metal::Gold, Money::new(100));
        assert_eq!(accounts.balance_of(a(), Metal::Gold), Money::new(100));
        assert_eq!(accounts.total_minted(Metal::Gold), Money::new(100));
        assert_eq!(accounts.total_money(Metal::Gold), Money::new(100));
        accounts.audit();
    }

    #[test]
    fn transfer_moves_exact_amount() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Metal::Gold, Money::new(100));
        accounts
            .transfer(a(), b(), Metal::Gold, Money::new(30))
            .unwrap();
        assert_eq!(accounts.balance_of(a(), Metal::Gold), Money::new(70));
        assert_eq!(accounts.balance_of(b(), Metal::Gold), Money::new(30));
        assert_eq!(accounts.total_money(Metal::Gold), Money::new(100));
    }

    #[test]
    fn transfer_insufficient_funds_is_atomic() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Metal::Gold, Money::new(10));
        let result = accounts.transfer(a(), b(), Metal::Gold, Money::new(20));
        assert_eq!(result, Err(MoneyError::InsufficientFunds));
        // no partial application — nothing changed
        assert_eq!(accounts.balance_of(a(), Metal::Gold), Money::new(10));
        assert_eq!(accounts.balance_of(b(), Metal::Gold), Money::ZERO);
    }

    #[test]
    fn transfer_zero_is_noop() {
        let mut accounts = Accounts::new();
        accounts
            .transfer(a(), b(), Metal::Gold, Money::ZERO)
            .unwrap();
        assert_eq!(accounts.total_money(Metal::Gold), Money::ZERO);
        // creates no account entry (tests may touch private fields — same module)
        assert!(accounts.balances.is_empty());
    }

    #[test]
    fn transfer_to_self() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Metal::Gold, Money::new(50));
        accounts
            .transfer(a(), a(), Metal::Gold, Money::new(20))
            .unwrap();
        assert_eq!(accounts.balance_of(a(), Metal::Gold), Money::new(50));
    }

    #[test]
    fn burn_debits_and_logs() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Metal::Gold, Money::new(100));
        accounts.burn(a(), Metal::Gold, Money::new(40)).unwrap();
        assert_eq!(accounts.balance_of(a(), Metal::Gold), Money::new(60));
        assert_eq!(accounts.total_burned(Metal::Gold), Money::new(40));
        accounts.audit();
    }

    #[test]
    fn burn_insufficient_funds_is_atomic() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Metal::Gold, Money::new(10));
        assert_eq!(
            accounts.burn(a(), Metal::Gold, Money::new(20)),
            Err(MoneyError::InsufficientFunds)
        );
        assert_eq!(accounts.balance_of(a(), Metal::Gold), Money::new(10));
        assert_eq!(accounts.total_burned(Metal::Gold), Money::ZERO);
    }

    #[test]
    fn burn_zero_is_noop() {
        let mut accounts = Accounts::new();
        accounts.burn(a(), Metal::Gold, Money::ZERO).unwrap();
        assert_eq!(accounts.total_burned(Metal::Gold), Money::ZERO);
        assert!(accounts.balances.is_empty());
    }

    #[test]
    fn audit_passes_after_op_sequence() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Metal::Gold, Money::new(100));
        accounts.audit();
        accounts
            .transfer(a(), b(), Metal::Gold, Money::new(30))
            .unwrap();
        accounts.audit();
        // failed ops must leave the books balanced too
        assert!(
            accounts
                .transfer(b(), a(), Metal::Gold, Money::new(999))
                .is_err()
        );
        accounts.audit();
        accounts.burn(a(), Metal::Gold, Money::new(20)).unwrap();
        accounts.audit();
        assert!(accounts.burn(b(), Metal::Gold, Money::new(999)).is_err());
        accounts.audit();
        accounts.mint(b(), Metal::Gold, Money::new(5));
        accounts.audit();
    }

    #[test]
    #[should_panic]
    fn audit_panics_on_imbalance() {
        let mut accounts = Accounts::new();
        accounts.mint(a(), Metal::Gold, Money::new(100));
        accounts.set_balance_for_test(a(), Metal::Gold, Money::new(150));
        accounts.audit();
    }

    #[test]
    fn total_money_includes_external() {
        // External is just an id from Accounts' perspective; 1 is its
        // reserved value (World reserves it properly in Task 2).
        let external = AgentId(1);
        let mut accounts = Accounts::new();
        accounts.mint(a(), Metal::Gold, Money::new(100));
        accounts
            .transfer(a(), external, Metal::Gold, Money::new(60))
            .unwrap();
        // out of circulation but still counted by the audit
        assert_eq!(accounts.total_money(Metal::Gold), Money::new(100));
        accounts.audit();
    }

    #[test]
    fn times_scales_a_unit_price() {
        assert_eq!(Money::new(5).times(3), Money::new(15));
        assert_eq!(Money::new(5).times(0), Money::ZERO);
        assert_eq!(Money::ZERO.times(999), Money::ZERO);
    }

    #[test]
    #[should_panic(expected = "money overflow")]
    fn times_panics_on_overflow() {
        Money::new(u64::MAX).times(2);
    }

    #[test]
    fn divided_by_floors() {
        assert_eq!(Money::new(25).divided_by(10), Money::new(2));
        assert_eq!(Money::new(9).divided_by(10), Money::ZERO);
        assert_eq!(Money::ZERO.divided_by(10), Money::ZERO);
    }
}
