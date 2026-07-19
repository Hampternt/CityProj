// Data-only until the phases land (Tasks 6-10). Remove with Task 12's
// dead-code sweep.
#![allow(dead_code)]

//! The goods market's shopping logic (§8.6: pricing/purchasing logic
//! lives HERE, never on agents or money). `plan_purchases` is pure —
//! wallet + inventory + posted offers in, purchase plan out; no world
//! access, fully deterministic. sim.rs builds `Offer`s from
//! `World::businesses()` and applies plans through `World::pay`.

use std::collections::HashMap;

use crate::agent::AgentId;
use crate::goods::Good;
use crate::money::Money;

/// One business's posted sale, snapshotted at phase start. Every agent
/// plans against the same snapshot; apply-time caps handle staleness.
#[derive(Debug, Clone)]
pub struct Offer {
    pub business: AgentId,
    pub good: Good,
    pub price: Money,
    pub stock: u32,
}

/// Planned units from one business — coalesced: at most one entry per
/// (business, good) pair, in first-planned order.
#[derive(Debug, PartialEq, Eq)]
pub struct Purchase {
    pub business: AgentId,
    pub good: Good,
    pub units: u32,
}

/// Greedy needs-shopping with diminishing returns: repeatedly buy 1 unit
/// of the highest-scoring good — score = weight / (held + planned + 1) —
/// that is (a) affordable in the remaining budget, (b) in remaining offer
/// stock, and (c) below its `target_days × consumption_rate` cap. Ties
/// between goods keep the earlier `Good::ALL` entry; same-good offers go
/// cheapest first, then input order. An empty plan is the valid "can't
/// afford anything" result. Terminates: every iteration moves some good
/// toward its finite cap.
pub fn plan_purchases(
    wallet: Money,
    inventory: &HashMap<Good, u32>,
    offers: &[Offer],
) -> Vec<Purchase> {
    let mut budget = wallet;
    let mut remaining: Vec<u32> = offers.iter().map(|offer| offer.stock).collect();
    let mut planned: HashMap<Good, u32> = HashMap::new();
    let mut purchases: Vec<Purchase> = Vec::new();

    while let Some(index) = best_buy(budget, inventory, offers, &remaining, &planned) {
        let offer = &offers[index];
        budget = budget.minus(offer.price); // affordability was checked
        remaining[index] -= 1;
        *planned.entry(offer.good).or_insert(0) += 1;
        let existing = purchases
            .iter_mut()
            .find(|p| p.business == offer.business && p.good == offer.good);
        match existing {
            Some(purchase) => purchase.units += 1,
            None => purchases.push(Purchase {
                business: offer.business,
                good: offer.good,
                units: 1,
            }),
        }
    }
    purchases
}

/// The single next unit to buy, as an index into `offers` — or `None`
/// when no good qualifies. Scores are compared by cross-multiplication
/// (`w_a/d_a > w_b/d_b  ⇔  w_a·d_b > w_b·d_a` in u64) so integer
/// division never truncates a ranking; the strict `>` keeps earlier
/// `Good::ALL` entries on ties.
fn best_buy(
    budget: Money,
    inventory: &HashMap<Good, u32>,
    offers: &[Offer],
    remaining: &[u32],
    planned: &HashMap<Good, u32>,
) -> Option<usize> {
    let mut best: Option<(usize, u32, u32)> = None; // (offer index, weight, denominator)
    for good in Good::ALL {
        let held = inventory.get(&good).copied().unwrap_or(0);
        let in_plan = planned.get(&good).copied().unwrap_or(0);
        if held + in_plan >= good.target_days() * good.consumption_rate() {
            continue; // cap reached — diminishing returns bottom out
        }
        let Some(index) = cheapest_offer(good, budget, offers, remaining) else {
            continue; // nothing affordable in stock
        };
        let denominator = held + in_plan + 1;
        let beats_best = match best {
            None => true,
            Some((_, best_weight, best_denominator)) => {
                u64::from(good.weight()) * u64::from(best_denominator)
                    > u64::from(best_weight) * u64::from(denominator)
            }
        };
        if beats_best {
            best = Some((index, good.weight(), denominator));
        }
    }
    best.map(|(index, _, _)| index)
}

/// Cheapest affordable offer of `good` with stock left; price ties keep
/// the earliest input offer.
fn cheapest_offer(
    good: Good,
    budget: Money,
    offers: &[Offer],
    remaining: &[u32],
) -> Option<usize> {
    let mut cheapest: Option<usize> = None;
    for (index, offer) in offers.iter().enumerate() {
        if offer.good != good || remaining[index] == 0 || offer.price > budget {
            continue;
        }
        if cheapest.is_none_or(|current| offer.price < offers[current].price) {
            cheapest = Some(index);
        }
    }
    cheapest
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentId;
    use std::collections::HashMap;

    fn offer(business: u32, good: Good, price: u64, stock: u32) -> Offer {
        Offer { business: AgentId(business), good, price: Money::new(price), stock }
    }

    fn full_offers() -> Vec<Offer> {
        vec![
            offer(10, Good::Food, 1, 1000),
            offer(11, Good::Entertainment, 2, 1000),
            offer(12, Good::Luxury, 5, 1000),
        ]
    }

    #[test]
    fn no_offers_or_no_money_yields_the_empty_plan() {
        let inventory = HashMap::new();
        assert!(plan_purchases(Money::new(100), &inventory, &[]).is_empty());
        assert!(plan_purchases(Money::ZERO, &inventory, &full_offers()).is_empty());
    }

    #[test]
    fn highest_weight_wins_on_an_empty_stomach() {
        // one affordable unit: Food's weight 100 beats 30 and 10
        let plan = plan_purchases(Money::new(1), &HashMap::new(), &full_offers());
        assert_eq!(
            plan,
            vec![Purchase { business: AgentId(10), good: Good::Food, units: 1 }]
        );
    }

    #[test]
    fn diminishing_returns_divert_spending() {
        // held Food 10 → score 100/11 ≈ 9; empty Entertainment → 30/1
        let inventory = HashMap::from([(Good::Food, 10)]);
        let plan = plan_purchases(Money::new(2), &inventory, &full_offers());
        assert_eq!(
            plan,
            vec![Purchase { business: AgentId(11), good: Good::Entertainment, units: 1 }]
        );
    }

    #[test]
    fn purchases_coalesce_per_business_and_good() {
        // budget 3 buys food thrice from the same stall → one entry, units 3
        let plan = plan_purchases(Money::new(3), &HashMap::new(), &full_offers());
        assert_eq!(
            plan,
            vec![Purchase { business: AgentId(10), good: Good::Food, units: 3 }]
        );
    }

    #[test]
    fn offer_stock_is_respected() {
        // only 2 food on the shelf; the rest of the budget moves down-list
        let offers = vec![offer(10, Good::Food, 1, 2), offer(11, Good::Entertainment, 2, 1000)];
        let plan = plan_purchases(Money::new(4), &HashMap::new(), &offers);
        assert_eq!(
            plan,
            vec![
                Purchase { business: AgentId(10), good: Good::Food, units: 2 },
                Purchase { business: AgentId(11), good: Good::Entertainment, units: 1 },
            ]
        );
    }

    #[test]
    fn target_cap_stops_the_stockpile() {
        // Food cap = 7 days × 10/tick = 70; holding 69 leaves room for 1
        let inventory = HashMap::from([(Good::Food, 69)]);
        let offers = vec![offer(10, Good::Food, 1, 1000)];
        let plan = plan_purchases(Money::new(100), &inventory, &offers);
        assert_eq!(
            plan,
            vec![Purchase { business: AgentId(10), good: Good::Food, units: 1 }]
        );
        // at the cap exactly: nothing to buy
        let at_cap = HashMap::from([(Good::Food, 70)]);
        assert!(plan_purchases(Money::new(100), &at_cap, &offers).is_empty());
    }

    #[test]
    fn same_good_offers_go_cheapest_first_then_input_order() {
        // pricier stall listed first; cheaper one must still win
        let offers = vec![offer(20, Good::Food, 2, 1000), offer(21, Good::Food, 1, 1)];
        let plan = plan_purchases(Money::new(3), &HashMap::new(), &offers);
        assert_eq!(
            plan,
            vec![
                Purchase { business: AgentId(21), good: Good::Food, units: 1 },
                Purchase { business: AgentId(20), good: Good::Food, units: 1 },
            ]
        );
        // price tie: earlier input offer wins
        let tied = vec![offer(30, Good::Food, 1, 1000), offer(31, Good::Food, 1, 1000)];
        let plan = plan_purchases(Money::new(2), &HashMap::new(), &tied);
        assert_eq!(
            plan,
            vec![Purchase { business: AgentId(30), good: Good::Food, units: 2 }]
        );
    }

    #[test]
    fn unaffordable_goods_are_skipped_not_blocking() {
        // 1 coin: luxury (5) unaffordable, food affordable
        let inventory = HashMap::from([(Good::Food, 69), (Good::Entertainment, 35)]);
        // Entertainment at cap (35 = 7×5), food nearly capped: luxury would
        // score highest but costs too much — food's last unit still sells
        let plan = plan_purchases(Money::new(1), &inventory, &full_offers());
        assert_eq!(
            plan,
            vec![Purchase { business: AgentId(10), good: Good::Food, units: 1 }]
        );
    }
}
