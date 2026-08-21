//! Hand-seeded scenarios. Every coin enters through `mint`, so the audit
//! counts each seed as the entire per-metal money supply, forever — there
//! is no tick-time faucet (§8.4). Deterministic and seedless: no RNG
//! exists in this codebase, so the same world boots every run.

use std::collections::HashMap;

use crate::agent::AgentId;
use crate::business::RoleSlot;
use crate::goods::Good;
use crate::metal::Metal;
use crate::money::Money;
use crate::role::Role;
use crate::world::World;

/// The 07-19 minimal-needs scenario: farm, theater, and jeweler (one
/// Labourer slot each at wage 35), three employed agents, one unemployed,
/// all housed at the residence. Worldgen seeds every business with one
/// wage bill — tick 1 must be pre-funded because there is no per-tick mint
/// faucet; the seed is the entire money supply and tick 1's wages are paid
/// before any business revenue exists — and every agent with a small wallet
/// plus one day's goods. All seeding goes through `mint`, so the audit
/// counts it (§8.4). The economy trades in gold only; each agent also
/// holds small silver and copper savings (pack 2, D1) that stay inert
/// until the market layer can price non-gold metals — they exist so every
/// metal's ledger and conservation total is live in production.
///
/// Since town-colony pack 2 this is the small TEST FIXTURE — the shipped
/// scenario is [`town_world`], so this only compiles into test builds.
#[cfg(test)]
pub fn template_world() -> World {
    let mut world = World::new();
    let residence = world.add_house("1 Mill Lane", vec![]);

    let farm = world.add_house("Greenrow Farm", vec![]);
    let theater = world.add_house("Gilt Curtain Theater", vec![]);
    let jeweler = world.add_house("Karat & Co", vec![]);
    let scenario = [
        (farm, Good::Food, Money::new(1), "alice"),
        (theater, Good::Entertainment, Money::new(2), "bob"),
        (jeweler, Good::Luxury, Money::new(5), "carol"),
    ];
    for (house, product, price, worker_name) in scenario {
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(35),
                headcount: 1,
            },
        );
        let business = world
            .create_business(house, product, price, roles)
            .expect("fresh house");
        let bill = world
            .house(house)
            .expect("just added")
            .business
            .as_ref()
            .expect("just created")
            .wage_bill();
        world.accounts.mint(business, Metal::Gold, bill);
        let worker = world.spawn_agent(worker_name, Some(residence), Some(house));
        world.agent_mut(worker).expect("just spawned").employed_role = Some(Role::Labourer);
    }
    world.spawn_agent("dave", Some(residence), None); // unemployed, housed

    let everyone: Vec<AgentId> = world.agents.iter().map(|agent| agent.id).collect();
    for id in everyone {
        world.accounts.mint(id, Metal::Gold, Money::new(35));
        // Inert savings (D1): nothing spends non-gold until markets can
        // price it, so these only exercise the per-metal books.
        world.accounts.mint(id, Metal::Silver, Money::new(10));
        world.accounts.mint(id, Metal::Copper, Money::new(20));
        let agent = world.agent_mut(id).expect("listed above");
        for good in Good::ALL {
            agent.inventory.insert(good, good.consumption_rate());
        }
    }
    world
}

// ---------------------------------------------------------------------
// town_world tuning constants (pack 2): iterated against the spec's
// pinned soak exit criteria, then frozen. All gold — silver and copper
// stay inert savings until the market layer can price them.

/// Wallet each employed agent starts with — bridges the revenue drought
/// a business sees before the price rotation first reaches it (its
/// workers still buy while their wages accrue as arrears).
const EMPLOYED_WALLET: u64 = 120;
/// Savings each seeded-unemployed agent lives off until pack 3's labor
/// market gives them income — sized to keep them buying Food through the
/// 100-tick soak with margin (the liveness criterion applies to them
/// too; steady-state spend is ~25g/tick at settled prices).
const UNEMPLOYED_SAVINGS: u64 = 4000;
/// External's gold settlement fund: pack 4's immigration grubstakes draw
/// from here; until then it sits on the books, audited like everything.
const SETTLEMENT_FUND: u64 = 600;
const SILVER_SAVINGS: u64 = 10;
const COPPER_SAVINGS: u64 = 20;
/// How many full-staffing wage bills each business's coffer starts with.
const WAGE_BILLS_SEEDED: u32 = 3;

/// Every named resident, spawn order = ascending `AgentId`. The first
/// headcount-sum names (16) fill business slots in declaration order; the
/// rest are seeded unemployed.
const NAMES: [&str; 30] = [
    "alice", "bob", "carol", "dave", "ed", "fiona", "george", "hana", "ivan", "judit", "karl",
    "lena", "marco", "nadia", "otto", "petra", "quinn", "rosa", "sam", "tessa", "ulf", "vera",
    "will", "xenia", "yara", "zeno", "mira", "tomas", "orla", "bram",
];

/// The shipped town (town-colony spec, `town_world` contract): 30 agents
/// across 4 occupied residences (8/8/8/6), 2 zero-occupant spare
/// residences (pack 4's landing pads), and 6 multi-worker businesses over
/// all three Goods — two competing sellers of each. 16 agents are
/// seeded employed, 14 unemployed (they live off savings until pack 3's
/// labor market). Each business is pre-funded with one full-staffing wage
/// bill, three deep (`WAGE_BILLS_SEEDED`); External holds the settlement
/// fund. Deterministic and seedless;
/// its per-metal totals are the entire money supply, pinned by test and
/// audit forever.
pub fn town_world() -> World {
    let mut world = World::new();
    let residences = [
        "1 Mill Lane",
        "2 Mill Lane",
        "3 Orchard Row",
        "4 Orchard Row",
    ]
    .map(|address| world.add_house(address, vec![]));
    // Zero-occupant spares: vacancy is "no occupants, hosts no business".
    world.add_house("5 Weir Cottage", vec![]);
    world.add_house("6 Weir Cottage", vec![]);

    // (address, product, price, wage, headcount) — soak-tuned, then
    // frozen, all three regimes measured, not taste. FOOD runs a mild
    // surplus (8×40=320 vs 30×10=300): surplus caps prices low while
    // rotation sell-outs keep them moving, and the loser's backlog
    // serves the buying-order tail. ENT and LUX run deliberate SCARCITY
    // (80 vs 150 nominal, 32 vs 60): a staffer's output value at the
    // floor (production_rate × 1g) is below any livable wage for these
    // goods, so parity or surplus bankrupts their venues (measured —
    // arrears in the thousands by t60); scarcity keeps their prices
    // above the floor where revenue covers payroll, and only Food's
    // liveness is universal by criterion. Not every agent gets
    // entertainment every tick — that is the town's poverty, not a bug: with surplus every
    // seller floors out and the houses-order tie-break routes all demand
    // to the first forever; at parity the cheapest sells out, raises,
    // and demand rotates. TWO sellers per good, measured, not taste: a
    // third wins at most a warm-up transient before its payroll starves
    // — the floor can't be undercut, ties favor earlier houses, and the
    // loser-lowers step re-undercuts before its turn comes again
    // (recorded in the pack ledger as a deviation from the spec's "7–9
    // businesses"; the mechanics admit exactly two solvent sellers per
    // good until the market layer changes).
    let businesses = [
        ("Greenrow Farm", Good::Food, 2u64, 35u64, 4u32),
        ("Longacre Farm", Good::Food, 3, 35, 4),
        ("Gilt Curtain Theater", Good::Entertainment, 2, 36, 2),
        ("The Brass Bell", Good::Entertainment, 3, 36, 2),
        ("Karat & Co", Good::Luxury, 4, 40, 2),
        ("Silverthread Atelier", Good::Luxury, 5, 40, 2),
    ];

    let mut next_name = 0;
    for (address, product, price, wage, headcount) in businesses {
        let house = world.add_house(address, vec![]);
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(wage),
                headcount,
            },
        );
        let business = world
            .create_business(house, product, Money::new(price), roles)
            .expect("fresh house");
        let bill = world
            .house(house)
            .expect("just added")
            .business
            .as_ref()
            .expect("just created")
            .wage_bill();
        // Three bills deep, not one: the demand rotation takes several
        // ticks to first reach each seller, and payroll must survive
        // that drought (soak-tuned, then frozen).
        world
            .accounts
            .mint(business, Metal::Gold, bill.times(WAGE_BILLS_SEEDED));
        // Shelves start two ticks deep: without opening stock, the first
        // ticks create pantry deficits in the late buying order that an
        // exactly-cleared market can never absorb again (soak-tuned).
        world
            .house_mut(house)
            .expect("just added")
            .business
            .as_mut()
            .expect("just created")
            .stock = 2 * product.production_rate() * headcount;
        for _ in 0..headcount {
            let name = NAMES[next_name];
            let home = residences[next_name / 8];
            next_name += 1;
            let worker = world.spawn_agent(name, Some(home), Some(house));
            world.agent_mut(worker).expect("just spawned").employed_role = Some(Role::Labourer);
        }
    }
    // The rest are unemployed until pack 3's labor market hires them.
    while next_name < NAMES.len() {
        world.spawn_agent(NAMES[next_name], Some(residences[next_name / 8]), None);
        next_name += 1;
    }

    let everyone: Vec<(AgentId, bool)> = world
        .agents
        .iter()
        .map(|agent| (agent.id, agent.workplace.is_some()))
        .collect();
    for (id, employed) in everyone {
        let wallet = if employed {
            EMPLOYED_WALLET
        } else {
            UNEMPLOYED_SAVINGS
        };
        world.accounts.mint(id, Metal::Gold, Money::new(wallet));
        world
            .accounts
            .mint(id, Metal::Silver, Money::new(SILVER_SAVINGS));
        world
            .accounts
            .mint(id, Metal::Copper, Money::new(COPPER_SAVINGS));
        let agent = world.agent_mut(id).expect("listed above");
        for good in Good::ALL {
            agent.inventory.insert(good, good.consumption_rate());
        }
    }
    world
        .accounts
        .mint(world.external_id, Metal::Gold, Money::new(SETTLEMENT_FUND));
    world
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D1 (pack 2 manifest): gold funds the whole economy — 3 wage bills of
    /// 35 plus 4 wallets of 35 — and every agent holds inert silver 10 /
    /// copper 20 savings, so all three ledgers are live from tick 0.
    #[test]
    fn template_world_seeds_the_decided_metals() {
        let world = template_world();
        assert_eq!(world.accounts.total_money(Metal::Gold), Money::new(245));
        assert_eq!(world.accounts.total_minted(Metal::Gold), Money::new(245));
        assert_eq!(world.accounts.total_money(Metal::Silver), Money::new(40));
        assert_eq!(world.accounts.total_minted(Metal::Silver), Money::new(40));
        assert_eq!(world.accounts.total_money(Metal::Copper), Money::new(80));
        assert_eq!(world.accounts.total_minted(Metal::Copper), Money::new(80));
        world.accounts.audit();
    }

    #[test]
    fn town_world_has_the_decided_shape() {
        let world = town_world();
        // 30 agents, 16 employed, 14 unemployed (pack 3's hiring pool)
        assert_eq!(world.agents.len(), 30);
        let employed = world
            .agents
            .iter()
            .filter(|a| a.workplace.is_some())
            .count();
        assert_eq!(employed, 16);
        // every employed agent has a slotted role (they must earn)
        assert!(
            world
                .agents
                .iter()
                .filter(|a| a.workplace.is_some())
                .all(|a| a.employed_role.is_some())
        );
        // 12 houses: 4 occupied residences + 2 vacant spares + 6 premises
        assert_eq!(world.houses.len(), 12);
        assert_eq!(world.businesses().count(), 6);
        // ≥2 competing Food sellers, and every Good is produced
        for good in Good::ALL {
            let sellers = world
                .businesses()
                .filter(|(_, b)| b.product == good)
                .count();
            assert!(sellers >= 2, "{good} needs competing sellers");
        }
        // the spares stand empty and host nothing
        for spare in ["5 Weir Cottage", "6 Weir Cottage"] {
            let house = world
                .houses
                .iter()
                .find(|h| h.address == spare)
                .expect("spare exists");
            assert!(world.occupants_of(house.id).is_empty());
            assert!(house.business.is_none());
        }
        // every business fully staffed at seed, multi-worker slots
        for (house, business) in world.businesses() {
            let staff = world.employees_of(house.id).len() as u32;
            let headcount: u32 = business.roles.values().map(|slot| slot.headcount).sum();
            assert_eq!(staff, headcount, "{} staffed to headcount", house.address);
            assert!(headcount > 1, "{} is multi-worker", house.address);
        }
        world.accounts.audit();
    }

    /// The pack-2 conservation re-pin (one deliberate item): town_world's
    /// per-metal totals are the entire money supply, forever — the audit
    /// holds them here every tick. Gold = 6 coffers at three full-staffing
    /// bills (3×584) + 16 employed wallets of 120 + 14 savings of 4000 +
    /// External's 600 fund. A worldgen change must change these constants
    /// consciously, in its own item.
    #[test]
    fn town_world_seeds_the_decided_metals() {
        let world = town_world();
        assert_eq!(world.accounts.total_money(Metal::Gold), Money::new(60272));
        assert_eq!(world.accounts.total_minted(Metal::Gold), Money::new(60272));
        assert_eq!(world.accounts.total_money(Metal::Silver), Money::new(300));
        assert_eq!(world.accounts.total_minted(Metal::Silver), Money::new(300));
        assert_eq!(world.accounts.total_money(Metal::Copper), Money::new(600));
        assert_eq!(world.accounts.total_minted(Metal::Copper), Money::new(600));
        world.accounts.audit();
    }

    /// The spec's pinned soak exit criteria (town-colony spec, "Pinned
    /// soak exit criteria"): the tuning constants above were iterated
    /// until this held, then frozen. 100 ticks, evaluated from tick 10
    /// (warm-up excluded); the audit runs inside every `tick`, so any §8
    /// break panics the soak.
    #[test]
    fn town_soak_holds_the_pinned_exit_criteria() {
        use crate::sim::{self, Event};

        const LAST: u64 = 100;
        const FROM: u64 = 10; // warm-up excluded
        const WINDOW: u64 = 5;
        let floor = Money::new(1);

        let mut world = town_world();
        let mut food_ticks: HashMap<AgentId, Vec<u64>> = HashMap::new();
        let mut cheapest: HashMap<Good, Vec<Money>> = HashMap::new();
        // per business: (rises, falls) after warm-up
        let mut moved: HashMap<AgentId, (u32, u32)> = HashMap::new();

        for t in 1..=LAST {
            // the prices in force during tick t are those posted before
            // it — sample ahead of the write-back, not after
            for good in Good::ALL {
                let min = world
                    .businesses()
                    .filter(|(_, b)| b.product == good)
                    .map(|(_, b)| b.price)
                    .min()
                    .expect("every good has sellers");
                cheapest.entry(good).or_default().push(min);
            }
            let report = sim::tick(&mut world);
            for event in &report.events {
                match event {
                    Event::Sold {
                        buyer,
                        good: Good::Food,
                        ..
                    } => food_ticks.entry(*buyer).or_default().push(t),
                    Event::PriceMoved {
                        business, from, to, ..
                    } if t >= FROM => {
                        let entry = moved.entry(*business).or_default();
                        if to > from {
                            entry.0 += 1;
                        } else {
                            entry.1 += 1;
                        }
                    }
                    _ => {}
                }
            }
        }

        // 1. every agent completes ≥1 Food purchase in every rolling
        //    5-tick window of the evaluated span
        for agent in &world.agents {
            let ticks = food_ticks
                .get(&agent.id)
                .unwrap_or_else(|| panic!("{} never bought Food", agent.name));
            for start in FROM..=(LAST - WINDOW + 1) {
                assert!(
                    ticks.iter().any(|&t| t >= start && t < start + WINDOW),
                    "{} bought no Food in ticks {start}–{}",
                    agent.name,
                    start + WINDOW - 1,
                );
            }
        }

        // 2. per Good — not per seller: the cheapest posted price neither
        //    sits at the floor for the whole span nor rises monotonically
        for good in Good::ALL {
            let series = &cheapest[&good][(FROM as usize - 1)..];
            assert!(
                series.iter().any(|&p| p != floor),
                "{good}'s cheapest price is floor-pinned all span"
            );
            let nondecreasing = series.windows(2).all(|w| w[1] >= w[0]);
            let rose = series.last() > series.first();
            assert!(
                !(nondecreasing && rose),
                "{good}'s cheapest price rises monotonically"
            );
        }

        // 3. at least one price moves in both directions — ONE posted
        //    price, not an aggregate across sellers (spec sentence)
        assert!(
            moved.values().any(|&(rises, falls)| rises > 0 && falls > 0),
            "no single posted price moved in both directions after warm-up"
        );
    }
}
