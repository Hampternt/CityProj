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

/// Wallet each employed agent starts with — a cushion over one wage so
/// early price spikes don't starve anyone before pay day settles in.
const EMPLOYED_WALLET: u64 = 60;
/// Savings each seeded-unemployed agent lives off until pack 3's labor
/// market gives them income — sized to keep them buying Food through the
/// 100-tick soak (the liveness criterion applies to them too).
const UNEMPLOYED_SAVINGS: u64 = 2500;
/// External's gold settlement fund: pack 4's immigration grubstakes draw
/// from here; until then it sits on the books, audited like everything.
const SETTLEMENT_FUND: u64 = 600;
const SILVER_SAVINGS: u64 = 10;
const COPPER_SAVINGS: u64 = 20;

/// Every named resident, spawn order = ascending `AgentId`. The first 24
/// fill business slots in declaration order; the rest are seeded
/// unemployed.
const NAMES: [&str; 28] = [
    "alice", "bob", "carol", "dave", "ed", "fiona", "george", "hana", "ivan", "judit", "karl",
    "lena", "marco", "nadia", "otto", "petra", "quinn", "rosa", "sam", "tessa", "ulf", "vera",
    "will", "xenia", "yara", "zeno", "mira", "tomas",
];

/// The shipped town (town-colony spec, `town_world` contract): 28 agents
/// across 4 occupied residences (7 each), 2 zero-occupant spare
/// residences (pack 4's landing pads), and 8 multi-worker businesses over
/// all three Goods — three competing Food farms among them. 24 agents are
/// seeded employed, 4 unemployed (they live off savings until pack 3's
/// labor market). Each business is pre-funded with one full-staffing wage
/// bill; External holds the settlement fund. Deterministic and seedless;
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

    // (address, product, price, wage, headcount) — supply outbuilds
    // demand on every good: food 8×40=320 vs 280 eaten, entertainment
    // 8×20=160 vs 140, luxury 8×8=64 vs 56.
    let businesses = [
        ("Greenrow Farm", Good::Food, 2u64, 35u64, 3u32),
        ("Longacre Farm", Good::Food, 2, 35, 3),
        ("Stonefield Farm", Good::Food, 3, 35, 2),
        ("Gilt Curtain Theater", Good::Entertainment, 2, 32, 3),
        ("The Brass Bell", Good::Entertainment, 2, 32, 3),
        ("Riverlight Hall", Good::Entertainment, 3, 32, 2),
        ("Karat & Co", Good::Luxury, 5, 40, 4),
        ("Silverthread Atelier", Good::Luxury, 6, 40, 4),
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
        world.accounts.mint(business, Metal::Gold, bill);
        for _ in 0..headcount {
            let name = NAMES[next_name];
            let home = residences[next_name / 7];
            next_name += 1;
            let worker = world.spawn_agent(name, Some(home), Some(house));
            world.agent_mut(worker).expect("just spawned").employed_role = Some(Role::Labourer);
        }
    }
    // The rest are unemployed until pack 3's labor market hires them.
    while next_name < NAMES.len() {
        world.spawn_agent(NAMES[next_name], Some(residences[next_name / 7]), None);
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
        // 28 agents, 24 employed, 4 unemployed
        assert_eq!(world.agents.len(), 28);
        let employed = world
            .agents
            .iter()
            .filter(|a| a.workplace.is_some())
            .count();
        assert_eq!(employed, 24);
        // every employed agent has a slotted role (they must earn)
        assert!(
            world
                .agents
                .iter()
                .filter(|a| a.workplace.is_some())
                .all(|a| a.employed_role.is_some())
        );
        // 14 houses: 4 occupied residences + 2 vacant spares + 8 premises
        assert_eq!(world.houses.len(), 14);
        assert_eq!(world.businesses().count(), 8);
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
}
