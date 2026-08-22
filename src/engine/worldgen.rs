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
    /// supply, forever — the audit holds them here every tick. Gold =
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
        const FROM: u64 = 10; // warm-up excluded
        const WINDOW: u64 = 5;
        let floor = Money::new(1);

        let mut world = town_world();
        let mut food_ticks: HashMap<AgentId, Vec<u64>> = HashMap::new();
        let mut cheapest: HashMap<Good, Vec<Money>> = HashMap::new();
        // per business: (rises, falls) after warm-up
        let mut moved: HashMap<AgentId, (u32, u32)> = HashMap::new();
        let mut quits = 0u32;
        let mut drew: HashMap<AgentId, u32> = HashMap::new();

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
                    Event::ProfitDrawn { business, .. } => *drew.entry(*business).or_default() += 1,
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
        //    least once across the soak — coffers recirculate to owners
        //    instead of pooling (the fuse cure landed; magnitudes in the
        //    pack-1 ledger).
        for (house, business) in world.businesses() {
            assert!(
                drew.get(&business.id).copied().unwrap_or(0) > 0,
                "{} never drew profit across the soak",
                house.address
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

    /// The pack-4 soak (town-colony spec): 200 ticks from `town_world`
    /// with every mechanic live. The measured breathing chain: the
    /// unemployed dis-save and the destitute leave (~t127 on, every
    /// metal swept); the demand shock bites a venue's payroll; quits
    /// open slots; the K-aged vacancy pulls grubstaked immigrants
    /// (~t182 on) who are hired within a tick or two. The audit runs
    /// inside every `tick`, so any §8 break panics the soak.
    #[test]
    fn town_soak_population_moves_both_directions() {
        use crate::sim::{self, Event};

        const LAST: u64 = 200;
        let mut world = town_world();
        let seed_population = world.agents.len();
        let mut departed_ids: Vec<AgentId> = Vec::new();
        let mut first_departed: Option<u64> = None;
        let mut first_arrived_after_departure: Option<u64> = None;
        let mut dipped = false;
        let mut rose = false;
        for t in 1..=LAST {
            let before = world.agents.len();
            let report = sim::tick(&mut world);
            for event in &report.events {
                match event {
                    Event::Departed { agent, .. } => {
                        departed_ids.push(*agent);
                        first_departed.get_or_insert(t);
                    }
                    // only an arrival at a strictly later tick than the
                    // first departure counts — the pull ANSWERING the
                    // shock, which a boot transient cannot satisfy
                    // (phase order puts Arrived before Departed inside
                    // one tick, so strictly-later is the honest bar)
                    Event::Arrived { .. } if first_departed.is_some_and(|d| t > d) => {
                        first_arrived_after_departure.get_or_insert(t);
                    }
                    _ => {}
                }
            }
            let after = world.agents.len();
            dipped |= after < seed_population;
            rose |= after > before;
        }
        // population moves in BOTH directions (spec observable), on the
        // per-tick series — offsetting same-tick moves cannot fake it
        assert!(!departed_ids.is_empty(), "nobody left in {LAST} ticks");
        assert!(
            first_arrived_after_departure.is_some(),
            "no arrival answered the shock (first departure at {first_departed:?})"
        );
        assert!(dipped, "population never fell below the seed count");
        assert!(rose, "population never rose across a tick");
        // no orphan balances: every leaver's account is empty on every
        // metal — the per-account check the totals-only audit cannot
        // make (ids are never reused, so these must still be zero)
        for leaver in departed_ids {
            for metal in Metal::ALL {
                assert_eq!(
                    world.accounts.balance_of(leaver, metal),
                    Money::ZERO,
                    "orphan balance parked on departed {leaver:?}"
                );
            }
        }
        world.accounts.audit();
    }
}
