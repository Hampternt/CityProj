//! The FROZEN pre-cure baseline (conserved-recycle pack 2, acceptance
//! criterion A9).
//!
//! These are verbatim copies of the three town soaks as they stood at
//! `c11a73e`, immediately before the recycle was given a magnitude — same
//! bodies, same criteria, same literals — with exactly ONE edit each:
//! `sim::tick(&mut world)` becomes `sim::tick_with_rate(&mut world, 0)`.
//!
//! Why copies rather than one body parameterised over the rate: the cured
//! soaks in `worldgen.rs` are re-cut by this pack (the cure inverts six of
//! the 200-tick soak's assertions and panics a seventh). If one body served
//! both, re-cutting the cured criteria would necessarily reshape the null
//! criteria too — laundering exactly the regression A9 exists to detect.
//! Frozen copies cost ~600 duplicated lines.
//!
//! **What they do and do NOT freeze — corrected 2026-09-08 after a review
//! caught the original claim overreaching.** This file said the copies "buy
//! a baseline that cannot drift". They cannot: a copied body still reads
//! LIVE production constants, so any tuning change moves the trajectory
//! here without editing a line of this file — pack 3's founding-headcount
//! re-freeze did exactly that, and every criterion kept passing on its
//! slack. What is frozen is the CRITERIA. What is now also pinned, at the
//! bottom of the 200-tick twin, is the trajectory itself.
//!
//! **These must never be "fixed" to match a re-cut cured soak.** If one goes
//! red, either the recycle changed behavior at rate 0 — a bug in the
//! mechanic — or some other constant moved the baseline, which is legitimate
//! but must be re-pinned deliberately with the cause named. Establish which
//! before touching anything: revert the suspected change and re-run.

// (the module is gated `#[cfg(test)]` at its declaration in engine/mod.rs;
// a second inner gate here would be dead and could mask that one's removal)

use std::collections::HashMap;

use crate::agent::AgentId;
use crate::engine::worldgen::town_world;
use crate::goods::Good;
use crate::housing::HouseId;
use crate::metal::Metal;
use crate::money::Money;

/// The spec's pinned soak exit criteria (town-colony spec, "Pinned
/// soak exit criteria"): the tuning constants above were iterated
/// until this held, then frozen. 100 ticks, evaluated from tick 10
/// (warm-up excluded); the audit runs inside every `tick`, so any §8
/// break panics the soak.
#[test]
fn null_town_soak_holds_the_pinned_exit_criteria() {
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
        let report = sim::tick_with_rate(&mut world, 0);
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
fn null_town_soak_reaches_near_full_employment() {
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
        let report = sim::tick_with_rate(&mut world, 0);
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

/// The pack-4 soak (town-colony spec): 200 ticks from `town_world`
/// with every mechanic live. The measured breathing chain
/// (re-measured under the firm-lifecycle draw — pack-1 ledger): the
/// unemployed dis-save and the destitute leave (~t127 on, every
/// metal swept); the demand shock bites venue payrolls — earlier
/// and broader under the draw, since the coffer cushions that used
/// to absorb it are drawn down by design (quit churn from ~t134,
/// three venues, was ~t174 at one); quits open slots; the K-aged
/// vacancy pulls grubstaked immigrants who are hired within a tick
/// or two. The audit runs inside every `tick`, so any §8 break
/// panics the soak.
///
/// **Re-measured under pack-2 closure, 2026-08-30.** The spec's
/// named re-cut turned out NOT to be needed and no criterion was
/// weakened — see the pack-2 ledger. Arrivals still answer the shock
/// (t183/184/185, after the first departure at t127), because
/// closure DELETES the arrears-carrying venue that the Arrive
/// exclusion refuses to recruit for, leaving the survivor's
/// post-layoff vacancies clean and pull-eligible. What the re-measure
/// did find is pinned below: pack 2's closure cascade, and pack 3's
/// answer to it.
///
/// **Pack 2 (closure without founding) emptied the town.** Every
/// venue died — t140/t153/t156/t171/t172 and Greenrow Farm at t201,
/// one tick past this horizon — leaving no businesses at all at
/// population 4. Measured against a pack-1 baseline, only three of
/// those were the cascade's own: Longacre, The Brass Bell and Gilt
/// Curtain already carried terminal arrears streaks of 73 / 60 / 57
/// ticks with closure absent and simply never died, while Karat &
/// Co, Silverthread and Greenrow carried ZERO arrears for 200 ticks
/// and died of the layoffs.
///
/// **Pack 3's founding answers it.** Same soak, founding live:
/// businesses 5 (was 1), population 20 (was 4), minimum 20 (was 1).
/// Six closures, five foundings, every seeded death answered within
/// 1–5 ticks, and no founded firm dying inside the horizon.
/// Criterion 5 below pins that with slack; criteria 6 and 7 pin the
/// anti-churn bound and the full closure→vacancy→founding→hire
/// chain.
#[test]
fn null_town_soak_population_moves_both_directions() {
    use crate::sim::{self, Event};

    const LAST: u64 = 200;
    let mut world = town_world();
    let seed_population = world.agents.len();
    let mut departed_ids: Vec<AgentId> = Vec::new();
    let mut first_departed: Option<u64> = None;
    let mut first_answer_after_departure: Option<u64> = None;
    let mut dipped = false;
    let mut closed: Vec<u64> = Vec::new();
    let mut min_population = seed_population;
    // Per-good birth/death stream for the anti-churn bound. A closed
    // firm's id no longer resolves, so the good is remembered in a
    // side table seeded from the boot set and extended on every
    // Founded.
    let mut sector: HashMap<AgentId, Good> = world
        .businesses()
        .map(|(_, business)| (business.id, business.product))
        .collect();
    let mut births: HashMap<Good, Vec<u64>> = HashMap::new();
    let mut deaths: HashMap<Good, Vec<u64>> = HashMap::new();
    // Deaths OF FOUNDED FIRMS only — the found→close cycle the
    // anti-churn criterion is actually about. Counting every death of
    // a good conflates a seeded venue's death with an entrant's, and
    // an entrant dying is the failure the scarcity gate exists to
    // prevent.
    let mut founded_deaths: HashMap<Good, Vec<(u64, u64)>> = HashMap::new();
    let mut born_at: HashMap<AgentId, u64> = HashMap::new();
    // The full-cycle chain, as four deliberate observations.
    let mut freed: Vec<(u64, HouseId)> = Vec::new();
    let mut chain_reoccupied: Option<(u64, HouseId, u64)> = None; // tick, house, freed_at
    let mut chain_business: Option<AgentId> = None;
    let mut chain_hired: Option<u64> = None;
    let mut founded_ids: Vec<AgentId> = Vec::new();
    for t in 1..=LAST {
        let report = sim::tick_with_rate(&mut world, 0);
        for event in &report.events {
            match event {
                // no compiler help for either arm: this match ends in
                // `_ => {}`, so a missing one ships green
                Event::Founded {
                    business,
                    house,
                    good,
                    ..
                } => {
                    sector.insert(*business, *good);
                    births.entry(*good).or_default().push(t);
                    founded_ids.push(*business);
                    born_at.insert(*business, t);
                    // this arm shadows the answer arm below, so it
                    // records the answer itself
                    if first_departed.is_some_and(|d| t > d) {
                        first_answer_after_departure.get_or_insert(t);
                    }
                    if let Some((at, _)) = freed.iter().find(|(at, id)| *at < t && id == house)
                        && chain_reoccupied.is_none()
                    {
                        chain_reoccupied = Some((t, *house, *at));
                        chain_business = Some(*business);
                    }
                }
                // the hire must name the very firm founded into the
                // freed house — not merely any founded firm, which an
                // unrelated founding elsewhere would satisfy
                Event::Hired { business, .. }
                    if chain_business.is_some_and(|id| id == *business) =>
                {
                    chain_hired.get_or_insert(t);
                }
                Event::Departed { agent, .. } => {
                    departed_ids.push(*agent);
                    first_departed.get_or_insert(t);
                }
                // only an answer at a strictly later tick than the
                // first departure counts — the town ANSWERING the
                // shock, which a boot transient cannot satisfy
                // (phase order puts both before Departed inside one
                // tick, so strictly-later is the honest bar)
                Event::Arrived { .. } if first_departed.is_some_and(|d| t > d) => {
                    first_answer_after_departure.get_or_insert(t);
                }
                Event::Closed {
                    business, house, ..
                } => {
                    closed.push(t);
                    freed.push((t, *house));
                    if let Some(good) = sector.get(business) {
                        deaths.entry(*good).or_default().push(t);
                        if let Some(born) = born_at.get(business) {
                            founded_deaths.entry(*good).or_default().push((*born, t));
                        }
                    }
                }
                _ => {}
            }
        }
        let after = world.agents.len();
        dipped |= after < seed_population;
        min_population = min_population.min(after);
    }
    // The town ANSWERS the shock. Through pack 2 the only available
    // answer was immigration; since pack 3 founding is the other, and
    // measurably the one that fires — so the criterion is stated over
    // both, which is the firm-lifecycle spec's own full-cycle wording
    // ("SOME house is `Founded`- or `Arrived`-into after the
    // closure"), not a weakened version of the pack-4 assertion.
    //
    // Measured 2026-08-30, and worth stating because it reads like a
    // regression and is not one: with founding live there are ZERO
    // arrivals in 300 ticks, while 2–3 houses stand vacant the whole
    // time. Premises are not the blocker — an aged clean slot is.
    // Founding creates jobs, and the town's own unemployed take them
    // within a tick or two, so no vacancy ever survives the
    // VACANCY_PULL_TICKS wait. That is the pull rule working as
    // designed: importing a stranger is for demand the residents
    // cannot meet. The pack-4 arrival criterion was measured on code
    // where nothing else could answer a shock.
    assert!(!departed_ids.is_empty(), "nobody left in {LAST} ticks");
    assert!(
        first_answer_after_departure.is_some(),
        "nothing answered the shock — no founding and no arrival \
         (first departure at {first_departed:?})"
    );
    assert!(dipped, "population never fell below the seed count");
    // 5. (firm-lifecycle pack 3) FOUNDING ANSWERS THE CASCADE.
    //    Measured 2026-08-30 over this exact run, against pack 2's
    //    same soak with founding absent:
    //
    //        pack 2   businesses 1, population 4, min 1
    //        pack 3   businesses 5, population 20, min 20
    //
    //    closures [140, 153, 156, 179, 182, 199]; births Food [142],
    //    Entertainment [155, 157], Luxury [180, 184]. Every seeded
    //    death is answered within 1–5 ticks, and NO founded firm
    //    closes inside the horizon — the anti-churn target holds with
    //    room. The bounds below sit BELOW those measurements on
    //    purpose: pack 2 shipped two zero-margin floors and its close
    //    review was right to call them traps, so these carry slack
    //    (5 → 4, 20 → 15, 5 → 3) and are stated as measured-vs-
    //    asserted rather than as "deliberately loose".
    assert!(
        world.businesses().count() >= 4,
        "the town kept only {} venues — founding is not answering",
        world.businesses().count()
    );
    assert!(
        min_population >= 15,
        "population troughed at {min_population} — the cascade is winning"
    );
    let total_births: usize = births.values().map(Vec::len).sum();
    assert!(
        total_births >= 3,
        "only {total_births} firms were founded in {LAST} ticks"
    );

    // 6. ANTI-CHURN: no good may run more than one found→close cycle
    //    per 100-tick window — a FOUNDED firm dying is the failure the
    //    scarcity gate's direction test exists to prevent, and a
    //    SEEDED venue's death is not that. Measured at this horizon:
    //    zero founded deaths in any sector, so the bound has real
    //    room rather than sitting on its own value.
    for good in Good::ALL {
        let cycles = founded_deaths.get(&good).cloned().unwrap_or_default();
        for window in 0..LAST {
            let inside = cycles
                .iter()
                .filter(|(_, death)| (window..window + 100).contains(death))
                .count();
            assert!(
                inside <= 1,
                "{good} churned: {inside} founded firms died in t{window}..t{} ({cycles:?})",
                window + 100
            );
        }
    }

    // 7. THE FULL-CYCLE CHAIN, as four deliberate observations: a
    //    venue closed, its house passed the vacancy predicate, a
    //    firm was founded into that very house at a strictly later
    //    tick, and someone was hired into it. Hand-written — this
    //    match ends in `_ => {}` and forces nothing.
    let (reoccupied_at, house, freed_at) =
        chain_reoccupied.expect("no freed house was ever founded into");
    // The house really was emptied by a closure BEFORE the founding —
    // the middle link a bare "some house was founded into" skips.
    assert!(
        freed_at < reoccupied_at,
        "the founding at t{reoccupied_at} did not follow a freeing (t{freed_at})"
    );
    let hired_at = chain_hired.expect("nobody was hired into THAT founded firm");
    assert!(
        hired_at >= reoccupied_at,
        "the hire (t{hired_at}) preceded the founding (t{reoccupied_at})"
    );

    // 8. (firm-lifecycle pack 2) the fuse fires in the real town,
    //    not only on fixtures. **Pack 2's numbers, kept as the
    //    before-picture:** closures at t140/t153/t156/t171/t172, min
    //    population 1, final population 4, one surviving venue AT
    //    THAT HORIZON ONLY — the cascade was TOTAL, since Greenrow
    //    Farm closed at t201, one tick past this window, after which
    //    the town held ZERO businesses through t300.
    //
    //    Pack 3's founding is what changed that, and criterion 5
    //    above carries the raised bounds. The two zero-margin floors
    //    pack 2 shipped here — `businesses().count() >= 1`, which
    //    cleared by exactly one tick, and `min_population >= 1`,
    //    which sat exactly on the measured minimum — are DELETED,
    //    replaced by bounds with real slack. `closed >= 3` survives
    //    below as the proof the fuse still fires at all.
    // Report before asserting, so a red run says what it measured.
    println!(
        "PACK3 SOAK: closures {closed:?} | births {births:?} | deaths {deaths:?} | \
         pop {} min {min_population} | businesses {} | chain {:?}/{:?}",
        world.agents.len(),
        world.businesses().count(),
        chain_reoccupied,
        chain_hired,
    );
    let _ = (&founded_ids, &house);
    assert!(
        closed.len() >= 3,
        "closure never reached the demand-losing venues (closed at {closed:?})"
    );

    // TRAJECTORY PIN — added by the 2026-09-08 consolidation review, which
    // found that this file was NOT frozen in the way its header claimed.
    //
    // The twins copy test BODIES; they do not copy the production constants
    // those bodies read. So pack 3's `FOUNDING_TEMPLATE` change (Food's
    // founding headcount 2 → 4) moved this trajectory **without touching
    // this file**, and the criteria above kept passing on their slack.
    // Measured, same test, same rate 0, only that constant differing:
    //
    //     headcount 2   closures [140, 153, 156, 179, 182, 199]
    //                   Food births [142]              5 businesses
    //     headcount 4   closures [140, 153, 156, 166, 190, 193, 195]
    //                   Food births [142, 169, 194]    6 businesses
    //
    // Pack 3's ledger said "the frozen null twins stay frozen … no twin was
    // touched". The FILE was untouched; the BASELINE was not, and passing is
    // not the same as unchanged. That conflation is the exact laundering this
    // module exists to prevent, so the trajectory is now pinned outright:
    // any change that moves it — the recycle or anything else — fails here
    // and must be re-pinned deliberately, with the cause named.
    assert_eq!(
        closed,
        vec![140, 153, 156, 166, 190, 193, 195],
        "the rate-0 closure trajectory moved; name what moved it before re-pinning"
    );
    assert_eq!(
        world.businesses().count(),
        6,
        "the rate-0 surviving-firm count moved; name what moved it before re-pinning"
    );

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
