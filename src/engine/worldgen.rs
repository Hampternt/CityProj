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
        // Owner-before-venue (firm-lifecycle pack 1): the worker must
        // exist so `create_business` can validate them as owner — each
        // venue is an owner-operator shop.
        let worker = world.spawn_agent(worker_name, Some(residence), Some(house));
        world.agent_mut(worker).expect("just spawned").employed_role = Some(Role::Labourer);
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(35),
                headcount: 1,
                unfilled_ticks: 0,
            },
        );
        let business = world
            .create_business(house, worker, product, price, roles)
            .expect("fresh house, spawned owner");
        let bill = world
            .house(house)
            .expect("just added")
            .business
            .as_ref()
            .expect("just created")
            .wage_bill();
        world.accounts.mint(business, Metal::Gold, bill);
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
/// too; steady-state spend is ~25g/tick at settled prices). Also the
/// town's DEMAND FUSE (pack-3 close review; re-timed for pack 4): the
/// frozen equilibrium is ~30% dis-saving-financed — coffers absorb
/// ~90g/tick as one-way sinks while the 9 permanently unemployed
/// dis-save ~25g/tick each — so this constant times ~25 sets when
/// destitution arrives. Pack 4 shortened it 4000 → 3400 so the WHOLE
/// breathing chain (broke → hungry → depart → demand shock → quits →
/// K-aged vacancy → grubstaked arrival) completes inside the 200-tick
/// soak window, while the first departure (t127, measured) lands safely
/// beyond the 100-tick soak's criteria span. Migration relieves the
/// fuse. The phase-6 draw (firm-lifecycle pack 1) killed the coffer
/// SINK — coffers now cap at the retained buffer and ~22k gold
/// recirculated to owners over 200 ticks — but the fuse itself barely
/// moved (first departure still t127, measured): `target_days` purchase
/// caps keep owner income pooling in wallets, never reaching the
/// unemployed. The pooled capital is the recorded seam for founding
/// (pack 3) and phase 6's expand-capacity half.
const UNEMPLOYED_SAVINGS: u64 = 3400;
/// External's gold settlement fund: pack 4's immigration grubstakes draw
/// from here; until then it sits on the books, audited like everything.
const SETTLEMENT_FUND: u64 = 600;
const SILVER_SAVINGS: u64 = 10;
const COPPER_SAVINGS: u64 = 20;
/// How many full-staffing wage bills each business's coffer starts with.
const WAGE_BILLS_SEEDED: u32 = 3;

/// Every named resident, spawn order = ascending `AgentId`. The first
/// seeded-staff-sum names (16) fill business slots in declaration order;
/// the rest are seeded unemployed.
const NAMES: [&str; 30] = [
    "alice", "bob", "carol", "dave", "ed", "fiona", "george", "hana", "ivan", "judit", "karl",
    "lena", "marco", "nadia", "otto", "petra", "quinn", "rosa", "sam", "tessa", "ulf", "vera",
    "will", "xenia", "yara", "zeno", "mira", "tomas", "orla", "bram",
];

/// The shipped town (town-colony spec, `town_world` contract): 30 agents
/// across 4 occupied residences (8/8/8/6), 2 zero-occupant spare
/// residences (pack 4's landing pads), and 6 multi-worker businesses over
/// all three Goods — two competing sellers of each. 16 agents are
/// seeded employed; the rest are seeded unemployed and live off savings
/// until phase 1's labor market hires them into the open headcount
/// (pack 3: headcount exceeds seeded staffing, so slots stand open at
/// boot). Each business is pre-funded with one full-staffing wage
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
    // (address, product, price, wage, headcount, seeded_staff) —
    // pack 3: `headcount` exceeds `seeded_staff` so the labor market
    // has open slots to clear at boot; the wage column is seeded
    // SOLVENT (payroll at full staffing coverable by measured revenue)
    // because pack 3's quit rule turns latent insolvency into churn —
    // pack 2's lux wage of 40 bled 1320g of arrears by t100, harmless
    // only while quitting didn't exist (pack-3 ledger, traced).
    let businesses = [
        ("Greenrow Farm", Good::Food, 2u64, 35u64, 4u32, 4u32),
        ("Longacre Farm", Good::Food, 3, 35, 4, 4),
        ("Gilt Curtain Theater", Good::Entertainment, 2, 36, 3, 2),
        ("The Brass Bell", Good::Entertainment, 3, 36, 4, 2),
        ("Karat & Co", Good::Luxury, 4, 24, 3, 2),
        ("Silverthread Atelier", Good::Luxury, 5, 24, 3, 2),
    ];

    let mut next_name = 0;
    for (address, product, price, wage, headcount, seeded_staff) in businesses {
        let house = world.add_house(address, vec![]);
        // Owner-before-venue (firm-lifecycle pack 1): each venue's staff
        // spawn BEFORE `create_business` so the first seeded worker's id
        // exists to validate as owner — the owner-operator pattern
        // (alice, ed, ivan, karl, marco, otto by construction). The
        // agent-spawn ORDER (names, homes, sequence) is unchanged; only
        // the business-id interleaving moves, which is why this landed
        // as one deliberate re-pin item.
        let mut first_worker = None;
        for _ in 0..seeded_staff {
            let name = NAMES[next_name];
            let home = residences[next_name / 8];
            next_name += 1;
            let worker = world.spawn_agent(name, Some(home), Some(house));
            world.agent_mut(worker).expect("just spawned").employed_role = Some(Role::Labourer);
            first_worker.get_or_insert(worker);
        }
        let owner = first_worker.expect("every seeded venue has staff");
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(wage),
                headcount,
                unfilled_ticks: 0,
            },
        );
        let business = world
            .create_business(house, owner, product, Money::new(price), roles)
            .expect("fresh house, spawned owner");
        let bill = world
            .house(house)
            .expect("just added")
            .business
            .as_ref()
            .expect("just created")
            .wage_bill();
        // Three bills deep, not one: the demand rotation takes several
        // ticks to first reach each seller, and payroll must survive
        // that drought (soak-tuned, then frozen). Bills price the FULL
        // headcount, so open slots arrive pre-funded.
        world
            .accounts
            .mint(business, Metal::Gold, bill.times(WAGE_BILLS_SEEDED));
        // Shelves start two ticks deep — sized by the staff that
        // actually produce at boot, not the target headcount: without
        // opening stock, the first ticks create pantry deficits in the
        // late buying order that an exactly-cleared market can never
        // absorb again (soak-tuned).
        world
            .house_mut(house)
            .expect("just added")
            .business
            .as_mut()
            .expect("just created")
            .stock = 2 * product.production_rate() * seeded_staff;
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
    use crate::housing::HouseId;

    /// The conserved-recycle acceptance soak: criteria A1–A8 and A10 of the
    /// 2026-09-07 spec, over 5,000 ticks of the shipped town.
    ///
    /// **A0, the disallow clause, is why this test is shaped the way it is.**
    /// No criterion here may be "population 30, six firms, nobody left" —
    /// that passes identically on a healthy town and on a corpse, and the
    /// refused demurrage arms proved it: with no Food seller alive, phase
    /// 7's destitution decide is guarded, nobody can be judged destitute,
    /// and the population freezes *by rule*. So the load-bearing criteria
    /// are TRANSACTION VOLUME (A2) and the LIMIT CYCLE (A3), and the
    /// population figures are read only after those are green.
    ///
    /// Three horizons, not one: a value that holds only at the shortest
    /// horizon is a failure, not a pass (the refused mint-stipend arms
    /// looked perfectly stable at t300 and decayed by t1000–t2000).
    #[test]
    fn conserved_recycle_holds_the_acceptance_criteria() {
        use crate::sim::{self, Event, RECYCLE_PERMILLE};

        const LAST: u64 = 5_000;
        const SUPPLY: u64 = 52_148;
        /// The constants-derived full basket at settled prices:
        /// Food 10×1 + Entertainment 5×2 + Luxury 2×4. A4's denominator.
        const BASKET: u64 = 28;

        let mut world = town_world();
        let genesis_external = world.accounts.balance_of(world.external_id, Metal::Gold);
        let t20_price: HashMap<Good, Money> = HashMap::new();
        let mut t20_price = t20_price;
        let mut max_insolvent = 0u32;
        let mut max_sold_out = 0u32;
        let mut hunger_ticks: Vec<u64> = Vec::new();
        let mut vectors: HashMap<u64, Vec<(String, Money, u32)>> = HashMap::new();
        let mut windows: HashMap<u64, (u32, Money, Money, u32)> = HashMap::new();

        for t in 1..=LAST {
            let minted_before = world.accounts.total_minted(Metal::Gold);
            let burned_before = world.accounts.total_burned(Metal::Gold);
            let report = sim::tick(&mut world);

            let mut sold = 0u32;
            let mut turnover = Money::ZERO;
            let mut wages = Money::ZERO;
            let mut produced = 0u32;
            let mut recycled: Option<(Money, usize, Money)> = None;
            for event in &report.events {
                match event {
                    Event::Sold {
                        units, price: p, ..
                    } => {
                        sold += 1;
                        turnover = turnover.plus(p.times(*units));
                    }
                    Event::WagePaid { amount, .. } => wages = wages.plus(*amount),
                    Event::Produced { units, .. } => produced += *units,
                    Event::WentHungry { .. } => hunger_ticks.push(t),
                    Event::Recycled { pot, heads, share } => {
                        recycled = Some((*pot, *heads, *share))
                    }
                    _ => {}
                }
            }

            // --- A1: conservation and MATCHED ISSUE, every tick, every metal.
            // `tick` asserts the per-metal deltas internally; this pins the
            // stock and ties the event's own `pot` to the §8.4 logs, so a
            // report that lied about the pot would fail here.
            assert_eq!(
                world.accounts.total_money(Metal::Gold),
                Money::new(SUPPLY),
                "supply moved at t{t}"
            );
            assert_eq!(
                world
                    .accounts
                    .total_minted(Metal::Gold)
                    .minus(world.accounts.total_burned(Metal::Gold)),
                Money::new(SUPPLY),
                "net creation at t{t}"
            );
            let pot = recycled.map(|(pot, ..)| pot).unwrap_or(Money::ZERO);
            assert_eq!(
                world
                    .accounts
                    .total_minted(Metal::Gold)
                    .minus(minted_before),
                pot,
                "t{t}: minted != the Recycled event's pot"
            );
            assert_eq!(
                world
                    .accounts
                    .total_burned(Metal::Gold)
                    .minus(burned_before),
                pot,
                "t{t}: burned != the Recycled event's pot"
            );

            // --- A4 (stock half): nobody is left below one tick's basket,
            // at EVERY tick — not at sampled instants, since a decaying
            // trajectory can look solvent on the ticks you happen to check.
            if t >= 100 {
                for agent in &world.agents {
                    assert!(
                        world.accounts.balance_of(agent.id, Metal::Gold) >= Money::new(BASKET),
                        "t{t}: {} holds less than one basket",
                        agent.name
                    );
                }
            }

            // --- A8: External and the Mint never move, and there are no
            // orphan balances behind a removed agent.
            assert_eq!(
                world.accounts.balance_of(world.external_id, Metal::Gold),
                genesis_external,
                "External moved at t{t}"
            );
            assert_eq!(
                world.accounts.balance_of(world.mint_id, Metal::Gold),
                Money::ZERO,
                "the Mint account moved at t{t}"
            );

            // --- A6 (soak half): the levy is net progressive. Every tick in
            // the sampled windows, the per-head share strictly exceeds what
            // the poorest LEVIED agent paid — so the mechanic redistributes
            // upward-to-downward by construction, not by luck.
            if let Some((_, _, share)) = recycled
                && ((480..=500).contains(&t) || (1980..=2000).contains(&t))
            {
                {
                    let poorest_levy = world
                        .agents
                        .iter()
                        .map(|agent| {
                            sim::levy_amount(
                                world.accounts.balance_of(agent.id, Metal::Gold),
                                RECYCLE_PERMILLE,
                            )
                        })
                        .filter(|levy| *levy > Money::ZERO)
                        .min();
                    if let Some(levy) = poorest_levy {
                        assert!(
                            share > levy,
                            "t{t}: share {share} did not exceed the poorest levy {levy}"
                        );
                    }
                }
            }

            for (_, business) in world.businesses() {
                max_insolvent = max_insolvent.max(business.insolvent_ticks);
                max_sold_out = max_sold_out.max(business.sold_out_ticks);
            }
            if t == 20 {
                for good in Good::ALL {
                    if let Some(price) = world
                        .businesses()
                        .filter(|(_, business)| business.product == good)
                        .map(|(_, business)| business.price)
                        .min()
                    {
                        t20_price.insert(good, price);
                    }
                }
            }
            // --- A7: price tripwire. `adjust_price` has no ceiling, and A2's
            // floors are LOWER bounds — a slow ratchet would raise turnover
            // and sail past every other criterion.
            if t > 20 {
                for good in Good::ALL {
                    if let (Some(price), Some(base)) = (
                        world
                            .businesses()
                            .filter(|(_, business)| business.product == good)
                            .map(|(_, business)| business.price)
                            .min(),
                        t20_price.get(&good),
                    ) {
                        assert!(
                            price <= base.times(4),
                            "t{t}: {good} at {price} against a t20 {base}"
                        );
                    }
                }
            }

            if [500u64, 2_000, 5_000].contains(&t) {
                windows.insert(t, (sold, turnover, wages, produced));
            }
            if (600..=630).contains(&t) || t == 2_000 || t == 5_000 {
                vectors.insert(
                    t,
                    world
                        .agents
                        .iter()
                        .map(|agent| {
                            (
                                agent.name.clone(),
                                world.accounts.balance_of(agent.id, Metal::Gold),
                                agent.inventory.get(&Good::Food).copied().unwrap_or(0),
                            )
                        })
                        .collect(),
                );
            }
        }

        // --- A2: TRANSACTION VOLUME at three horizons. Floors sit at
        // roughly 70–90% of the measured steady state (82 Sold, 912 g
        // turnover, 662 g wages, 508 units) so the criterion has margin
        // without being vacuous. A rule-frozen town scores zero on all four.
        for horizon in [500u64, 2_000, 5_000] {
            let (sold, turnover, wages, produced) = windows[&horizon];
            assert!(sold >= 55, "t{horizon}: only {sold} Sold events");
            assert!(
                turnover >= Money::new(500),
                "t{horizon}: goods turnover {turnover}"
            );
            assert!(wages >= Money::new(600), "t{horizon}: wages {wages}");
            assert!(produced >= 450, "t{horizon}: produced {produced} units");
        }

        // --- A3: a LIVE limit cycle, measured under this spec's own
        // lowest-id remainder rule rather than inherited from the probe
        // arm. Recurrence alone is satisfied by a corpse; recurrence PLUS
        // lag-1 difference proves the town is moving through a cycle.
        assert_eq!(vectors[&600], vectors[&610], "no recurrence at lag 10");
        assert_ne!(vectors[&600], vectors[&601], "frozen: lag 1 is identical");
        assert_ne!(vectors[&600], vectors[&602], "frozen: lag 2 is identical");
        assert_ne!(vectors[&600], vectors[&605], "frozen: lag 5 is identical");
        // ...and the cycle is the same one 4,400 ticks later.
        assert_eq!(
            vectors[&600], vectors[&2_000],
            "the fixed point drifted by t2000"
        );
        assert_eq!(
            vectors[&2_000], vectors[&5_000],
            "the fixed point drifted by t5000"
        );

        // --- A5: hunger is a WARM-UP TRANSIENT and nothing else.
        assert!(
            hunger_ticks.iter().all(|&t| t <= 14),
            "hunger fired after the warm-up: {:?}",
            hunger_ticks
                .iter()
                .filter(|&&t| t > 14)
                .take(5)
                .collect::<Vec<_>>()
        );
        assert!(
            world.agents.iter().all(|agent| agent.hunger == 0),
            "an agent ends the soak hungry"
        );

        // --- A10: the CLOSE_INSOLVENT_TICKS standing obligation, discharged
        // with the number rather than assumed. The constant's doc comment
        // binds any pack that touches coffers to re-measure the healthy max
        // streak; this pack changes demand, therefore revenue, therefore
        // coffers.
        assert!(
            max_insolvent < sim::CLOSE_INSOLVENT_TICKS,
            "healthy max arrears streak {max_insolvent} reached the fuse"
        );
        assert_eq!(
            max_insolvent, 1,
            "the measured healthy streak moved from 1 — re-freeze CLOSE_INSOLVENT_TICKS \
             against the new number rather than widening this assertion"
        );
        // A7's second clause: no seller sits sold-out for long, which is
        // the other way a price ratchet starts.
        assert!(
            max_sold_out <= 20,
            "a sell-out streak ran to {max_sold_out}"
        );

        // Supporting observations — read ONLY now that A2 and A3 are green
        // (A0: these can never carry the gate on their own).
        assert_eq!(world.agents.len(), 30);
        assert_eq!(world.businesses().count(), 6);
        assert_eq!(
            world
                .agents
                .iter()
                .filter(|agent| agent.workplace.is_some())
                .count(),
            21
        );
        world.accounts.audit();
    }

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
        // multi-worker slots, seeded staff within headcount, and open
        // slots for the labor market to clear (pack 3: 21 jobs, 5 open —
        // see the manifest's tuning saga and deviation record)
        let mut total_open = 0;
        for (house, business) in world.businesses() {
            let staff = world.employees_of(house.id).len() as u32;
            let headcount: u32 = business.roles.values().map(|slot| slot.headcount).sum();
            assert!(
                staff <= headcount,
                "{} staffed within headcount",
                house.address
            );
            assert!(staff > 1, "{} is multi-worker at seed", house.address);
            total_open += headcount - staff;
        }
        assert_eq!(total_open, 5, "the boot-time hiring pool's slots");
        world.accounts.audit();
    }

    /// The conservation re-pin (re-pinned by pack 3's worldgen item —
    /// open headcounts widen the seeded bills, lux wages drop to
    /// solvency): town_world's per-metal totals are the entire money
    /// supply, forever. *(Amendment 22 corrects the reason this line used
    /// to give: it said "the audit holds them here every tick", and the
    /// audit provably cannot — `mint` moves the balance and `total_minted`
    /// together, so the identity holds for any mint. What holds the supply
    /// is that nothing minted at tick time, and since the conserved recycle,
    /// that phase 8 re-issues exactly the pot phase 7 burned — asserted per
    /// tick in `sim::tick`.)* Gold =
    /// coffers at three full-headcount bills (3×676 = 2028) + 16
    /// employed wallets of 120 + 14 savings of 3400 + External's 600
    /// fund (re-pinned by pack 4's fuse shortening). A worldgen change
    /// must change these constants consciously, in its own item.
    #[test]
    fn town_world_seeds_the_decided_metals() {
        let world = town_world();
        assert_eq!(world.accounts.total_money(Metal::Gold), Money::new(52148));
        assert_eq!(world.accounts.total_minted(Metal::Gold), Money::new(52148));
        assert_eq!(world.accounts.total_money(Metal::Silver), Money::new(300));
        assert_eq!(world.accounts.total_minted(Metal::Silver), Money::new(300));
        assert_eq!(world.accounts.total_money(Metal::Copper), Money::new(600));
        assert_eq!(world.accounts.total_minted(Metal::Copper), Money::new(600));
        world.accounts.audit();
    }

    /// Firm-lifecycle pack 1: each venue's owner is its first seeded
    /// worker — the owner-operator pattern, deterministic by spawn
    /// order. All six owners are employed, so the 9 permanently
    /// unemployed stay non-owners and the emigration pool survives.
    #[test]
    fn town_world_seeds_owner_operators() {
        let world = town_world();
        let expected = [
            ("Greenrow Farm", "alice"),
            ("Longacre Farm", "ed"),
            ("Gilt Curtain Theater", "ivan"),
            ("The Brass Bell", "karl"),
            ("Karat & Co", "marco"),
            ("Silverthread Atelier", "otto"),
        ];
        for (address, owner_name) in expected {
            let (house, business) = world
                .businesses()
                .find(|(house, _)| house.address == address)
                .expect("seeded venue");
            let owner = world
                .agent(business.owner)
                .expect("owner is a living agent");
            assert_eq!(owner.name, owner_name, "{address}");
            // owner-operator: employed at their own venue
            assert_eq!(owner.workplace, Some(house.id), "{address}");
        }
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
        // Warm-up excluded. RE-PINNED 10 → 15 by conserved-recycle pack 2,
        // and the reason is measured, not guessed: under the cure the last
        // agent to enter the Food market (bram) makes their first purchase
        // at **t15**, so a rolling 5-tick window opening before then is
        // testing warm-up, not steady state. The arithmetic minimum that
        // passes is 11 (15 − WINDOW + 1); 15 is the principled value — the
        // window must not open before every agent is in the market. The
        // criterion's SHAPE is untouched: every agent still completes ≥1
        // Food purchase in every rolling 5-tick window of the evaluated
        // span, and in the cured steady state the worst gap any agent
        // posts is 2 ticks (measured to t5000).
        const FROM: u64 = 15;
        const WINDOW: u64 = 5;
        let floor = Money::new(1);

        let mut world = town_world();
        let mut food_ticks: HashMap<AgentId, Vec<u64>> = HashMap::new();
        let mut cheapest: HashMap<Good, Vec<Money>> = HashMap::new();
        // per business: (rises, falls) after warm-up
        let mut moved: HashMap<AgentId, (u32, u32)> = HashMap::new();
        let mut quits = 0u32;
        let mut drew: HashMap<AgentId, u32> = HashMap::new();
        // criterion 7 (firm-lifecycle pack 2): the tuned town never
        // closes. A max-tracking map alone cannot prove that — a closed
        // venue simply drops out of `businesses()` and scores as "never
        // got high" — so it is paired with a live-count check and a
        // hand-written Closed tally. Worldgen's `match event` arms end in
        // `_ => {}`, so neither Closed nor LaidOff forces at compile time
        // here: these assertions have no compiler help and must be
        // written deliberately.
        let mut distress: HashMap<AgentId, u32> = HashMap::new();
        let mut closures = 0u32;
        let mut foundings = 0u32;

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
                    Event::Quit { .. } => quits += 1,
                    Event::Closed { .. } => closures += 1,
                    Event::Founded { .. } => foundings += 1,
                    // t >= 20 like criterion 5: the t1–2 boot burst (the
                    // seeded-wallet spending wave flowing through the
                    // coffers) would satisfy an unguarded tally even for
                    // a draw that broke after warm-up.
                    Event::ProfitDrawn { business, .. } if t >= 20 => {
                        *drew.entry(*business).or_default() += 1
                    }
                    _ => {}
                }
            }
            // 5. (firm-lifecycle pack 1) the sink is dead: from tick 20
            //    every coffer sits at or under the retained buffer —
            //    DRAW_BUFFER_BILLS full-staffing bills plus outstanding
            //    arrears. Phase 7 can only debit coffers after the draw
            //    (settlements), so the post-tick bound is the phase-6
            //    bound or tighter.
            if t >= 20 {
                for (house, business) in world.businesses() {
                    let bound = business
                        .wage_bill()
                        .times(crate::sim::DRAW_BUFFER_BILLS)
                        .plus(business.owed_total());
                    assert!(
                        world.accounts.balance_of(business.id, Metal::Gold) <= bound,
                        "{}'s coffer exceeds the draw buffer at t{t}",
                        house.address
                    );
                }
            }
            // 7. (firm-lifecycle pack 2) the tuned town never closes.
            //    Sampled post-tick, which IS the next tick's phase-start
            //    snapshot — the value phase 6's closure pass will read.
            for (_, business) in world.businesses() {
                let entry = distress.entry(business.id).or_default();
                *entry = (*entry).max(business.insolvent_ticks);
            }
            assert_eq!(
                world.businesses().count(),
                6,
                "the tuned town's venue count moved at t{t} — a death or a birth"
            );
            // 8. (firm-lifecycle pack 3) and NOTHING is founded — named
            //    at its cause rather than only at its symptom: founding's
            //    carrying-capacity gate needs a good to drop below two
            //    sellers, and in the tuned town none ever does.
            for good in Good::ALL {
                assert!(
                    world
                        .businesses()
                        .filter(|(_, b)| b.product == good)
                        .count()
                        >= 2,
                    "{good} fell below two sellers at t{t} — founding's gate would open"
                );
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

        // 4. zero quits across the whole soak (pack 3): the tuned town
        //    is solvent, so nobody walks out — no-spurious-quits as an
        //    assertion, not a hope. A quitting town is a sick town; the
        //    quit mechanism itself is demonstrated in the sim:: tests.
        assert_eq!(quits, 0, "the tuned town fired {quits} spurious quits");

        // 6. (firm-lifecycle pack 1) profit flows: every venue drew at
        //    least once AFTER warm-up — coffers recirculate to owners in
        //    steady state instead of pooling (the fuse cure landed;
        //    magnitudes in the pack-1 ledger; measured minimum ~4
        //    post-warm-up draws at Longacre).
        for (house, business) in world.businesses() {
            assert!(
                drew.get(&business.id).copied().unwrap_or(0) > 0,
                "{} never drew profit across the soak",
                house.address
            );
        }

        // 7. (firm-lifecycle pack 2) NOTHING closes in the tuned town.
        //    Three assertions, because no one of them is sufficient: the
        //    counter never reaches the threshold, no venue vanished (the
        //    per-tick count above), and no closure was ever narrated.
        //    Measured 2026-08-30: exactly ten one-tick flickers over the
        //    whole span, all at Longacre Farm, owing 2–11g against a 140g
        //    bill — observed maximum 1, so CLOSE_INSOLVENT_TICKS = 12
        //    clears it by eleven ticks. The other five venues never
        //    accrue a coin. This is a REGRESSION GUARD, not a tuning
        //    constraint: it passes at any threshold ≥ 2, so it proves the
        //    constant is not too LOW and says nothing about it being too
        //    high. Only the 200-tick soak can show the fuse still fires.
        assert_eq!(closures, 0, "the tuned town closed {closures} venues");
        // Hand-written, like criterion 7: this match ends in `_ => {}`,
        // so `Event::Founded` forces nothing here and a missing arm would
        // ship green while observing nothing.
        assert_eq!(foundings, 0, "the tuned town founded {foundings} venues");
        for (house, business) in world.businesses() {
            let max = distress.get(&business.id).copied().unwrap_or(0);
            assert!(
                max < crate::sim::CLOSE_INSOLVENT_TICKS,
                "{} reached {max} insolvent ticks against a threshold of {}",
                house.address,
                crate::sim::CLOSE_INSOLVENT_TICKS
            );
        }
    }

    /// The pack-3 soak (town-colony spec, pack-3 criteria): 50 ticks
    /// from `town_world` with the labor market live. Constants are
    /// iterated until this and the 100-tick soak above BOTH hold — the
    /// union is the gate — then frozen.
    #[test]
    fn town_soak_reaches_near_full_employment() {
        use crate::role::Role;
        use crate::sim::{self, Event};

        const LAST: u64 = 50;
        /// "Near-full employment": the measured, frozen target — see the
        /// pack-3 manifest's deviation record for why it is not 27/30.
        const NEAR_FULL: usize = 21;
        /// A rose-without-falling wage series must be flat this long at
        /// the end — proof the rise plateaued (the affordability gate or
        /// a filled slot ended it) rather than compounding unbounded.
        const PLATEAU: usize = 10;

        let mut world = town_world();
        let mut first_hire: Option<u64> = None;
        let mut reached_full: Option<u64> = None;
        // per (business, role): the wage in force before each tick
        let mut wages: HashMap<(AgentId, Role), Vec<Money>> = HashMap::new();

        for t in 1..=LAST {
            for (_, business) in world.businesses() {
                for &role in Role::ALL.iter() {
                    if let Some(slot) = business.roles.get(&role) {
                        wages
                            .entry((business.id, role))
                            .or_default()
                            .push(slot.wage);
                    }
                }
            }
            let report = sim::tick(&mut world);
            if first_hire.is_none()
                && report
                    .events
                    .iter()
                    .any(|event| matches!(event, Event::Hired { .. }))
            {
                first_hire = Some(t);
            }
            let employed = world
                .agents
                .iter()
                .filter(|agent| agent.workplace.is_some())
                .count();
            match reached_full {
                None if employed >= NEAR_FULL => reached_full = Some(t),
                Some(reached) => assert!(
                    employed >= NEAR_FULL,
                    "employment fell back to {employed} at tick {t} after reaching {NEAR_FULL} at tick {reached}"
                ),
                None => {}
            }
        }

        // 1. the seeded unemployed start getting hired within a few ticks
        assert!(
            first_hire.is_some_and(|t| t <= 3),
            "first hire at {first_hire:?}, not within 3 ticks"
        );
        // 2. near-full employment by the soak's end, held once reached
        assert!(
            reached_full.is_some_and(|t| t <= LAST),
            "never reached {NEAR_FULL} employed within {LAST} ticks"
        );

        // 3. no posted wage rises monotonically: never strictly
        //    increasing across the span, and a series that rose without
        //    ever falling must have plateaued — the unit tests pin the
        //    affordability gate itself; this pins the absence of an
        //    ungated unbounded rise in the real town
        for ((business, role), series) in &wages {
            let strictly_rising = series.windows(2).all(|w| w[1] > w[0]);
            assert!(
                !strictly_rising,
                "{business:?}/{role} wage strictly rising all span"
            );
            let never_fell = series.windows(2).all(|w| w[1] >= w[0]);
            let rose = series.last() > series.first();
            if never_fell && rose {
                // A founded firm's wage series starts mid-soak, so it can
                // be shorter than PLATEAU — `series.len() - PLATEAU`
                // would underflow into a panic rather than a failure.
                // Too short to judge a plateau is not evidence of one.
                if series.len() < PLATEAU {
                    continue;
                }
                let tail = &series[series.len() - PLATEAU..];
                assert!(
                    tail.windows(2).all(|w| w[1] == w[0]),
                    "{business:?}/{role} rose without falling and never plateaued: {series:?}"
                );
            }
            // and no slow sawtooth escapes both checks above: every
            // wage ends within ~2 raise-steps of where it started —
            // the cascade's measured overshoot, not an unbounded climb
            let bound = crate::market::stepped_wage(crate::market::stepped_wage(
                *series.first().expect("series has one entry per tick"),
            ));
            assert!(
                *series.last().expect("series has one entry per tick") <= bound,
                "{business:?}/{role} climbed past the cascade bound: {series:?}"
            );
        }
    }

    /// RE-CUT from decay-driven to SHOCK-DRIVEN by conserved-recycle pack 2.
    ///
    /// This was `town_soak_population_moves_both_directions`, and its entire
    /// premise was that the town decays on its own: it asserted somebody
    /// left, that a closure freed a house, that founding answered it, and
    /// that the population dipped below the seed count. **The cure removes
    /// the decay, so six of its assertions invert and a seventh panics.**
    /// Measured to t5000 under `RECYCLE_PERMILLE = 20`: population 30, firms
    /// 6, employed 21, zero closures, zero foundings, zero departures, zero
    /// quits, hunger confined to a t2–t14 warm-up.
    ///
    /// Every retired criterion, with the measurement that retired it — so
    /// this is a re-cut on the record, not a quiet weakening:
    ///
    /// | old criterion | retired by |
    /// |---|---|
    /// | `!departed_ids.is_empty()` (:896) | 0 departures at every horizon |
    /// | `first_answer_after_departure.is_some()` (:898) | no departure, so nothing to answer |
    /// | `dipped` (:902) | population never leaves 30 |
    /// | `min_population >= 15` (:928) | subsumed: the minimum IS 30 |
    /// | `total_births >= 3` (:930) | `plan_founding` returns `None` on every tick — 2 sellers of every good, always |
    /// | anti-churn over found→close cycles (:~965) | zero foundings and zero closures to churn |
    /// | `chain_reoccupied.expect(...)` (:961) — a PANIC, not an assert | no closure ever frees a house |
    /// | `closed.len() >= 3` (:1000) | 0 closures |
    ///
    /// What survives is the part that was always the point: **the lifecycle
    /// must still WORK when something actually goes wrong.** A quiet town
    /// and a dead one look identical from population alone (measured: the
    /// refused demurrage arms froze population by rule, because with no Food
    /// seller nobody can be judged destitute), so this drives the cured town
    /// to t100, force-closes a Food seller, and asserts the whole chain
    /// fires: the house frees, founding answers it, someone is hired into
    /// it, the town does not collapse, and prices do not run away.
    ///
    /// **It also PINS a live defect rather than asserting it away.** The
    /// Food founding template posts headcount 2 against the 4-headcount
    /// venue it replaces, so the phoenix cycle permanently costs two jobs
    /// and installs standing hunger. Measured over t118–t400 after the
    /// shock: employment settles at **19**, never back to 21, and 5–8
    /// hungry agent-ticks fire every tick indefinitely. That is the
    /// conserved-recycle spec's A11 headcount clause failing, it is pack 3's
    /// founding-template sweep to answer, and it is asserted here at its
    /// measured value so that fixing it shows up as a deliberate re-pin.
    #[test]
    fn town_survives_and_rebuilds_after_a_forced_closure() {
        use crate::sim::{self, Event};

        // Pack 3 moved the shock from t100 to t500, per the spec's A11:
        // t500 is deep inside the measured fixed point, so the recovery is
        // read against a settled town rather than one still finding it.
        const SHOCK: u64 = 500;
        const LAST: u64 = 1_200;
        const SEED_POPULATION: usize = 30;
        /// Measured recovery window. Two recoveries happen, and they are
        /// NOT the same tick, so both are pinned rather than conflated:
        /// the shock at t500 is answered by a founding at t570 and full
        /// re-staffing at **t571** (`STAFFING_LAG`), but the food pipeline
        /// takes eight ticks more to refill, so the last hunger event is
        /// **t579**. A11's window must cover both, so `K = 79`.
        const K: u64 = 79;
        /// Ticks from the shock to full re-staffing — the faster of the
        /// two recoveries, pinned separately so a regression in either is
        /// legible on its own.
        const STAFFING_LAG: u64 = 71;

        let mut world = town_world();
        for _ in 1..SHOCK {
            sim::tick(&mut world);
        }

        // Pre-shock: the cured town is quiet, and quiet is the thing this
        // test refuses to accept as evidence of health on its own.
        assert_eq!(world.agents.len(), SEED_POPULATION);
        assert_eq!(world.businesses().count(), 6);
        let employed_before = world
            .agents
            .iter()
            .filter(|agent| agent.workplace.is_some())
            .count();
        assert_eq!(employed_before, 21, "the cured town is fully staffed");
        let food_price_before = world
            .businesses()
            .filter(|(_, business)| business.product == Good::Food)
            .map(|(_, business)| business.price)
            .min()
            .expect("food is sold pre-shock");
        // A11's band: the [min, max] of the cheapest Food price over
        // t400..=t500, which the recovery must return inside. Measured on
        // the fixed point, where it is a single value.
        let (band_lo, band_hi) = (food_price_before, food_price_before);

        // THE SHOCK: kill a Food seller outright. Not a tuning nudge — the
        // sector that feeds the town loses half its capacity in one tick.
        let victim = world
            .businesses()
            .filter(|(_, business)| business.product == Good::Food)
            .map(|(house, _)| house.id)
            .next()
            .expect("a food seller to kill");
        let receipt = world.close_business(victim).expect("the victim is live");
        assert_eq!(
            receipt.laid_off.len(),
            4,
            "the shock should cost four jobs — if this moved, the shock changed size"
        );
        assert_eq!(world.businesses().count(), 5, "the shock landed");

        let mut founded_into: Option<(u64, HouseId)> = None;
        let mut hired_after_founding: Option<u64> = None;
        let mut founded_business: Option<AgentId> = None;
        let mut closures_after_shock = 0u32;
        let mut departures_after_shock = 0u32;
        let mut min_population = world.agents.len();
        let mut worst_food_price = food_price_before;
        let mut hunger_after_recovery = 0u32;
        let mut recovered_at: Option<u64> = None;
        let mut volume_shortfalls = 0u32;
        let mut ordering_violations: Vec<(u64, String)> = Vec::new();

        for t in SHOCK..=LAST {
            // A11's ORDERING CLAUSE, checked BEFORE the tick so the
            // pre-dividend wallet is the one the destitution decide will
            // read. The burn/mint split forecloses paying the dividend
            // ahead of that decide (only row 8 permits the payout leg), so
            // an agent can in principle be swept to External at phase 7
            // while the share that would have saved them lands at phase 8.
            // This looks for exactly that case.
            let cheapest_food_now = world
                .businesses()
                .filter(|(_, business)| business.product == Good::Food)
                .map(|(_, business)| business.price)
                .min();
            let about_to_depart: Vec<(AgentId, String, Money)> = world
                .agents
                .iter()
                .map(|agent| {
                    (
                        agent.id,
                        agent.name.clone(),
                        world.accounts.balance_of(agent.id, Metal::Gold),
                    )
                })
                .collect();

            let report = sim::tick(&mut world);
            for event in &report.events {
                match event {
                    Event::Founded {
                        house,
                        business,
                        good,
                        ..
                    } => {
                        if *good == Good::Food && founded_into.is_none() {
                            founded_into = Some((t, *house));
                            founded_business = Some(*business);
                        }
                    }
                    Event::Hired { business, .. } => {
                        if founded_business == Some(*business) && hired_after_founding.is_none() {
                            hired_after_founding = Some(t);
                        }
                    }
                    Event::Closed { .. } => closures_after_shock += 1,
                    Event::Departed { agent, .. } => {
                        departures_after_shock += 1;
                        if let (Some(cheapest), Some((_, name, before))) = (
                            cheapest_food_now,
                            about_to_depart.iter().find(|(id, ..)| id == agent).cloned(),
                        ) {
                            // would this tick's dividend have cleared the
                            // cheapest posted Food price for them?
                            let share = report.events.iter().find_map(|e| match e {
                                Event::Recycled { share, .. } => Some(*share),
                                _ => None,
                            });
                            if let Some(share) = share
                                && before.plus(share) >= cheapest
                            {
                                ordering_violations.push((t, name));
                            }
                        }
                    }
                    Event::WentHungry { .. } if t > SHOCK + K => hunger_after_recovery += 1,
                    _ => {}
                }
            }
            // A2's volume floors must hold AGAIN once the town has
            // recovered — a town that survives on paper but trades at half
            // its old volume has not recovered.
            if t > SHOCK + K {
                let sold = report
                    .events
                    .iter()
                    .filter(|e| matches!(e, Event::Sold { .. }))
                    .count();
                if sold < 55 {
                    volume_shortfalls += 1;
                }
            }
            if recovered_at.is_none()
                && world
                    .agents
                    .iter()
                    .filter(|agent| agent.workplace.is_some())
                    .count()
                    >= employed_before
            {
                recovered_at = Some(t);
            }
            min_population = min_population.min(world.agents.len());
            if let Some(price) = world
                .businesses()
                .filter(|(_, business)| business.product == Good::Food)
                .map(|(_, business)| business.price)
                .min()
            {
                worst_food_price = worst_food_price.max(price);
            }
        }

        // 1. THE CHAIN FIRES — the whole point of keeping this soak: the
        //    shock frees premises, founding answers the scarcity it caused,
        //    and the new venue is staffed. Four observations, not "some
        //    founding happened".
        let (founded_at, house) = founded_into.expect(
            "no Food venue was founded after the shock — the lifecycle is entombed, \
             which is exactly what a population-flat criterion would have missed",
        );
        assert!(
            world.is_fully_vacant(victim),
            "the closure did not leave its house vacant, so the chain's middle link is broken"
        );
        // MEASURED, and DIFFERENT from the retired soak's same-house
        // criterion: founding takes the FIRST fully-vacant house in houses
        // order, and the town's two spare residences sort ahead of any
        // house a closure frees. So the freed house is genuinely vacant and
        // available, and the founder still picks a spare — measured here as
        // HouseId(4) ("5 Weir Cottage") against a victim at HouseId(6).
        // The old criterion asserted the same house because on the decaying
        // trajectory the spares had already been taken; asserting it here
        // would be asserting an artifact of the old trajectory.
        assert_ne!(
            house, victim,
            "founding chose the freed house — the spare-houses-sort-first reading above is \
             stale and this criterion needs re-measuring, not deleting"
        );
        // A12's reachability window, PINNED at its measured value — and
        // the spec's own figure for it is refuted here (spec erratum 1).
        // A12 said founding must land "within `FOUND_SIGNAL_TICKS + 2`
        // ticks", i.e. 4. That figure counts only the sell-out streak and
        // forgets what must happen first: the surviving seller's price has
        // to climb off `PRICE_FLOOR` to the viability signal before the
        // scarcity tier can fire at all. Measured, the answer also depends
        // on where in the 10-tick limit cycle the shock lands — a shock at
        // t100 is answered in 13 ticks, one at t500 in 70. The sim is
        // deterministic and seedless, so the honest form is an exact pin
        // rather than a window whose width would be guesswork.
        const FOUNDING_LAG: u64 = 70;
        assert_eq!(
            founded_at,
            SHOCK + FOUNDING_LAG,
            "founding answered the shock at t{founded_at}, not the pinned t{}",
            SHOCK + FOUNDING_LAG
        );
        let hired_at = hired_after_founding.expect("nobody was hired into the founded venue");
        assert!(
            hired_at >= founded_at,
            "hiring cannot precede the founding it staffs"
        );

        // 2. THE TOWN DOES NOT COLLAPSE. One forced closure must not
        //    cascade: no further firm dies, nobody leaves, and the
        //    population never dips.
        assert_eq!(
            closures_after_shock, 0,
            "the forced closure cascaded into {closures_after_shock} more"
        );
        assert_eq!(
            departures_after_shock, 0,
            "{departures_after_shock} residents left after the shock"
        );
        assert_eq!(min_population, SEED_POPULATION, "the population dipped");
        assert_eq!(
            world.businesses().count(),
            6,
            "the town did not get its sixth venue back"
        );

        // 3. PRICE TRIPWIRE (A7). `adjust_price` has no ceiling, and a
        //    demand shock is exactly where a runaway would start — measured
        //    elsewhere at 179g and diverging when outside demand was let in.
        //    Food's cheapest posted price must stay inside 4× its pre-shock
        //    level at every tick of the recovery, not merely return there.
        assert!(
            worst_food_price <= food_price_before.times(4),
            "food price ran to {worst_food_price} against a pre-shock {food_price_before}"
        );

        // 4. FULL RECOVERY (A11) — RE-PINNED by pack 3, in the direction
        //    pack 2 said to watch for. Pack 2 shipped this block asserting
        //    the DEFECT at its measured values: employment stuck at 19 and
        //    hunger that never stopped, because the Food entrant posted
        //    headcount 2 against the dead venue's 4. Pack 3's sweep raised
        //    Food's founding headcount to 4 — and ONLY Food's, because the
        //    churn the old sweep blamed on "bigger entrants" was
        //    Entertainment's — so the town now rebuilds to its full size.
        //    Both numbers moved exactly as that block predicted they would.
        let employed_after = world
            .agents
            .iter()
            .filter(|agent| agent.workplace.is_some())
            .count();
        assert_eq!(
            employed_after, employed_before,
            "the town did not rebuild to its pre-shock headcount"
        );
        assert_eq!(
            recovered_at,
            Some(SHOCK + STAFFING_LAG),
            "re-staffing moved off its pinned lag of {STAFFING_LAG} ticks"
        );
        assert_eq!(
            hunger_after_recovery, 0,
            "hunger persisted past the recovery window — the under-replacement defect is back"
        );

        // 5. A11's remaining clauses: the price band, the volume floors,
        //    and the ordering clause the burn/mint split owes.
        assert!(
            (band_lo..=band_hi).contains(
                &world
                    .businesses()
                    .filter(|(_, business)| business.product == Good::Food)
                    .map(|(_, business)| business.price)
                    .min()
                    .expect("food is sold after recovery")
            ),
            "the cheapest Food price did not return to its pre-shock band"
        );
        assert_eq!(
            volume_shortfalls, 0,
            "{volume_shortfalls} ticks after recovery traded below A2's floor"
        );
        assert!(
            ordering_violations.is_empty(),
            "phase-7-levy/phase-8-payout ordering cost someone their home: {ordering_violations:?} \
             — record this in the ledger as a measured asymmetry of the burn/mint split"
        );

        // no orphan balances: the forced closure's own account is empty
        for metal in Metal::ALL {
            assert_eq!(
                world.accounts.balance_of(receipt.business, metal),
                Money::ZERO,
                "the closed business kept {metal}"
            );
        }
        world.accounts.audit();
    }
}
