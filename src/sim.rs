//! The fixed 9-phase tick (parent doc §6). A new mechanic lands INSIDE its
//! phase; adding or reordering phases requires amending the spec's phase
//! contract table. The conservation audit (§8.3) is unconditionally last.

use crate::agent::{Agent, AgentId};
use crate::business::RoleSlot;
use crate::goods::Good;
use crate::housing::HouseId;
use crate::market::{self, JobOffer, Offer, SellerSnapshot};
use crate::metal::Metal;
use crate::money::Money;
use crate::role::Role;
use crate::world::{ClosureReceipt, World};
use std::collections::HashMap;

/// What an agent wants to do, decided in a pure pass and executed in an
/// apply pass (see `goods_market` for the worked template). Mechanics add
/// variants; every `match intent` stays exhaustive so a new variant is a
/// compile-time forcing function on every apply fn.
pub enum Intent {
    /// Buy `units` of `good` from `business`'s stock (phase 4). Planned
    /// against the tick-start snapshot, so `units` may exceed stock by
    /// apply time — apply caps to what is really on the shelf.
    Buy {
        buyer: AgentId,
        business: AgentId,
        good: Good,
        units: u32,
    },
    /// Take `role` at `business` (phase 1). Planned against the
    /// tick-start snapshot; apply re-checks the live headcount so racing
    /// hires die cleanly, like stale Buys.
    TakeJob {
        agent: AgentId,
        business: AgentId,
        role: Role,
    },
    /// Walk out of the current job over unpaid wages (phase 1). Apply
    /// clears `workplace` and `employed_role` together; the `owed_to`
    /// entry persists (settlement only at emigration — Amendment 17,
    /// pack 4).
    Quit { agent: AgentId },
    /// A newcomer arrives to take a vacant residence (phase 1's pull
    /// rule, pack 4): decided when a slot has aged past the pull
    /// threshold, a vacant residence stands, and External can stake
    /// them. They join the applicant pool next tick.
    Arrive { name: String, home: HouseId },
    /// A destitute agent leaves town (phase 7's push rule, pack 4):
    /// hunger past the threshold and too poor to buy a single unit of
    /// Food. Apply is `World::remove_agent` — settle, sweep, remove.
    Depart { agent: AgentId },
    /// An unemployed resident stakes their own capital into a new venue
    /// (phase 6's founding rule, pack 3). Planned against the phase-start
    /// snapshot, at most ONE per tick (the one-arrival precedent: a
    /// legible wallet drain and a deterministic pass). `house` is the
    /// premises, never a home — the founder does not move in. Apply
    /// re-checks every fact and dies cleanly if any moved.
    Found {
        founder: AgentId,
        house: HouseId,
        good: Good,
        price: Money,
    },
}

/// One observable thing a phase did this tick, for the shell to narrate.
/// Data-only (town-colony spec, `Event`/`TickReport` contract): dropping a
/// report changes no state. Each pack adds its own variants; the shell
/// matches exhaustively, so a new variant forces the renderer at compile
/// time.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Phase 1 apply: a worker walked out over unpaid wages. `owed` is
    /// the arrears entry they leave behind — it persists on the ledger
    /// (settlement only at emigration, Amendment 17 / pack 4).
    Quit {
        agent: AgentId,
        business: AgentId,
        owed: Money,
    },
    /// Phase 1 apply: an unemployed agent took an open role, at the wage
    /// posted when they applied (the write-back may move it before
    /// payday).
    Hired {
        agent: AgentId,
        business: AgentId,
        role: Role,
        wage: Money,
    },
    /// Phase 1 write-back: the posted wage changed for the next tick's
    /// matching. A held wage is not an event.
    WageMoved {
        business: AgentId,
        role: Role,
        from: Money,
        to: Money,
    },
    /// Phase 2: a staffed business added `units` of its product to stock.
    Produced {
        business: AgentId,
        good: Good,
        units: u32,
    },
    /// Phase 3: `worker` received `amount` — this tick's wage and any
    /// past-due arrears share one pot, so this is what actually moved.
    WagePaid {
        business: AgentId,
        worker: AgentId,
        amount: Money,
    },
    /// Phase 3: after paying what the coffers covered, `remaining` stays
    /// on the `owed_to` ledger against this worker.
    PayrollShort {
        business: AgentId,
        worker: AgentId,
        remaining: Money,
    },
    /// Phase 4 apply: a purchase landed, at the snapshot price.
    Sold {
        business: AgentId,
        buyer: AgentId,
        good: Good,
        units: u32,
        price: Money,
    },
    /// Phase 4 write-back: the posted price changed for next tick. A held
    /// price is not an event.
    PriceMoved {
        business: AgentId,
        good: Good,
        from: Money,
        to: Money,
    },
    /// Phase 5: the agent's Food could not cover one tick's consumption;
    /// the stored `Agent.hunger` counter moves with it (pack 4).
    WentHungry { agent: AgentId },
    /// Phase 6 (firm-lifecycle pack 1): a business paid its gold coffer
    /// surplus above the retained buffer to its owner. A zero draw is
    /// not an event (the held-price precedent).
    ProfitDrawn {
        business: AgentId,
        owner: AgentId,
        amount: Money,
    },
    /// Phase 1 apply (pack 4): an immigrant took a vacant residence.
    /// They apply for work from the next tick's snapshot.
    Arrived {
        agent: AgentId,
        name: String,
        home: HouseId,
    },
    /// A business settled arrears outside payroll — what its coffer
    /// covered, whole or partial. Two sources since pack 2: phase 7's
    /// Amendment-17 settlement immediately before a leaver's sweep, and
    /// `close_business`'s creditor pass, which reaches FORMER workers
    /// too (they are settled but not laid off). The written-off
    /// remainder is silent bookkeeping — the preceding `PayrollShort`s
    /// already told that story.
    Settled {
        business: AgentId,
        agent: AgentId,
        amount: Money,
    },
    /// Phase 6, or phase 7's forced liquidation (firm-lifecycle pack 2):
    /// a business was liquidated. Carries `house` because the business
    /// id resolves to nothing afterward, and `owner_name` because in the
    /// forced path the owner id does too (the `Departed` name-carrying
    /// precedent, applied to both dead-id classes). `proceeds` lists
    /// every `Metal::ALL` entry in that order, zeros included (D3
    /// visible-zeros). The money story is told by the accompanying
    /// `Settled`s and by `proceeds`; sourced from the `ClosureReceipt`,
    /// never re-derived.
    Closed {
        business: AgentId,
        house: HouseId,
        owner: AgentId,
        owner_name: String,
        proceeds: Vec<(Metal, Money)>,
    },
    /// Phase 6, or phase 7's forced liquidation (firm-lifecycle pack 2):
    /// a worker's employer was liquidated under them. Distinct from
    /// `Quit` — pushed, not walked: no volition, and no arrears
    /// implication (their debt, if any, was settled or written off in
    /// the same command). They re-enter the applicant pool next tick.
    LaidOff { agent: AgentId, business: AgentId },
    /// Phase 6 (firm-lifecycle pack 3): an unemployed resident founded a
    /// firm and self-hired into it. `capital` is READ BACK from the new
    /// firm's balance after the stake, never assumed — so the event
    /// cannot lie about a stake that failed (the balance-delta
    /// precedent). One per tick at most.
    Founded {
        business: AgentId,
        founder: AgentId,
        house: HouseId,
        good: Good,
        price: Money,
        capital: Money,
    },
    /// Phase 7 apply (pack 4): an agent emigrated, every balance swept
    /// to External (settlement included). Carries BOTH the id (tests
    /// and soaks harvest it — names are not enforced unique) and the
    /// name (the id resolves to nothing once they are gone). `took`
    /// lists every `Metal::ALL` entry in that order, zeros included
    /// (D3 visible-zeros precedent).
    Departed {
        agent: AgentId,
        name: String,
        took: Vec<(Metal, Money)>,
    },
}

/// Everything `tick` observed happen, in phase order and then each phase's
/// pinned iteration order (businesses in houses order, agents in
/// `world.agents` order). Pure observation (Amendment 15): dropping it
/// changes no state, and the audit emits nothing.
#[derive(Debug, Default)]
pub struct TickReport {
    pub events: Vec<Event>,
}

/// Runs one tick: phases 1–8 in exactly the spec table's order — labor
/// clears, produce, wages, goods clear, consume, invest, sinks, mint — then
/// the conservation audit, unconditionally last; no early return skips it.
/// Live phases append [`Event`]s to the returned report (Amendment 15);
/// stub phases gain the report parameter when they gain behavior.
///
/// # Panics
///
/// Panics if the closing [`audit`](crate::money::Accounts::audit) finds the
/// books imbalanced (§8.3) — meaning some phase moved money outside the
/// §8.2 chokepoint.
pub fn tick(world: &mut World) -> TickReport {
    let mut report = TickReport::default();
    labor_market(world, &mut report);
    produce(world, &mut report);
    pay_wages(world, &mut report);
    goods_market(world, &mut report);
    consume(world, &mut report);
    invest(world, &mut report);
    sinks(world, &mut report);
    mint_phase(world);
    // Phase 9: audit (§8.3) — read-only, never gains behavior, emits nothing.
    world.accounts.audit();
    report
}

/// How many ticks' worth of unpaid wages a worker tolerates: quit when
/// `owed_to[worker] > QUIT_ARREARS_BILLS × their slot's wage` (strictly
/// greater — exactly N bills is endured). Tuning constant beside its fn,
/// like the market constants; soak-tuned with worldgen's, then frozen.
const QUIT_ARREARS_BILLS: u32 = 3;

/// How many full-staffing wage bills a business retains before paying
/// profit to its owner (phase 6, firm-lifecycle pack 1). An independent
/// constant, deliberately NOT "worldgen's proven drought depth": the
/// seeded 3 bills price full headcount against partial boot staffing
/// (~4.5–6 real payroll ticks of runway), while this buffer holds
/// exactly `DRAW_BUFFER_BILLS` zero-revenue ticks at full staffing —
/// the pack-1 re-measure sizes it against the longest observed revenue
/// drought, then freezes it. FROZEN at 3 (pack-1 ledger): the healthy
/// 100-tick window shows max sold-drought 2 and a single one-tick
/// arrears flicker (11g, Longacre) — three bills carry every measured
/// drought with margin, and all four soaks hold. `pub(crate)` so the
/// worldgen soak asserts the coffer bound against the same constant.
pub(crate) const DRAW_BUFFER_BILLS: u32 = 3;

/// A business is *insolvent this tick* when it ends the tick owing
/// anyone anything. Pure and scalar-taking like [`draw_amount`], for two
/// reasons: it gives the boundary a unit-test home outside the sim, and
/// it gives the write-back one definition instead of an inline
/// comparison. No cross-module consumer today: the 100-tick soak's
/// criterion 7 asserts on the COUNTER this predicate writes and on
/// `CLOSE_INSOLVENT_TICKS`, never on the predicate itself.
///
/// Deliberately reads the `owed_total()` FIELD, never `PayrollShort`
/// events: a venue with no live staff accrues nothing and emits neither
/// `WagePaid` nor `PayrollShort` ever again, so an event-keyed trigger
/// would be permanently blind to exactly the zombie closure must kill
/// (measured on a forced fixture, pack-2 probe).
pub(crate) fn insolvent_now(owed_total: Money) -> bool {
    owed_total > Money::ZERO
}

/// Consecutive insolvent ticks a firm carries before phase 6 liquidates
/// it. An arrears-**persistence** trigger, deliberately not an arrears
/// level: the quit rule caps per-worker debt near four wages and
/// post-quit arrears freeze, so persistence is the one signal that stays
/// reachable. FROZEN at 12, 2026-08-30, by the procedure the spec
/// prescribes ("the observed per-venue maximum … and the threshold frozen
/// above it"): observed maximum 1, over ten single-tick flickers in the
/// 100-tick tuned soak.
///
/// Measured, 200-tick `town_world` under the pack-1 draw:
/// - **healthy t≤100** — max streak 1 (Longacre's isolated one-tick
///   2–11g flickers against a 140g bill); three venues never accrue a
///   coin in 200 ticks. 12 clears the observed maximum by 11.
/// - **doomed** — terminal streaks 73 / 60 / 57, i.e. 6.1× / 5.0× / 4.75×
///   this fuse; every doomed venue dies with room.
/// - **ordering** — the write-back is last and closure reads the
///   phase-start snapshot, so a firm crossing at tick t closes at t+1:
///   closure tick = 128+k / 141+k / 144+k against first quits t134 /
///   t146 / t148. A strict one-tick lead over the quit needs k ≥ 7
///   (Longacre binding). At 12 the derivation predicted closures at
///   t140 / t153 / t156, and the 200-tick soak under live closure hit
///   those three ticks EXACTLY — slack +6 / +7 / +8 over the quits.
///   (The spec's provisional 6 closes Longacre on its own quit tick — a
///   within-tick tie, phase 1 before phase 6, so not an ordering failure
///   but zero slack.)
/// - **cascade, recorded** — closure is not free, and the cascade is
///   TOTAL: every venue dies, the last at t201 (one tick past the
///   200-tick soak's horizon), after which the town holds no businesses
///   at all. Measured against a pack-1 baseline, only three of the six
///   deaths are the cascade's own: Longacre / Brass Bell / Gilt Curtain
///   already carry 73 / 60 / 57-tick terminal arrears streaks with
///   closure absent and simply never die, whereas Karat & Co,
///   Silverthread and Greenrow carry ZERO arrears for 200 ticks there
///   and die only under closure's layoffs. That is the cost of landing
///   "firms die" before "firms are born", not a mistuning — no
///   threshold inside the range that separates the two arrears modes
///   avoids it, and one above the measured 73-tick streaks would make
///   closure soak-invisible instead. Pack 3's founding is the cure.
///
/// Mnemonic: 12 = 3 × (`QUIT_ARREARS_BILLS` + 1), three nominal quit
/// cycles — but the nominal 4-tick cycle assumes a full-bill shortfall
/// and the measured horizon is 6 / 5 / 4, so the measurement, not the
/// mnemonic, is the justification.
///
/// STANDING OBLIGATION: the healthy flicker moved 0 → 1 when pack 1's
/// draw thinned coffers. Any future pack that touches coffers (a larger
/// `DRAW_BUFFER_BILLS`, demurrage, imports) must re-measure the healthy
/// max streak and re-freeze this; the 100-tick soak's zero-closure
/// criterion is the tripwire. `pub(crate)` so that soak and the shell
/// name this same constant (the `DRAW_BUFFER_BILLS` precedent).
pub(crate) const CLOSE_INSOLVENT_TICKS: u32 = 12;

/// Wage bills of capital a founder stakes into a new firm — the
/// `WAGE_BILLS_SEEDED` precedent, so a founded firm is funded exactly
/// like a worldgen one. Structurally forced rather than tuned: at
/// `capital == wage_bill × DRAW_BUFFER_BILLS` with no arrears the new
/// firm's coffer sits EXACTLY at `draw_amount`'s buffer, so it draws
/// zero on its founding tick and the stake cannot round-trip to the
/// founder inside the same phase 6. Changing either constant without
/// the other breaks that; a test pins them equal.
const FOUND_CAPITAL_BILLS: u32 = 3;

/// Gold a founder keeps back for themselves — nobody founds themselves
/// destitute. PROVISIONAL: the pack-3 planning probe found it
/// non-binding at every tick the gate fired (the marginal founders held
/// thousands), and no value above zero is satisfiable in the measured
/// capital drought, so the pack's re-measure freezes it only if it ever
/// actually binds.
const FOUNDER_RESERVE: Money = Money::new(200);

/// Consecutive hungry ticks before a destitute agent gives up on the
/// town (phase 7's push rule, pack 4). Soak-tuned, then frozen.
const DEPART_HUNGER_TICKS: u8 = 5;
/// How many consecutive post-matching ticks a slot must sit open before
/// phase 1 pulls an immigrant (pack 4). Soak-tuned, then frozen.
const VACANCY_PULL_TICKS: u32 = 3;
/// The gold External stakes each arrival (Amendment 16) — refused whole
/// if External cannot cover it at apply (§8.5), leaving a
/// penniless-but-valid newcomer. Soak-tuned, then frozen.
const GRUBSTAKE: Money = Money::new(100);
/// Fixed immigrant name table — with `World.arrivals` as the counter,
/// naming is deterministic, no RNG (town-colony spec). Distinct from
/// worldgen's resident names so the shell's name-inspect stays exact.
const IMMIGRANT_NAMES: [&str; 12] = [
    "Mara", "Ivo", "Corin", "Alba", "Sten", "Noor", "Talia", "Bruno", "Edda", "Falk", "Greta",
    "Hollis",
];

/// Deterministic immigrant naming: table + counter, wrapping with a
/// generation suffix ("Mara 2") if the town ever churns past the list.
fn immigrant_name(arrivals: u32) -> String {
    let index = arrivals as usize % IMMIGRANT_NAMES.len();
    let generation = arrivals as usize / IMMIGRANT_NAMES.len();
    if generation == 0 {
        IMMIGRANT_NAMES[index].to_string()
    } else {
        format!("{} {}", IMMIGRANT_NAMES[index], generation + 1)
    }
}

/// Phase 1: match hires, adjust wage offers. Money ops allowed: none —
/// hiring and quitting move no coin (the immigration grubstake is
/// Amendment 16, pack 4). The goods template applied to labor (pack 3):
/// snapshot → pure decide (quits, then applications) → apply with live
/// re-checks → wage write-back, which steers the NEXT tick's matching
/// (this tick's decide only ever saw the snapshot; this tick's payroll
/// reads the live wage the way produce reads live staffing).
fn labor_market(world: &mut World, report: &mut TickReport) {
    // Snapshot (pure): every open role, businesses in houses order ×
    // `Role::ALL` order — never HashMap iteration (the no-RNG guarantee
    // is only as good as pinned iteration) — and each worker's creditor
    // employers (order-free membership set, so the ledger's HashMap
    // iteration is harmless here).
    let snapshot: &World = world;
    let offers: Vec<JobOffer> = snapshot
        .businesses()
        .flat_map(|(house, business)| {
            Role::ALL.iter().filter_map(move |&role| {
                let slot = business.roles.get(&role)?;
                let staffed = staff_in_role(snapshot, house.id, role);
                Some(JobOffer {
                    business: business.id,
                    role,
                    wage: slot.wage,
                    open_slots: slot.headcount.saturating_sub(staffed),
                })
            })
        })
        .collect();
    let mut owed_by: HashMap<AgentId, Vec<AgentId>> = HashMap::new();
    for (_, business) in snapshot.businesses() {
        for (&worker, &amount) in &business.owed_to {
            if amount > Money::ZERO {
                owed_by.entry(worker).or_default().push(business.id);
            }
        }
    }

    // Decide (pure), quits first: employed agents in `world.agents`
    // order walk out when arrears pass the threshold. A same-tick
    // quitter does not also apply — they were employed at snapshot and
    // re-enter the pool next tick.
    let mut intents: Vec<Intent> = Vec::new();
    for agent in &snapshot.agents {
        let Some(workplace) = agent.workplace else {
            continue;
        };
        let Some(role) = agent.employed_role else {
            continue;
        };
        let Some(business) = snapshot.house(workplace).and_then(|h| h.business.as_ref()) else {
            continue;
        };
        let Some(slot) = business.roles.get(&role) else {
            continue; // unslotted role accrues nothing, so never quits
        };
        let owed = business
            .owed_to
            .get(&agent.id)
            .copied()
            .unwrap_or(Money::ZERO);
        if owed > slot.wage.times(QUIT_ARREARS_BILLS) {
            intents.push(Intent::Quit { agent: agent.id });
        }
    }
    // Then applications: unemployed agents in `world.agents` order —
    // ascending AgentId by construction, the contended-pass tie-break.
    let empty: Vec<AgentId> = Vec::new();
    for agent in &snapshot.agents {
        if agent.workplace.is_some() {
            continue;
        }
        let creditors = owed_by.get(&agent.id).unwrap_or(&empty);
        if let Some(application) = market::plan_application(&offers, creditors) {
            intents.push(Intent::TakeJob {
                agent: agent.id,
                business: application.business,
                role: application.role,
            });
        }
    }

    // The pull rule (pack 4, Amendment 16): a slot aged past the pull
    // threshold, a vacant residence standing, External able to stake —
    // at most ONE arrival per tick, so the External drain stays legible
    // and the pass deterministic. The newcomer is named from the fixed
    // table by the World counter.
    // The arrears conjunct (pack 2) sits in the OUTER closure, so a
    // deadbeat's aged slot never justifies an arrival at all: the town
    // stops importing strangers for employers who don't pay. A RULE, not
    // a race — `unfilled_ticks` ages independently of arrears (boot
    // openings age from tick 1), so no closure timing could protect an
    // immigrant from a venue whose slot aged before its insolvency began.
    // Recorded residual: once arrived, a newcomer owes nothing, so next
    // tick's `plan_application` can still match them INTO an
    // arrears-carrying venue — the same exposure every resident
    // non-creditor has. This fixes the importing, not the matching.
    let slot_aged = snapshot.businesses().any(|(_, business)| {
        business.owed_total() == Money::ZERO
            && Role::ALL.iter().any(|role| {
                business
                    .roles
                    .get(role)
                    .is_some_and(|slot| slot.unfilled_ticks >= VACANCY_PULL_TICKS)
            })
    });
    if slot_aged
        && snapshot
            .accounts
            .balance_of(snapshot.external_id, Metal::Gold)
            >= GRUBSTAKE
    {
        // houses are add-ordered, so `find` is the lowest vacant HouseId
        let vacant = snapshot
            .houses
            .iter()
            .find(|house| snapshot.is_fully_vacant(house.id));
        if let Some(house) = vacant {
            intents.push(Intent::Arrive {
                name: immigrant_name(snapshot.arrivals),
                home: house.id,
            });
        }
    }

    // Apply: re-check live state, mirroring the goods apply — stale
    // intents die cleanly, nothing partially applied. Applications are
    // tallied per (business, role) so the write-back can see the stale
    // queue (applied − landed).
    let mut applied: HashMap<(AgentId, Role), u32> = HashMap::new();
    let mut landed: HashMap<(AgentId, Role), u32> = HashMap::new();
    for intent in intents {
        apply_labor_intent(world, intent, &mut applied, &mut landed, report);
    }

    // Wage write-back (logic in market.rs, §8.6): unfilled-and-affordable
    // raises one step, a stale queue lowers one step. New wages steer the
    // next tick's matching.
    let slots: Vec<(HouseId, AgentId, Role, Money, u32)> = world
        .businesses()
        .flat_map(|(house, business)| {
            Role::ALL.iter().filter_map(move |&role| {
                let slot = business.roles.get(&role)?;
                Some((house.id, business.id, role, slot.wage, slot.headcount))
            })
        })
        .collect();
    for (house_id, business_id, role, wage, headcount) in slots {
        let open_slots = headcount.saturating_sub(staff_in_role(world, house_id, role));
        let total_applied = applied.get(&(business_id, role)).copied().unwrap_or(0);
        let total_landed = landed.get(&(business_id, role)).copied().unwrap_or(0);
        let queue = total_applied - total_landed;
        // Affordable = the coffer covers one tick's full-staffing bill
        // with THIS role's wage stepped (the wage_bill precedent) — the
        // gate that keeps an insolvent business from posting raises.
        // Sibling roles are priced at their LIVE wage, deliberately: an
        // earlier same-tick raise (Role::ALL order) tightens the later
        // roles' gates, never loosens them. Single-role in every
        // shipped world; pinned here for the first two-role business.
        let stepped = market::stepped_wage(wage);
        let business = world
            .house(house_id)
            .expect("collected from businesses()")
            .business
            .as_ref()
            .expect("collected from businesses()");
        let bill = Role::ALL
            .iter()
            .filter_map(|&r| {
                business.roles.get(&r).map(|slot| {
                    let per = if r == role { stepped } else { slot.wage };
                    per.times(slot.headcount)
                })
            })
            .fold(Money::ZERO, |sum, part| sum.plus(part));
        // Net of arrears: a business still owing back wages is not
        // solvent enough to post a raise, whatever sits in the coffer
        // this instant (measured: the per-tick-coffer test passes for a
        // venue with four figures of wage debt, and its raises feed the
        // very churn the gate exists to stop).
        let owed = world
            .house(house_id)
            .expect("collected from businesses()")
            .business
            .as_ref()
            .expect("collected from businesses()")
            .owed_total();
        let affordable = world.accounts.balance_of(business_id, Metal::Gold) >= bill.plus(owed);
        let adjusted = market::adjust_wage(wage, open_slots, queue, affordable);
        if adjusted != wage {
            report.events.push(Event::WageMoved {
                business: business_id,
                role,
                from: wage,
                to: adjusted,
            });
        }
        let slot = world
            .house_mut(house_id)
            .expect("collected from businesses()")
            .business
            .as_mut()
            .expect("collected from businesses()")
            .roles
            .get_mut(&role)
            .expect("collected from this business's roles");
        slot.wage = adjusted;
        // The vacancy-pull age (pack 4), measured post-matching — this
        // write-back is its single writer.
        slot.unfilled_ticks = if open_slots > 0 {
            slot.unfilled_ticks.saturating_add(1)
        } else {
            0
        };
    }
}

/// Live staff filling `role` at `house` — the per-role headcount check
/// (open slots are counted per role; for v1's single-role businesses
/// this equals the spec's `employees_of().len()` formula).
fn staff_in_role(world: &World, house: HouseId, role: Role) -> u32 {
    world
        .employees_of(house)
        .into_iter()
        .filter(|&worker| {
            world
                .agent(worker)
                .is_some_and(|a| a.employed_role == Some(role))
        })
        .count() as u32
}

fn apply_labor_intent(
    world: &mut World,
    intent: Intent,
    applied: &mut HashMap<(AgentId, Role), u32>,
    landed: &mut HashMap<(AgentId, Role), u32>,
    report: &mut TickReport,
) {
    match intent {
        Intent::Quit { agent } => {
            // Resolve the live workplace for the event — and so a quit
            // whose facts vanished dies cleanly.
            let Some(business) = world
                .agent(agent)
                .and_then(|person| person.workplace)
                .and_then(|workplace| world.house(workplace))
                .and_then(|house| house.business.as_ref())
            else {
                return;
            };
            let business_id = business.id;
            let owed = business.owed_to.get(&agent).copied().unwrap_or(Money::ZERO);
            if world.vacate_workplace(agent).is_err() {
                return;
            }
            report.events.push(Event::Quit {
                agent,
                business: business_id,
                owed,
            });
        }
        Intent::TakeJob {
            agent,
            business,
            role,
        } => {
            *applied.entry((business, role)).or_insert(0) += 1;
            let Some((house_id, headcount, wage)) = world
                .businesses()
                .find(|(_, b)| b.id == business)
                .and_then(|(house, b)| {
                    b.roles
                        .get(&role)
                        .map(|slot| (house.id, slot.headcount, slot.wage))
                })
            else {
                return; // business or slot vanished — intents don't outlive facts
            };
            // Re-check live state: an earlier hire this phase may have
            // filled the slot, and the agent must still be unemployed.
            let still_unemployed = world
                .agent(agent)
                .is_some_and(|person| person.workplace.is_none());
            if !still_unemployed || staff_in_role(world, house_id, role) >= headcount {
                return;
            }
            if world.assign_workplace(agent, house_id, role).is_err() {
                return;
            }
            *landed.entry((business, role)).or_insert(0) += 1;
            report.events.push(Event::Hired {
                agent,
                business,
                role,
                wage,
            });
        }
        Intent::Arrive { name, home } => {
            // Live re-checks, mirroring stale Buys and TakeJobs: the
            // pull's justifying vacancy must still exist — a slot filled
            // by this tick's hires kills the arrival (the boot cascade
            // races the pull; measured, review-caught) — and the home
            // re-validates inside the command.
            // The re-check inherits the same arrears conjunct: a venue
            // that owes back wages cannot CONFIRM an arrival either, so
            // an intent decided on a clean venue's aged slot dies cleanly
            // if only deadbeat headcount remains by apply time.
            let still_hiring = world.businesses().any(|(house, business)| {
                business.owed_total() == Money::ZERO
                    && Role::ALL.iter().any(|&role| {
                        business.roles.get(&role).is_some_and(|slot| {
                            slot.headcount > staff_in_role(world, house.id, role)
                        })
                    })
            });
            if !still_hiring {
                return;
            }
            let Ok(newcomer) = world.immigrate(name.clone(), home) else {
                return;
            };
            // Amendment 16 — phase 1's ONE money op: the grubstake,
            // whole or not at all. A refusal (External drained) leaves a
            // penniless-but-valid newcomer (§8.5), pinned by test.
            let _ = world.pay(world.external_id, newcomer, Metal::Gold, GRUBSTAKE);
            report.events.push(Event::Arrived {
                agent: newcomer,
                name,
                home,
            });
        }
        Intent::Buy { .. } | Intent::Depart { .. } | Intent::Found { .. } => {
            unreachable!("the labor apply only receives phase-1 intents")
        }
    }
}

/// Phase 2: labor + inputs → goods. Money ops allowed: none. Output
/// scales with headcount — `production_rate` is per staffer (pack 2).
fn produce(world: &mut World, report: &mut TickReport) {
    // The staffed check borrows world immutably; collect first, then
    // mutate stock through house_mut.
    let staffed: Vec<(HouseId, u32)> = world
        .businesses()
        .map(|(house, _)| (house.id, world.employees_of(house.id).len() as u32))
        .filter(|(_, staff)| *staff > 0)
        .collect();
    for (house_id, staff) in staffed {
        let house = world
            .house_mut(house_id)
            .expect("collected from businesses()");
        let business = house
            .business
            .as_mut()
            .expect("collected from businesses()");
        let units = business.product.production_rate() * staff;
        business.stock += units;
        report.events.push(Event::Produced {
            business: business.id,
            good: business.product,
            units,
        });
    }
}

/// Phase 3: firms pay agreed wages from their own coffers. Money ops
/// allowed: transfer only. Each tick's wage first joins the business's
/// `owed_to` ledger, then the business pays whatever its balance covers
/// — coffers drain to exactly zero before any wage goes unpaid, and
/// past-due wages repay automatically when revenue returns (arrears and
/// the current wage share one pot).
fn pay_wages(world: &mut World, report: &mut TickReport) {
    // Decide from the snapshot: who accrues which role's wage — every
    // employee of every business, in businesses() then ascending-id
    // order (pack 2: multi-worker payrolls share one coffer, paid in
    // that order). A worker with no employed_role, or a role the
    // business doesn't slot, earns nothing this milestone.
    // Shared reborrow: the decide pass is read-only, and `&World` is
    // Copy so the closures can hold it.
    let snapshot: &World = world;
    let accruals: Vec<(HouseId, AgentId, AgentId, Money)> = snapshot
        .businesses()
        .flat_map(|(house, business)| {
            snapshot
                .employees_of(house.id)
                .into_iter()
                .filter_map(move |worker| {
                    let role = snapshot.agent(worker)?.employed_role?;
                    let slot = business.roles.get(&role)?;
                    Some((house.id, business.id, worker, slot.wage))
                })
        })
        .collect();
    for (house_id, business_id, worker, wage) in accruals {
        let business = world
            .house_mut(house_id)
            .expect("collected from businesses()")
            .business
            .as_mut()
            .expect("collected from businesses()");
        let owed = business
            .owed_to
            .get(&worker)
            .copied()
            .unwrap_or(Money::ZERO)
            .plus(wage);
        business.owed_to.insert(worker, owed);
        // Pay what the coffers cover. Amount ≤ balance by construction,
        // so the transfer cannot err — but if it ever does, skip cleanly
        // (§8.5): the ledger keeps the full debt, never settled without
        // its payment.
        let payable = world
            .accounts
            .balance_of(business_id, Metal::Gold)
            .min(owed);
        if payable == Money::ZERO
            || world
                .pay(business_id, worker, Metal::Gold, payable)
                .is_err()
        {
            // A zero-wage slot with no arrears owes nothing — that is
            // not a shortfall, so it makes no event.
            if owed > Money::ZERO {
                report.events.push(Event::PayrollShort {
                    business: business_id,
                    worker,
                    remaining: owed,
                });
            }
            continue;
        }
        report.events.push(Event::WagePaid {
            business: business_id,
            worker,
            amount: payable,
        });
        let business = world
            .house_mut(house_id)
            .expect("collected from businesses()")
            .business
            .as_mut()
            .expect("collected from businesses()");
        if owed == payable {
            business.owed_to.remove(&worker);
        } else {
            let remaining = owed.minus(payable);
            business.owed_to.insert(worker, remaining);
            report.events.push(Event::PayrollShort {
                business: business_id,
                worker,
                remaining,
            });
        }
    }
}

/// Phase 4: agents buy goods, prices adjust. Money ops allowed: transfer
/// only. This phase is the WORKED decide→apply TEMPLATE — every behavior
/// phase copies this two-pass shape.
fn goods_market(world: &mut World, report: &mut TickReport) {
    // Decide (pure): every agent plans against the same tick-start offer
    // snapshot. No `&mut` anywhere — unit-testable and free of
    // iteration-order effects. Collective staleness (two buyers wanting
    // the same last unit) is resolved at apply time. `houses` runs
    // parallel to `offers` so the write-back below can reach each
    // offer's business without a second lookup.
    let (houses, offers): (Vec<HouseId>, Vec<Offer>) = world
        .businesses()
        .map(|(house, business)| {
            (
                house.id,
                Offer {
                    business: business.id,
                    good: business.product,
                    price: business.price,
                    stock: business.stock,
                },
            )
        })
        .unzip();
    let intents: Vec<Intent> = world
        .agents
        .iter()
        .flat_map(|agent| {
            decide_goods(
                agent,
                world.accounts.balance_of(agent.id, Metal::Gold),
                &offers,
            )
        })
        .collect();

    // Apply: the ONLY place this phase moves money. Unaffordable intents
    // fail cleanly (transfer errs) — wanting is unconstrained, paying is
    // not. `sold` counts units actually transacted, per business.
    let mut sold: HashMap<AgentId, u32> = HashMap::new();
    for intent in intents {
        apply_goods_intent(world, intent, &mut sold, report);
    }

    // Price write-back (logic in market.rs, §8.6): each price adjusts
    // from this tick's sell-through against the snapshot it was offered
    // at. New prices take effect next tick — the decide pass above only
    // ever saw the snapshot. A held price emits nothing.
    for (house_id, offer) in houses.into_iter().zip(offers) {
        let units = sold.get(&offer.business).copied().unwrap_or(0);
        let adjusted = market::adjust_price(offer.price, offer.stock, units);
        if adjusted != offer.price {
            report.events.push(Event::PriceMoved {
                business: offer.business,
                good: offer.good,
                from: offer.price,
                to: adjusted,
            });
        }
        let business = world
            .house_mut(house_id)
            .expect("snapshotted from businesses()")
            .business
            .as_mut()
            .expect("snapshotted from businesses()");
        business.price = adjusted;
        // This loop is `sold_out_ticks`'s SINGLE WRITER (pack 3). It reads
        // the same `market::sold_out` the ratchet above just used, so the
        // streak can never disagree with the price move. An empty shelf
        // is "no signal" and holds the streak rather than breaking it —
        // a sold-out seller that produces nothing next tick has not
        // stopped being scarce.
        business.sold_out_ticks = if offer.stock == 0 {
            business.sold_out_ticks
        } else if market::sold_out(offer.stock, units) {
            business.sold_out_ticks.saturating_add(1)
        } else {
            0
        };
    }
}

/// Needs-driven purchasing. Stays pure; the shopping algorithm itself
/// lives in market.rs (§8.6) — this just binds it to one agent.
fn decide_goods(agent: &Agent, wallet: Money, offers: &[Offer]) -> Vec<Intent> {
    market::plan_purchases(wallet, &agent.inventory, offers)
        .into_iter()
        .map(|purchase| Intent::Buy {
            buyer: agent.id,
            business: purchase.business,
            good: purchase.good,
            units: purchase.units,
        })
        .collect()
}

fn apply_goods_intent(
    world: &mut World,
    intent: Intent,
    sold: &mut HashMap<AgentId, u32>,
    report: &mut TickReport,
) {
    match intent {
        Intent::Buy {
            buyer,
            business,
            good,
            units,
        } => {
            // Re-read live stock: an earlier buyer this phase may have
            // emptied the shelf. Cap, pay, then hand over the goods —
            // money and goods move together or not at all.
            let found = world
                .businesses()
                .find(|(_, b)| b.id == business)
                .map(|(house, b)| (house.id, b.price));
            let Some((house_id, price)) = found else {
                return; // business vanished — intents don't outlive facts
            };
            let live_stock = world
                .house(house_id)
                .expect("found above")
                .business
                .as_ref()
                .expect("found above")
                .stock;
            let units = units.min(live_stock);
            if units == 0 {
                return;
            }
            if world
                .pay(buyer, business, Metal::Gold, price.times(units))
                .is_err()
            {
                return; // §8.5: skip cleanly, stock untouched
            }
            let house = world.house_mut(house_id).expect("found above");
            house.business.as_mut().expect("found above").stock -= units;
            let agent = world
                .agent_mut(buyer)
                .expect("intents are decided from world.agents");
            *agent.inventory.entry(good).or_insert(0) += units;
            *sold.entry(business).or_insert(0) += units;
            report.events.push(Event::Sold {
                business,
                buyer,
                good,
                units,
                price,
            });
        }
        Intent::TakeJob { .. }
        | Intent::Quit { .. }
        | Intent::Arrive { .. }
        | Intent::Depart { .. }
        | Intent::Found { .. } => {
            unreachable!("the goods apply only receives phase-4 intents")
        }
    }
}

/// Phase 5: goods consumed toward needs. Money ops allowed: none.
/// Shortfall just bottoms out at zero this milestone — no starvation
/// consequences yet (07-19 spec: out of scope). Going short of Food is
/// narrated (event-only; the stored hunger counter is pack 4's).
fn consume(world: &mut World, report: &mut TickReport) {
    for agent in &mut world.agents {
        let food = agent.inventory.get(&Good::Food).copied().unwrap_or(0);
        if food < Good::Food.consumption_rate() {
            report.events.push(Event::WentHungry { agent: agent.id });
            // The stored counter behind the event (pack 4): this is
            // hunger's SINGLE writer — nothing else touches it.
            agent.hunger = agent.hunger.saturating_add(1);
        } else {
            agent.hunger = 0;
        }
        for good in Good::ALL {
            let held = agent.inventory.entry(good).or_insert(0);
            *held = held.saturating_sub(good.consumption_rate());
        }
    }
}

/// The phase-6 profit draw (firm-lifecycle spec): gold coffer minus the
/// retained buffer — `DRAW_BUFFER_BILLS` full-staffing wage bills PLUS
/// the outstanding arrears — clamped at zero. Net of arrears by
/// contract (the pack-3 affordability erratum applied as formula):
/// arrears outrank the owner, so every coin a creditor is owed sits
/// behind the buffer and never reaches the owner while it is owed.
/// Netting, not gating — a venue carrying arrears still draws once its
/// coffer clears bills + owed (reachable: phase-4 revenue lands after
/// phase 3 carried the debt), which is what keeps a solvent business
/// with one frozen ex-worker entry from silently becoming a sink.
fn draw_amount(coffer: Money, wage_bill: Money, owed_total: Money) -> Money {
    let buffer = wage_bill.times(DRAW_BUFFER_BILLS).plus(owed_total);
    if coffer > buffer {
        coffer.minus(buffer)
    } else {
        Money::ZERO
    }
}

/// Phase 6: take profit (firm-lifecycle pack 1; closure and founding
/// land here in packs 2–3). Money ops allowed: transfer only — the draw
/// is a business→owner `World::pay`, inside the row's existing
/// allowance (Amendment 18 touched only the purpose text). A DIRECT
/// pass, no intents (the pay_wages precedent: objective per-business
/// state, zero contention), businesses in houses order, gold only —
/// the sole trading metal; closure's pack-2 `Metal::ALL` sweep is the
/// completeness backstop. This is the recorded cure for the
/// coffers-as-one-way-sinks fuse, expected partial: `target_days`
/// purchase caps mean owner income mostly pools (the recorded
/// expand-capacity seam), so the pack-1 re-measure pins what actually
/// moves rather than assuming.
/// Each good's market as the founding decide sees it. Built `Good::ALL`
/// outer × `businesses()` inner, so the order is pinned twice over and
/// `market::plan_founding`'s builder invariant holds by construction:
/// zero sellers yields `None` and a zero streak. The streak is the MAX
/// over the good's live sellers — one seller clearing its shelf is
/// enough to call the sector scarce.
fn seller_snapshots(world: &World) -> Vec<SellerSnapshot> {
    Good::ALL
        .iter()
        .map(|&good| {
            let mut sellers = 0;
            let mut cheapest_price: Option<Money> = None;
            let mut sold_out_streak = 0;
            for (_, business) in world.businesses().filter(|(_, b)| b.product == good) {
                sellers += 1;
                cheapest_price = Some(match cheapest_price {
                    Some(current) => current.min(business.price),
                    None => business.price,
                });
                sold_out_streak = sold_out_streak.max(business.sold_out_ticks);
            }
            SellerSnapshot {
                good,
                sellers,
                cheapest_price,
                sold_out_streak,
            }
        })
        .collect()
}

/// Phase 6's founding decide (pure, pack 3): the market names a good,
/// the roster names a founder, the houses name premises — or nobody
/// founds. At most ONE per tick.
///
/// Founder rule: the first UNEMPLOYED agent in `world.agents` order
/// (ascending id) whose gold covers the stake plus their own reserve.
/// Unemployed-only is the gate's signed ruling — founding turns a
/// dis-saver into an earner, the one channel in this milestone that
/// relieves the fuse. Recorded cost, measured by the pack-3 probe: it
/// also makes the wallet holding 99% of the town's gold — an EMPLOYED
/// owner drawing a monopolist's rent — ineligible to found.
fn decide_founding(world: &World) -> Option<Intent> {
    let snapshot: &World = world;
    let prospectus = market::plan_founding(&seller_snapshots(snapshot))?;
    let template = market::found_template(prospectus.good);
    let capital = template
        .wage
        .times(template.headcount * FOUND_CAPITAL_BILLS);
    let bar = capital.plus(FOUNDER_RESERVE);
    let founder = snapshot.agents.iter().find(|agent| {
        agent.workplace.is_none() && snapshot.accounts.balance_of(agent.id, Metal::Gold) >= bar
    })?;
    let house = snapshot
        .houses
        .iter()
        .find(|house| snapshot.is_fully_vacant(house.id))?;
    Some(Intent::Found {
        founder: founder.id,
        house: house.id,
        good: prospectus.good,
        price: prospectus.price,
    })
}

/// Phase 6's founding apply (pack 3). Kill-only live re-checks mirroring
/// stale Buys — every one collapsed to an owned scalar before any `&mut`
/// — then the three commands in a forced order: `found_business` (which
/// makes the id a known account), the capital stake, the self-hire.
fn apply_found_intent(world: &mut World, intent: Intent, report: &mut TickReport) {
    let Intent::Found {
        founder,
        house,
        good,
        price,
    } = intent
    else {
        unreachable!("the founding apply only receives phase-6 intents")
    };
    let template = market::found_template(good);
    let capital = template
        .wage
        .times(template.headcount * FOUND_CAPITAL_BILLS);
    // Re-checks: the founder must still exist and still be unemployed,
    // still afford the stake and their reserve, the premises must still
    // be vacant, and the sector must still have room. Any of them moving
    // kills the intent cleanly, with nothing half-founded.
    let eligible = world.agent(founder).is_some_and(|person| {
        person.workplace.is_none()
            && world.accounts.balance_of(founder, Metal::Gold) >= capital.plus(FOUNDER_RESERVE)
    });
    let room = seller_snapshots(world)
        .iter()
        .any(|snapshot| snapshot.good == good && snapshot.sellers < 2);
    if !eligible || !world.is_fully_vacant(house) || !room {
        return;
    }
    let mut roles = HashMap::new();
    roles.insert(
        Role::Labourer,
        RoleSlot {
            wage: template.wage,
            headcount: template.headcount,
            unfilled_ticks: 0,
        },
    );
    let Ok(business) = world.found_business(founder, house, good, price, roles) else {
        return; // re-checked above — dies cleanly regardless
    };
    // The stake cannot fail after the re-check: `found_business` is
    // money-free and nothing runs between it and this pay. The branch is
    // still written, mirroring the grubstake's honesty about its own
    // failure mode — and the self-hire still runs, so a penniless firm is
    // STAFFED and dies the normal payroll-arrears death rather than
    // standing forever as a trigger-proof empty vacancy magnet.
    let _ = world.pay(founder, business, Metal::Gold, capital);
    let _ = world.assign_workplace(founder, house, Role::Labourer);
    report.events.push(Event::Founded {
        business,
        founder,
        house,
        good,
        price,
        // read back, never assumed: a fresh firm starts at zero, so its
        // balance IS whatever the stake actually moved
        capital: world.accounts.balance_of(business, Metal::Gold),
    });
}

fn invest(world: &mut World, report: &mut TickReport) {
    // (0) The founding DECIDE, before anything moves — the phase-start
    // snapshot. A firm closing THIS tick still counts as a seller here,
    // so a refound is deliberately a t+1 event: the latency is
    // conservative and can only delay a founding, never cause a
    // premature one.
    let founding = decide_founding(world);

    // (1) Closures, FIRST, from the phase-start snapshot, houses order.
    // Above the draws collect so "a closing firm never draws" is
    // structural rather than a guard that could rot — and above the
    // counter write-back so a closed firm's counter dies with it.
    let doomed: Vec<HouseId> = world
        .businesses()
        .filter(|(_, business)| business.insolvent_ticks >= CLOSE_INSOLVENT_TICKS)
        .map(|(house, _)| house.id)
        .collect();
    for house in doomed {
        let receipt = world
            .close_business(house)
            .expect("collected from businesses()");
        // The owner outlives a steady-state closure, so the name resolves.
        let owner_name = world
            .agent(receipt.owner)
            .map(|person| person.name.clone())
            .unwrap_or_else(|| "(unknown agent)".to_string());
        emit_closure(&receipt, &owner_name, report);
    }

    // (2) The founding APPLY — after the closures whose freed houses it
    // may take, before the draws, so a firm founded this tick sits
    // exactly at its buffer and draws zero (see FOUND_CAPITAL_BILLS).
    if let Some(intent) = founding {
        apply_found_intent(world, intent, report);
    }

    let draws: Vec<(AgentId, AgentId, Money)> = world
        .businesses()
        .map(|(_, business)| {
            (
                business.id,
                business.owner,
                draw_amount(
                    world.accounts.balance_of(business.id, Metal::Gold),
                    business.wage_bill(),
                    business.owed_total(),
                ),
            )
        })
        .collect();
    for (business, owner, draw) in draws {
        if draw == Money::ZERO {
            continue;
        }
        // The pack-1 dangling-owner skip is RETIRED here: forced
        // liquidation (item 4) closes an emigrating owner's firms inside
        // `remove_agent`, so a live business always names a live owner
        // and this `pay` cannot meet a ghost. The invariant is pinned by
        // test, not defended by a branch — a silent skip would leave
        // money pooling in a firm nothing can drain.
        world
            .pay(business, owner, Metal::Gold, draw)
            .expect("min-bounded by the live coffer, both ids validated");
        report.events.push(Event::ProfitDrawn {
            business,
            owner,
            amount: draw,
        });
    }

    // The insolvency fuse (pack 2), LAST inside the phase and collected
    // AFTER any closures, so it never reaches through a detached house
    // and a closed firm's counter dies with its `Business`. This is the
    // single writer of `Business.insolvent_ticks`; the closure pass
    // reads it from the NEXT tick's phase-start snapshot, which is the
    // one tick of designed latency documented on the field. Same
    // collect-then-`house_mut` shape as `produce` (no `businesses_mut`).
    let marks: Vec<(HouseId, bool)> = world
        .businesses()
        .map(|(house, business)| (house.id, insolvent_now(business.owed_total())))
        .collect();
    for (house_id, insolvent) in marks {
        let business = world
            .house_mut(house_id)
            .expect("collected from businesses()")
            .business
            .as_mut()
            .expect("collected from businesses()");
        business.insolvent_ticks = if insolvent {
            business.insolvent_ticks.saturating_add(1)
        } else {
            0
        };
    }
}

/// Narrates one liquidation FROM its receipt, in causal order:
/// settlements, then layoffs, then the closure itself. Amounts are never
/// re-derived — deltas around the whole command cannot attribute flows
/// that share a wallet, which is exactly what the owner-as-creditor case
/// does (see [`ClosureReceipt`]). Shared by phase 6's closure pass and
/// phase 7's forced liquidation, so both paths narrate identically.
///
/// `owner_name` is passed in rather than looked up, because the forced
/// path cannot look it up: `remove_agent` returns its receipts only after
/// the leaver — who IS the owner there — is gone. Phase 6 reads the live
/// agent; phase 7 supplies the name it cloned before the command.
fn emit_closure(receipt: &ClosureReceipt, owner_name: &str, report: &mut TickReport) {
    for &(agent, amount) in &receipt.settlements {
        report.events.push(Event::Settled {
            business: receipt.business,
            agent,
            amount,
        });
    }
    for &agent in &receipt.laid_off {
        report.events.push(Event::LaidOff {
            agent,
            business: receipt.business,
        });
    }
    report.events.push(Event::Closed {
        business: receipt.business,
        house: receipt.house,
        owner: receipt.owner,
        owner_name: owner_name.to_string(),
        proceeds: receipt.residual.clone(),
    });
}

/// Phase 7: degradation, imports, and — since pack 4 — emigration.
/// Money ops allowed: burn, transfer→External, and the Amendment-17
/// settlement transfer (business→leaver, immediately before their
/// sweep). The push rule: worn down by hunger AND destitute — too poor
/// to buy even one unit of Food at the cheapest posted price. TODO:
/// demurrage and external purchases still land here.
fn sinks(world: &mut World, report: &mut TickReport) {
    // Decide (pure): the phase-start snapshot names the leavers, agents
    // in `world.agents` order.
    let snapshot: &World = world;
    // The reference price is each seller's LIVE posted price at phase-7
    // start — one write-back step ahead of the price this tick's failed
    // purchase saw, stock ignored (a sold-out seller's price still
    // counts). Deterministic either way; recorded in the manifest. No
    // Food market means no destitution test (no price to be below).
    // Since pack 2 that is REACHABLE in the shipped town rather than
    // hypothetical: the closure cascade eventually kills the last Food
    // seller (measured t201, one tick past the 200-tick soak), after
    // which nobody can be judged destitute at all. The guard scopes the
    // DECIDE, not the phase, so future phase-7 mechanics (demurrage,
    // imports) appended below stay unconditionally reached.
    let cheapest_food = snapshot
        .businesses()
        .filter(|(_, business)| business.product == Good::Food)
        .map(|(_, business)| business.price)
        .min();
    let intents: Vec<Intent> = match cheapest_food {
        Some(cheapest) => snapshot
            .agents
            .iter()
            .filter(|agent| {
                agent.hunger >= DEPART_HUNGER_TICKS
                    && snapshot.accounts.balance_of(agent.id, Metal::Gold) < cheapest
            })
            .map(|agent| Intent::Depart { agent: agent.id })
            .collect(),
        None => Vec::new(),
    };
    for intent in intents {
        apply_sinks_intent(world, intent, report);
    }
}

fn apply_sinks_intent(world: &mut World, intent: Intent, report: &mut TickReport) {
    match intent {
        Intent::Depart { agent } => {
            let Some(person) = world.agent(agent) else {
                return; // intents don't outlive facts
            };
            // Narration needs the name and the amounts BEFORE removal —
            // the id resolves to nothing afterward. Settlement and sweep
            // amounts are measured as balance deltas, so the events
            // report exactly what the command moved, never a
            // re-derivation that could drift.
            let name = person.name.clone();
            // The around-the-command delta recipe is valid ONLY where an
            // account has a single flow inside the command. Since
            // Amendment 19, a firm the leaver OWNS is liquidated inside
            // `remove_agent`: its coffer is drained by settlements to
            // every creditor plus the residual sweep, so `before − after`
            // would be the whole coffer and would surface as one bogus
            // A17 `Settled` to the leaver. Those firms narrate from their
            // receipts instead, so they are excluded here.
            let creditors: Vec<(AgentId, Money)> = world
                .businesses()
                .filter(|(_, business)| {
                    business.owner != agent
                        && business.owed_to.get(&agent).copied().unwrap_or(Money::ZERO)
                            > Money::ZERO
                })
                .map(|(_, business)| {
                    (
                        business.id,
                        world.accounts.balance_of(business.id, Metal::Gold),
                    )
                })
                .collect();
            let external_before: Vec<(Metal, Money)> = Metal::ALL
                .iter()
                .map(|&metal| (metal, world.accounts.balance_of(world.external_id, metal)))
                .collect();
            // The receipts MUST be bound: `.is_err()` here would compile
            // unchanged and silently discard every forced liquidation,
            // giving a green build that narrates no closure at all.
            let Ok(receipts) = world.remove_agent(agent) else {
                return; // existence checked above — dies cleanly regardless
            };
            // Causal order: the liquidations, then the A17 settlement of
            // OTHER firms' debts, then the departure itself.
            for receipt in &receipts {
                emit_closure(receipt, &name, report);
            }
            for (business, before) in creditors {
                let amount = before.minus(world.accounts.balance_of(business, Metal::Gold));
                if amount > Money::ZERO {
                    report.events.push(Event::Settled {
                        business,
                        agent,
                        amount,
                    });
                }
            }
            let took: Vec<(Metal, Money)> = external_before
                .into_iter()
                .map(|(metal, before)| {
                    (
                        metal,
                        world
                            .accounts
                            .balance_of(world.external_id, metal)
                            .minus(before),
                    )
                })
                .collect();
            report.events.push(Event::Departed { agent, name, took });
        }
        Intent::Buy { .. }
        | Intent::TakeJob { .. }
        | Intent::Quit { .. }
        | Intent::Arrive { .. }
        | Intent::Found { .. } => {
            unreachable!("the sinks apply only receives phase-7 intents")
        }
    }
}

/// Phase 8: new money from reserve. Money ops allowed: mint only.
/// Inert since the 07-19 pricing spec closed the tick-time faucet:
/// worldgen's seed is the entire supply and the §8.3 audit pins each
/// metal's `total_money(metal)` there forever. TODO: the literal staffed
/// Mint business (parent doc §2.1, metal goods → coins) lands here.
fn mint_phase(_world: &mut World) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentId;
    use crate::business::RoleSlot;
    use crate::goods::Good;
    use crate::housing::HouseId;
    use crate::money::Money;
    use crate::role::Role;
    use crate::world::World;
    use std::collections::HashMap;

    /// One single-role business at `wage`, staffed by a freshly spawned
    /// worker. Returns (house, business account, worker).
    fn staffed_business(
        world: &mut World,
        address: &str,
        product: Good,
        price: Money,
        wage: Money,
        worker_name: &str,
    ) -> (HouseId, AgentId, AgentId) {
        let house = world.add_house(address, vec![]);
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage,
                headcount: 1,
                unfilled_ticks: 0,
            },
        );
        let worker = world.spawn_agent(worker_name, None, Some(house));
        world.agent_mut(worker).expect("just spawned").employed_role = Some(Role::Labourer);
        let business = world
            .create_business(house, worker, product, price, roles)
            .expect("fresh house, spawned owner");
        (house, business, worker)
    }

    fn stock_of(world: &World, house: HouseId) -> u32 {
        world.house(house).unwrap().business.as_ref().unwrap().stock
    }

    fn price_of(world: &World, house: HouseId) -> Money {
        world.house(house).unwrap().business.as_ref().unwrap().price
    }

    fn set_stock(world: &mut World, house: HouseId, stock: u32) {
        world
            .house_mut(house)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .stock = stock;
    }

    fn held(world: &World, agent: AgentId, good: Good) -> u32 {
        world
            .agent(agent)
            .unwrap()
            .inventory
            .get(&good)
            .copied()
            .unwrap_or(0)
    }

    fn owed(world: &World, house: HouseId, worker: AgentId) -> Money {
        world
            .house(house)
            .unwrap()
            .business
            .as_ref()
            .unwrap()
            .owed_to
            .get(&worker)
            .copied()
            .unwrap_or(Money::ZERO)
    }

    #[test]
    fn produce_fills_staffed_stock_only() {
        let mut world = World::new();
        let (farm, _, _) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        // unstaffed: business exists, nobody works there
        let idle_house = world.add_house("Idle", vec![]);
        // owner off-premises: the business must stay genuinely unstaffed
        let landlord = world.spawn_agent("landlord", None, None);
        world
            .create_business(
                idle_house,
                landlord,
                Good::Luxury,
                Money::new(5),
                HashMap::new(),
            )
            .unwrap();
        produce(&mut world, &mut TickReport::default());
        assert_eq!(stock_of(&world, farm), Good::Food.production_rate());
        assert_eq!(stock_of(&world, idle_house), 0);
        // stock accumulates tick over tick
        produce(&mut world, &mut TickReport::default());
        assert_eq!(stock_of(&world, farm), 2 * Good::Food.production_rate());
    }

    #[test]
    fn n_ticks_run_clean() {
        let mut world = World::new();
        for _ in 0..100 {
            tick(&mut world);
        }
        // nothing mints yet, so the money supply must still be zero
        assert_eq!(world.accounts.total_money(Metal::Gold), Money::ZERO);
    }

    #[test]
    #[should_panic]
    fn tick_runs_audit_last() {
        let mut world = World::new();
        // corrupt the books via the sanctioned test hook; if any path
        // through tick skipped the audit, this would NOT panic
        world
            .accounts
            .set_balance_for_test(AgentId(7), Metal::Gold, Money::new(999));
        tick(&mut world);
    }

    #[test]
    fn pay_wages_transfers_the_role_wage() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        world.accounts.mint(farm, Metal::Gold, Money::new(50)); // funded
        pay_wages(&mut world, &mut TickReport::default());
        assert_eq!(
            world.accounts.balance_of(worker, Metal::Gold),
            Money::new(35)
        );
        assert_eq!(world.accounts.balance_of(farm, Metal::Gold), Money::new(15));
        // fully funded: nothing carried
        assert_eq!(owed(&world, farm_house, worker), Money::ZERO);
        world.accounts.audit();
    }

    #[test]
    fn underfunded_wage_drains_coffers_and_records_the_rest_as_arrears() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        world.accounts.mint(farm, Metal::Gold, Money::new(10)); // less than the wage
        pay_wages(&mut world, &mut TickReport::default());
        // partial payment IS a full valid transfer of a smaller amount (§8.5)
        assert_eq!(
            world.accounts.balance_of(worker, Metal::Gold),
            Money::new(10)
        );
        assert_eq!(world.accounts.balance_of(farm, Metal::Gold), Money::ZERO);
        assert_eq!(owed(&world, farm_house, worker), Money::new(25));
        world.accounts.audit();
    }

    #[test]
    fn arrears_accrue_and_repay_when_revenue_returns() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        // broke business: the full wage becomes debt, no transfer happens
        pay_wages(&mut world, &mut TickReport::default());
        assert_eq!(world.accounts.balance_of(worker, Metal::Gold), Money::ZERO);
        assert_eq!(owed(&world, farm_house, worker), Money::new(35));
        // revenue returns: this tick's wage joins the pot and all 70 clears
        world.accounts.mint(farm, Metal::Gold, Money::new(100));
        pay_wages(&mut world, &mut TickReport::default());
        assert_eq!(
            world.accounts.balance_of(worker, Metal::Gold),
            Money::new(70)
        );
        assert_eq!(world.accounts.balance_of(farm, Metal::Gold), Money::new(30));
        // paid-off entries leave the map entirely
        assert!(
            world
                .house(farm_house)
                .unwrap()
                .business
                .as_ref()
                .unwrap()
                .owed_to
                .is_empty()
        );
        world.accounts.audit();
    }

    #[test]
    fn unstaffed_business_pays_nobody() {
        let mut world = World::new();
        let house = world.add_house("Idle", vec![]);
        let landlord = world.spawn_agent("landlord", None, None);
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
            .create_business(house, landlord, Good::Food, Money::new(1), roles)
            .unwrap();
        world.accounts.mint(business, Metal::Gold, Money::new(50));
        pay_wages(&mut world, &mut TickReport::default());
        assert_eq!(
            world.accounts.balance_of(business, Metal::Gold),
            Money::new(50)
        );
    }

    #[test]
    fn roleless_worker_earns_nothing() {
        let mut world = World::new();
        let house = world.add_house("Farm", vec![]);
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(35),
                headcount: 1,
                unfilled_ticks: 0,
            },
        );
        // Spawn worker at the workplace but WITHOUT setting employed_role
        let worker = world.spawn_agent("f", None, Some(house));
        let business = world
            .create_business(house, worker, Good::Food, Money::new(1), roles)
            .unwrap();
        // employed_role stays None
        world.accounts.mint(business, Metal::Gold, Money::new(50));
        pay_wages(&mut world, &mut TickReport::default());
        assert_eq!(world.accounts.balance_of(worker, Metal::Gold), Money::ZERO);
        assert_eq!(
            world.accounts.balance_of(business, Metal::Gold),
            Money::new(50)
        );
    }

    #[test]
    fn unslotted_role_earns_nothing() {
        let mut world = World::new();
        let house = world.add_house("Farm", vec![]);
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(35),
                headcount: 1,
                unfilled_ticks: 0,
            },
        );
        // Spawn worker and assign Engineer role, which is NOT in the business's roles
        let worker = world.spawn_agent("e", None, Some(house));
        let business = world
            .create_business(house, worker, Good::Food, Money::new(1), roles)
            .unwrap();
        world.agent_mut(worker).expect("just spawned").employed_role = Some(Role::Engineer);
        world.accounts.mint(business, Metal::Gold, Money::new(50));
        pay_wages(&mut world, &mut TickReport::default());
        assert_eq!(world.accounts.balance_of(worker, Metal::Gold), Money::ZERO);
        assert_eq!(
            world.accounts.balance_of(business, Metal::Gold),
            Money::new(50)
        );
    }

    #[test]
    fn buy_moves_money_and_goods_together() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(2),
            Money::new(35),
            "f",
        );
        set_stock(&mut world, farm_house, 50);
        world.accounts.mint(worker, Metal::Gold, Money::new(10));
        goods_market(&mut world, &mut TickReport::default());
        // 10 coins at price 2 → 5 units, capped well below stock and target
        assert_eq!(held(&world, worker, Good::Food), 5);
        assert_eq!(stock_of(&world, farm_house), 45);
        assert_eq!(world.accounts.balance_of(worker, Metal::Gold), Money::ZERO);
        assert_eq!(world.accounts.balance_of(farm, Metal::Gold), Money::new(10));
        world.accounts.audit();
    }

    #[test]
    fn stale_intents_cap_to_live_stock() {
        let mut world = World::new();
        let (farm_house, _, first) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "a",
        );
        let second = world.spawn_agent("b", None, None);
        set_stock(&mut world, farm_house, 10);
        // both plan against the same 10-unit snapshot and could each afford it
        world.accounts.mint(first, Metal::Gold, Money::new(10));
        world.accounts.mint(second, Metal::Gold, Money::new(10));
        goods_market(&mut world, &mut TickReport::default());
        // agents-order: first drains the shelf, second is capped to zero
        assert_eq!(held(&world, first, Good::Food), 10);
        assert_eq!(held(&world, second, Good::Food), 0);
        assert_eq!(
            world.accounts.balance_of(second, Metal::Gold),
            Money::new(10)
        ); // unspent
        assert_eq!(stock_of(&world, farm_house), 0);
        world.accounts.audit();
    }

    #[test]
    fn broke_buyers_change_nothing() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        set_stock(&mut world, farm_house, 50);
        // no money minted to the worker at all
        goods_market(&mut world, &mut TickReport::default());
        assert_eq!(held(&world, worker, Good::Food), 0);
        assert_eq!(stock_of(&world, farm_house), 50);
        assert_eq!(world.accounts.balance_of(farm, Metal::Gold), Money::ZERO);
    }

    #[test]
    fn consume_drains_inventories_saturating_at_zero() {
        let mut world = World::new();
        let a = world.spawn_agent("a", None, None);
        let agent = world.agent_mut(a).unwrap();
        agent.inventory.insert(Good::Food, 25);
        agent.inventory.insert(Good::Entertainment, 3); // below the rate of 5
        // Luxury absent: stays absent-or-zero, never underflows
        consume(&mut world, &mut TickReport::default());
        assert_eq!(held(&world, a, Good::Food), 15);
        assert_eq!(held(&world, a, Good::Entertainment), 0);
        assert_eq!(held(&world, a, Good::Luxury), 0);
    }

    #[test]
    fn mint_phase_creates_no_money() {
        let mut world = World::new();
        let (_, farm, _) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        world.accounts.mint(farm, Metal::Gold, Money::new(35)); // worldgen-style seed
        mint_phase(&mut world);
        // the tick-time faucet is closed: nothing beyond the seed, ever
        assert_eq!(world.accounts.total_minted(Metal::Gold), Money::new(35));
        assert_eq!(world.accounts.total_money(Metal::Gold), Money::new(35));
        world.accounts.audit();
    }

    // --- Firm-lifecycle pack 1: the phase-6 profit draw ---

    #[test]
    fn draw_amount_clamps_and_respects_arrears() {
        let bill = Money::new(140); // 35 × headcount 4
        // above buffer: the surplus is drawn, integer-exact
        assert_eq!(
            draw_amount(Money::new(500), bill, Money::ZERO),
            Money::new(80) // 500 − 3×140
        );
        // at buffer exactly: nothing (a just-founded firm's state, pack 3)
        assert_eq!(draw_amount(Money::new(420), bill, Money::ZERO), Money::ZERO);
        // below buffer: nothing
        assert_eq!(draw_amount(Money::new(100), bill, Money::ZERO), Money::ZERO);
        // arrears widen the buffer — net of arrears, the erratum as formula
        assert_eq!(
            draw_amount(Money::new(500), bill, Money::new(80)),
            Money::ZERO
        );
        assert_eq!(
            draw_amount(Money::new(500), bill, Money::new(50)),
            Money::new(30)
        );
    }

    /// A venue whose owner is NOT on its payroll — the separation
    /// `staffed_business` cannot give (it makes its sole worker the
    /// owner, so every "a worker departs" test built on it is secretly
    /// an owner-emigration test once forced liquidation lands). The
    /// owner is the inert on-premises landlord of `open_slot_business`:
    /// workplace set, `employed_role` None, so both labor decides skip
    /// them, `staff_in_role` ignores them and payroll accrues them
    /// nothing. Off-premises is NOT an option — an unemployed landlord
    /// would apply for their own open slot and the vacancy would never
    /// age. Same caveat as `open_slot_business`: `produce` counts staff
    /// via the workplace-based `employees_of`, so full-tick tests on
    /// this fixture produce at staff+1.
    fn landlord_owner_business(
        world: &mut World,
        address: &str,
        product: Good,
        price: Money,
        wage: Money,
        headcount: u32,
        owner_name: &str,
    ) -> (HouseId, AgentId, AgentId) {
        let house = world.add_house(address, vec![]);
        let owner = world.spawn_agent(owner_name, None, Some(house));
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage,
                headcount,
                unfilled_ticks: 0,
            },
        );
        let business = world
            .create_business(house, owner, product, price, roles)
            .expect("fresh house, spawned owner");
        (house, business, owner)
    }

    /// Spawns a worker straight into `house`'s Labourer slot.
    fn hire_at(world: &mut World, house: HouseId, name: &str) -> AgentId {
        let worker = world.spawn_agent(name, None, Some(house));
        world.agent_mut(worker).expect("just spawned").employed_role = Some(Role::Labourer);
        worker
    }

    /// Reads the insolvency fuse (pack 2) off a live business.
    fn insolvent_ticks(world: &World, house: HouseId) -> u32 {
        world
            .house(house)
            .unwrap()
            .business
            .as_ref()
            .unwrap()
            .insolvent_ticks
    }

    /// Reads the scarcity-direction streak (pack 3) off a live business.
    fn sold_out_ticks(world: &World, house: HouseId) -> u32 {
        world
            .house(house)
            .unwrap()
            .business
            .as_ref()
            .unwrap()
            .sold_out_ticks
    }

    #[test]
    fn sold_out_ticks_accumulates_resets_and_holds_on_an_empty_shelf() {
        let mut world = World::new();
        let (house, farm, _owner) = landlord_owner_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(5),
            1,
            "boss",
        );
        let buyer = world.spawn_agent("b", None, None);
        world.accounts.mint(buyer, Metal::Gold, Money::new(1000));

        // A thin shelf the buyer clears: sold out, two ticks running.
        for expected in 1..=2 {
            set_stock(&mut world, house, 1);
            let mut report = TickReport::default();
            goods_market(&mut world, &mut report);
            assert_eq!(sold_out_ticks(&world, house), expected);
        }
        // An EMPTY shelf is "no signal" — the streak holds, it does not
        // break. A sold-out seller that produces nothing next tick has
        // not stopped being scarce.
        set_stock(&mut world, house, 0);
        let mut report = TickReport::default();
        goods_market(&mut world, &mut report);
        assert_eq!(sold_out_ticks(&world, house), 2, "empty shelf must hold");
        // A fat shelf the buyer cannot clear is a real signal, and it is
        // not a sell-out: the streak resets.
        set_stock(&mut world, house, 500);
        let mut report = TickReport::default();
        goods_market(&mut world, &mut report);
        assert_eq!(sold_out_ticks(&world, house), 0);
        let _ = farm;
        world.accounts.audit();
    }

    #[test]
    fn sold_out_ticks_has_a_single_writer() {
        // Phase 4's price write-back and nothing else — the discipline
        // `insolvent_ticks` and `unfilled_ticks` already keep.
        let mut world = World::new();
        let (house, farm, _owner) = landlord_owner_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(5),
            1,
            "boss",
        );
        let buyer = world.spawn_agent("b", None, None);
        world.accounts.mint(buyer, Metal::Gold, Money::new(1000));
        world.accounts.mint(farm, Metal::Gold, Money::new(1000));
        set_stock(&mut world, house, 1);
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        assert_eq!(sold_out_ticks(&world, house), 0, "phase 1 must not write");
        produce(&mut world, &mut report);
        assert_eq!(sold_out_ticks(&world, house), 0, "phase 2 must not write");
        pay_wages(&mut world, &mut report);
        assert_eq!(sold_out_ticks(&world, house), 0, "phase 3 must not write");
        set_stock(&mut world, house, 1); // produce added to the shelf
        goods_market(&mut world, &mut report);
        assert_eq!(sold_out_ticks(&world, house), 1, "phase 4 IS the writer");
        consume(&mut world, &mut report);
        assert_eq!(sold_out_ticks(&world, house), 1, "phase 5 must not write");
        invest(&mut world, &mut report);
        assert_eq!(sold_out_ticks(&world, house), 1, "phase 6 must not write");
        sinks(&mut world, &mut report);
        assert_eq!(sold_out_ticks(&world, house), 1, "phase 7 must not write");
        world.accounts.audit();
    }

    #[test]
    fn insolvent_now_is_strict_at_zero() {
        // The predicate is strict-positive: owing nothing is solvent,
        // owing one coin is not. Chosen over a magnitude level because
        // frozen arrears never grow again, so any `> N × bill` gate
        // leaves a venue frozen just under it immortal (pack-2 probe).
        assert!(!insolvent_now(Money::ZERO));
        assert!(insolvent_now(Money::new(1)));
        assert!(insolvent_now(Money::new(1033)));
    }

    #[test]
    fn insolvent_ticks_counts_consecutive_arrears_ticks_and_resets_on_a_clear_one() {
        let mut world = World::new();
        let (house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(20),
            "w",
        );
        // Broke coffer: phase 3 accrues the wage and pays none of it.
        for expected in 1..=3 {
            let mut report = TickReport::default();
            pay_wages(&mut world, &mut report);
            invest(&mut world, &mut report);
            assert_eq!(
                insolvent_ticks(&world, house),
                expected,
                "counter should climb while the ledger stays owed"
            );
        }
        assert!(owed(&world, house, worker) > Money::ZERO);
        // Fund the coffer: next phase 3 clears the whole ledger, and the
        // write-back resets — one clear tick, not a decrement.
        world.accounts.mint(farm, Metal::Gold, Money::new(500));
        let mut report = TickReport::default();
        pay_wages(&mut world, &mut report);
        invest(&mut world, &mut report);
        assert_eq!(owed(&world, house, worker), Money::ZERO);
        assert_eq!(insolvent_ticks(&world, house), 0);
    }

    #[test]
    fn insolvent_ticks_has_a_single_writer() {
        // Every other live phase must leave the counter alone — the
        // field's SINGLE WRITER contract, pinned the way `hunger`'s is.
        let mut world = World::new();
        let (house, farm, _) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(20),
            "w",
        );
        let mut report = TickReport::default();
        pay_wages(&mut world, &mut report); // accrues arrears, pays nothing
        assert_eq!(insolvent_ticks(&world, house), 0, "phase 3 must not write");
        labor_market(&mut world, &mut report);
        assert_eq!(insolvent_ticks(&world, house), 0, "phase 1 must not write");
        produce(&mut world, &mut report);
        assert_eq!(insolvent_ticks(&world, house), 0, "phase 2 must not write");
        goods_market(&mut world, &mut report);
        assert_eq!(insolvent_ticks(&world, house), 0, "phase 4 must not write");
        consume(&mut world, &mut report);
        assert_eq!(insolvent_ticks(&world, house), 0, "phase 5 must not write");
        invest(&mut world, &mut report);
        assert_eq!(insolvent_ticks(&world, house), 1, "phase 6 IS the writer");
        sinks(&mut world, &mut report);
        assert_eq!(insolvent_ticks(&world, house), 1, "phase 7 must not write");
        // ...and a business created mid-run starts the discipline at 0.
        let other = world.add_house("Other", vec![]);
        let owner = world.spawn_agent("o", None, None);
        world
            .create_business(other, owner, Good::Luxury, Money::new(2), HashMap::new())
            .expect("fresh house, spawned owner");
        assert_eq!(insolvent_ticks(&world, other), 0);
        let _ = farm;
    }

    #[test]
    fn draw_pass_pays_owner_and_pins_coffer_at_buffer() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        // coffer well above the 3-bill buffer (bill = 35 × headcount 1)
        world.accounts.mint(farm, Metal::Gold, Money::new(150));
        let mut report = TickReport::default();
        invest(&mut world, &mut report);
        // the owner is the worker (owner-operator fixture): 150 − 105
        assert_eq!(
            world.accounts.balance_of(worker, Metal::Gold),
            Money::new(45)
        );
        assert_eq!(
            world.accounts.balance_of(farm, Metal::Gold),
            Money::new(105)
        );
        assert_eq!(
            report.events,
            vec![Event::ProfitDrawn {
                business: farm,
                owner: worker,
                amount: Money::new(45),
            }]
        );
        world.accounts.audit();
        // at buffer now: a second pass draws nothing and emits nothing
        let mut report = TickReport::default();
        invest(&mut world, &mut report);
        assert_eq!(report.events, vec![]);
        // arrears close the tap even with the coffer above three bills
        world
            .house_mut(farm_house)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .owed_to
            .insert(worker, Money::new(60));
        world.accounts.mint(farm, Metal::Gold, Money::new(50)); // 155 < 105+60
        let mut report = TickReport::default();
        invest(&mut world, &mut report);
        assert_eq!(report.events, vec![]);
        world.accounts.audit();
    }

    /// Two healthy sellers of every good EXCEPT Food, so the existential
    /// tier stays shut and only Food's scarcity tier can fire. Then one
    /// Food seller that has sold out long enough to signal scarcity, an
    /// idle capitalized founder, and a vacant house — the minimum shape
    /// in which founding fires, and the reason a founding fixture cannot
    /// simply omit the goods it does not care about: an absent good has
    /// zero sellers, which tier 1 refounds unconditionally.
    fn founding_ready(founder_gold: Money) -> (World, HouseId, AgentId) {
        let mut world = World::new();
        for (index, good) in [Good::Entertainment, Good::Luxury].into_iter().enumerate() {
            for side in 0..2 {
                landlord_owner_business(
                    &mut world,
                    &format!("filler{index}{side}"),
                    good,
                    Money::new(2),
                    Money::new(30),
                    1,
                    &format!("keeper{index}{side}"),
                );
            }
        }
        let (seller, _s, _boss) = landlord_owner_business(
            &mut world,
            "Seller",
            Good::Food,
            Money::new(9),
            Money::new(35),
            1,
            "boss",
        );
        world
            .house_mut(seller)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .sold_out_ticks = 9;
        let vacant = world.add_house("5 Weir Cottage", vec![]);
        let founder = world.spawn_agent("mira", None, None);
        world.accounts.mint(founder, Metal::Gold, founder_gold);
        (world, vacant, founder)
    }

    #[test]
    fn founding_stakes_self_hires_and_draws_nothing_that_tick() {
        let (mut world, vacant, founder) = founding_ready(Money::new(5000));
        let mut report = TickReport::default();
        invest(&mut world, &mut report);

        let business = world
            .house(vacant)
            .unwrap()
            .business
            .as_ref()
            .expect("the vacant house should now host the founded firm");
        let capital = market::found_template(Good::Food)
            .wage
            .times(market::found_template(Good::Food).headcount * FOUND_CAPITAL_BILLS);
        assert_eq!(business.owner, founder);
        assert_eq!(business.product, Good::Food);
        // Enters AT market — the survivor's live price, not the template's.
        assert_eq!(business.price, Money::new(9));
        let firm = business.id;
        assert_eq!(world.accounts.balance_of(firm, Metal::Gold), capital);
        assert_eq!(
            report.events,
            vec![Event::Founded {
                business: firm,
                founder,
                house: vacant,
                good: Good::Food,
                price: Money::new(9),
                capital,
            }],
            "a founding narrates once and draws NOTHING the same tick"
        );
        // The founder self-hired: the firm produces next tick, and one
        // seat is left for the labor market.
        let person = world.agent(founder).unwrap();
        assert_eq!(person.workplace, Some(vacant));
        assert_eq!(person.employed_role, Some(Role::Labourer));
        assert_eq!(world.employees_of(vacant).len(), 1);
        assert_eq!(
            world
                .house(vacant)
                .unwrap()
                .business
                .as_ref()
                .unwrap()
                .roles[&Role::Labourer]
                .headcount,
            2
        );
        world.accounts.audit();
    }

    #[test]
    fn found_capital_sits_exactly_at_the_draw_buffer() {
        // Structural, not a coincidence: a firm staked at
        // wage_bill × FOUND_CAPITAL_BILLS with no arrears holds exactly
        // draw_amount's buffer, so the stake cannot round-trip to the
        // founder as a dividend inside the same phase 6.
        assert_eq!(FOUND_CAPITAL_BILLS, DRAW_BUFFER_BILLS);
        for good in Good::ALL {
            let template = market::found_template(good);
            let bill = template.wage.times(template.headcount);
            let capital = bill.times(FOUND_CAPITAL_BILLS);
            assert_eq!(
                draw_amount(capital, bill, Money::ZERO),
                Money::ZERO,
                "{good}: a freshly founded firm must draw nothing"
            );
        }
    }

    #[test]
    fn founding_dies_cleanly_on_every_stale_re_check() {
        let capital = Money::new(210);
        let bar = capital.plus(FOUNDER_RESERVE);
        // (a) the founder cannot afford the stake and their reserve
        let (mut world, vacant, _f) = founding_ready(bar.minus(Money::new(1)));
        let mut report = TickReport::default();
        invest(&mut world, &mut report);
        assert!(world.house(vacant).unwrap().business.is_none());
        assert_eq!(report.events, vec![]);
        // exactly at the bar, it fires — so (a) really tested the bound
        let (mut world, vacant, _f) = founding_ready(bar);
        invest(&mut world, &mut TickReport::default());
        assert!(world.house(vacant).unwrap().business.is_some());

        // (b) the sector already has room filled — two sellers
        let (mut world, vacant, _f) = founding_ready(Money::new(5000));
        let (_second, _b, _o) = landlord_owner_business(
            &mut world,
            "Rival",
            Good::Food,
            Money::new(9),
            Money::new(35),
            1,
            "rival",
        );
        let mut report = TickReport::default();
        invest(&mut world, &mut report);
        assert!(world.house(vacant).unwrap().business.is_none());
        assert!(
            !report
                .events
                .iter()
                .any(|e| matches!(e, Event::Founded { .. }))
        );

        // (c) no vacant premises
        let (mut world, vacant, _f) = founding_ready(Money::new(5000));
        world.spawn_agent("squatter", Some(vacant), None);
        let mut report = TickReport::default();
        invest(&mut world, &mut report);
        assert!(world.house(vacant).unwrap().business.is_none());
        assert!(
            !report
                .events
                .iter()
                .any(|e| matches!(e, Event::Founded { .. }))
        );

        // (d) the founder is employed
        let (mut world, vacant, founder) = founding_ready(Money::new(5000));
        let elsewhere = world.add_house("Elsewhere", vec![]);
        world
            .assign_workplace(founder, elsewhere, Role::Labourer)
            .ok();
        let mut report = TickReport::default();
        invest(&mut world, &mut report);
        assert!(world.house(vacant).unwrap().business.is_none());
        assert!(
            !report
                .events
                .iter()
                .any(|e| matches!(e, Event::Founded { .. }))
        );
        world.accounts.audit();
    }

    #[test]
    fn at_most_one_founding_per_tick_by_ascending_founder() {
        // Two dead sectors and three capitalized idle residents: exactly
        // one firm is born, by the lowest-id founder, into the first good
        // in Good::ALL order that qualifies.
        let mut world = World::new();
        // Entertainment alive with two sellers, so only Food and Luxury
        // qualify — and Food is scanned first.
        for name in ["a", "b"] {
            landlord_owner_business(
                &mut world,
                name,
                Good::Entertainment,
                Money::new(2),
                Money::new(36),
                1,
                name,
            );
        }
        world.add_house("5 Weir Cottage", vec![]);
        world.add_house("6 Weir Cottage", vec![]);
        let first = world.spawn_agent("early", None, None);
        let second = world.spawn_agent("later", None, None);
        for who in [first, second] {
            world.accounts.mint(who, Metal::Gold, Money::new(5000));
        }
        let mut report = TickReport::default();
        invest(&mut world, &mut report);
        let founded: Vec<&Event> = report
            .events
            .iter()
            .filter(|e| matches!(e, Event::Founded { .. }))
            .collect();
        assert_eq!(founded.len(), 1, "at most one founding per tick");
        assert!(matches!(
            founded[0],
            Event::Founded { founder, good, .. } if *founder == first && *good == Good::Food
        ));
        world.accounts.audit();
    }

    #[test]
    fn the_decide_reads_the_phase_start_snapshot_so_a_refound_is_next_tick() {
        // A firm closing THIS tick is still a seller when the decide
        // runs, so its sector cannot be refounded in the same phase.
        // Deliberate: the latency can only delay a founding.
        let (mut world, vacant, _f) = founding_ready(Money::new(5000));
        let (doomed, _d, _o) = landlord_owner_business(
            &mut world,
            "Doomed",
            Good::Food,
            Money::new(9),
            Money::new(35),
            1,
            "goner",
        );
        world
            .house_mut(doomed)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .insolvent_ticks = CLOSE_INSOLVENT_TICKS;
        let mut report = TickReport::default();
        invest(&mut world, &mut report);
        assert!(
            report
                .events
                .iter()
                .any(|e| matches!(e, Event::Closed { .. })),
            "the doomed venue should close this tick"
        );
        assert!(
            !report
                .events
                .iter()
                .any(|e| matches!(e, Event::Founded { .. })),
            "two sellers at phase-6 start means no room — the refound waits"
        );
        // Next tick, with one seller left, it fires.
        let mut report = TickReport::default();
        invest(&mut world, &mut report);
        assert!(
            report
                .events
                .iter()
                .any(|e| matches!(e, Event::Founded { .. }))
        );
        assert!(world.house(vacant).unwrap().business.is_some());
        world.accounts.audit();
    }

    #[test]
    fn closure_fires_on_persistence_and_narrates_from_the_receipt() {
        // The doomed venue that dies with its staff STILL EMPLOYED, so
        // the layoffs are observable. The shortfall is deliberately tiny:
        // per worker per tick it must satisfy s ≤ 3w/(k+1) — three bills
        // is the quit bar (`QUIT_ARREARS_BILLS`) and the venue survives
        // k+1 payroll ticks — or the workers walk first and this silently
        // becomes a quit test with an empty LaidOff vector, still green,
        // proving less. Here w = 40 and s = 2.
        //
        // `labor_market` is deliberately NOT run: its wage write-back
        // would move the bill under the arithmetic above. The quit path
        // is pinned separately by `closure_fires_after_quits_freeze_the_
        // ledger`.
        let mut world = World::new();
        let (house, venue, owner) = landlord_owner_business(
            &mut world,
            "The Brass Bell",
            Good::Entertainment,
            Money::new(3),
            Money::new(40),
            2,
            "karl",
        );
        let first = hire_at(&mut world, house, "a");
        let second = hire_at(&mut world, house, "b");

        // Twelve ticks of very nearly making payroll: 78g against an 80g
        // bill, so exactly one worker ends each tick 2g short.
        for tick in 1..=CLOSE_INSOLVENT_TICKS {
            world.accounts.mint(venue, Metal::Gold, Money::new(78));
            let mut report = TickReport::default();
            pay_wages(&mut world, &mut report);
            invest(&mut world, &mut report);
            assert_eq!(insolvent_ticks(&world, house), tick);
            assert!(
                world.house(house).unwrap().business.is_some(),
                "must not close before the fuse burns down"
            );
        }
        // The thirteenth tick: the counter is AT the threshold when phase
        // 6 reads the phase-start snapshot — one tick of designed latency
        // after it crossed.
        world.accounts.mint(venue, Metal::Gold, Money::new(78));
        let mut report = TickReport::default();
        pay_wages(&mut world, &mut report);
        // Phase-4 revenue lands AFTER payroll — the one way a venue
        // carrying arrears can hold gold at phase 6. Enough to clear the
        // ledger and leave 12g over.
        let arrears = world
            .house(house)
            .unwrap()
            .business
            .as_ref()
            .unwrap()
            .owed_total();
        world
            .accounts
            .mint(venue, Metal::Gold, arrears.plus(Money::new(12)));
        let mut report = TickReport::default();
        invest(&mut world, &mut report);

        // Neither worker was ever close to walking out.
        assert!(arrears < Money::new(40).times(QUIT_ARREARS_BILLS));
        // Settlements first, then the layoffs, then the closure itself.
        assert_eq!(
            report.events,
            vec![
                Event::Settled {
                    business: venue,
                    agent: second, // the only creditor: `first` is paid in full
                    amount: arrears,
                },
                // ascending AgentId, and the on-premises landlord counts:
                // closure clears EVERY workplace pointing at the venue,
                // owner included, so nobody is left working at a firm
                // that no longer exists
                Event::LaidOff {
                    agent: owner,
                    business: venue,
                },
                Event::LaidOff {
                    agent: first,
                    business: venue,
                },
                Event::LaidOff {
                    agent: second,
                    business: venue,
                },
                Event::Closed {
                    business: venue,
                    house,
                    owner,
                    owner_name: "karl".to_string(),
                    proceeds: vec![
                        (Metal::Gold, Money::new(12)),
                        (Metal::Silver, Money::ZERO),
                        (Metal::Copper, Money::ZERO),
                    ],
                },
            ],
            "closure narrates settlements, then layoffs, then the death"
        );
        // karl pockets 12g; the firm is gone and holds nothing.
        assert_eq!(
            world.accounts.balance_of(owner, Metal::Gold),
            Money::new(12)
        );
        assert!(world.house(house).unwrap().business.is_none());
        for metal in Metal::ALL {
            assert_eq!(world.accounts.balance_of(venue, metal), Money::ZERO);
        }
        // The landlord-owner is laid off with everyone else, so nobody is
        // left standing in a firm that no longer exists.
        for worker in [first, second, owner] {
            assert_eq!(world.agent(worker).unwrap().workplace, None);
        }
        world.accounts.audit();
    }

    #[test]
    fn a_closing_firm_never_draws() {
        // Pins the ordering the closure pass is placed for. The doomed
        // venue is ALSO above its draw buffer — reachable, because
        // phase-4 revenue lands after phase 3 carried the debt — so a
        // closure pass sitting BELOW the draws collect would pay its
        // owner a dividend out of money the creditors are owed, and then
        // liquidate. Comments claimed this was structural; nothing tested
        // it until now.
        let mut world = World::new();
        let (house, venue, owner) = landlord_owner_business(
            &mut world,
            "Doomed",
            Good::Entertainment,
            Money::new(3),
            Money::new(10),
            1,
            "boss",
        );
        // Fuse already burnt down, and a coffer far above 3 bills + owed.
        world
            .house_mut(house)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .insolvent_ticks = CLOSE_INSOLVENT_TICKS;
        world
            .house_mut(house)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .owed_to
            .insert(owner, Money::new(5));
        world.accounts.mint(venue, Metal::Gold, Money::new(500));
        assert!(
            draw_amount(Money::new(500), Money::new(10), Money::new(5)) > Money::ZERO,
            "the fixture must be draw-eligible, or this proves nothing"
        );

        let mut report = TickReport::default();
        invest(&mut world, &mut report);

        assert!(
            !report
                .events
                .iter()
                .any(|event| matches!(event, Event::ProfitDrawn { .. })),
            "a closing firm drew profit: {:?}",
            report.events
        );
        assert!(world.house(house).unwrap().business.is_none());
        // Everything it held reached the owner as liquidation PROCEEDS
        // (settlement + residual), not as a dividend taken ahead of them.
        assert_eq!(
            world.accounts.balance_of(owner, Metal::Gold),
            Money::new(500)
        );
        world.accounts.audit();
    }

    #[test]
    fn arrears_conjunct_gates_the_arrive_decide() {
        // DECIDE-site probe: only the deadbeat's slot is AGED, while a
        // clean venue has open headcount that has not aged. The apply's
        // `still_hiring` is therefore satisfied by the clean venue, so a
        // refusal can only come from the decide scan's arrears conjunct.
        let mut world = World::new();
        let (deadbeat, _d, _boss) = landlord_owner_business(
            &mut world,
            "Deadbeat",
            Good::Food,
            Money::new(2),
            Money::new(30),
            1,
            "dodger",
        );
        let ghost = world.spawn_agent("ghost", None, Some(deadbeat));
        world
            .house_mut(deadbeat)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .owed_to
            .insert(ghost, Money::new(1));
        world.add_house("5 Weir Cottage", vec![]);
        world
            .accounts
            .mint(world.external_id, Metal::Gold, Money::new(500));
        // Age ONLY the deadbeat's slot.
        world
            .house_mut(deadbeat)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .roles
            .get_mut(&Role::Labourer)
            .unwrap()
            .unfilled_ticks = VACANCY_PULL_TICKS;
        // A clean venue with open, UN-aged headcount keeps the apply's
        // re-check satisfiable.
        let (clean, _c, _b2) = landlord_owner_business(
            &mut world,
            "Clean",
            Good::Luxury,
            Money::new(2),
            Money::new(30),
            1,
            "honest",
        );
        let _ = clean;

        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        assert!(
            !report
                .events
                .iter()
                .any(|event| matches!(event, Event::Arrived { .. })),
            "the decide must not age-scan a venue that owes back wages"
        );
        world.accounts.audit();
    }

    #[test]
    fn arrears_conjunct_gates_the_arrive_apply() {
        // APPLY-site probe. Two venues: a CLEAN, AGED one justifies the
        // decide, and a DEADBEAT one is the only place still hiring by
        // the time the apply's `still_hiring` re-check runs — because a
        // local applicant fills the clean venue earlier in the same apply
        // loop (applications are applied before Arrive). Without the
        // conjunct on the re-check, the deadbeat's open headcount would
        // confirm the arrival; with it, the intent dies cleanly.
        let mut world = World::new();
        let (clean, _c, _boss) = landlord_owner_business(
            &mut world,
            "Clean",
            Good::Food,
            Money::new(2),
            Money::new(50), // the higher wage, so the local applies here
            1,
            "honest",
        );
        let (deadbeat, _d, _dodger) = landlord_owner_business(
            &mut world,
            "Deadbeat",
            Good::Luxury,
            Money::new(2),
            Money::new(10),
            1,
            "dodger",
        );
        // A creditor who is employed, so they never compete for a slot.
        let ghost = world.spawn_agent("ghost", None, Some(deadbeat));
        world
            .house_mut(deadbeat)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .owed_to
            .insert(ghost, Money::new(1));
        // The clean venue is the aged one — it alone justifies the pull.
        world
            .house_mut(clean)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .roles
            .get_mut(&Role::Labourer)
            .unwrap()
            .unfilled_ticks = VACANCY_PULL_TICKS;
        let local = world.spawn_agent("local", None, None);
        world.add_house("5 Weir Cottage", vec![]);
        world
            .accounts
            .mint(world.external_id, Metal::Gold, Money::new(500));

        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);

        // The local took the clean venue, so it is no longer hiring...
        assert!(
            report.events.iter().any(|event| matches!(
                event,
                Event::Hired { agent, .. } if *agent == local
            )),
            "the local must fill the clean venue for this to test the apply"
        );
        // ...leaving only the deadbeat's open headcount, which cannot
        // confirm an arrival.
        assert!(
            !report
                .events
                .iter()
                .any(|event| matches!(event, Event::Arrived { .. })),
            "an owing venue must not confirm an arrival: {:?}",
            report.events
        );
        world.accounts.audit();
    }

    #[test]
    fn closure_fires_after_quits_freeze_the_ledger() {
        // The reachability proof an arrears-LEVEL gate cannot give: a
        // worker walks out, their entry freezes (pay_wages iterates
        // current employees only), the deadbeat exclusion keeps them from
        // reapplying — and the venue is left owing a debt no mechanic can
        // ever pay down. Persistence is the one signal that still fires.
        let mut world = World::new();
        let (house, venue, _owner) = landlord_owner_business(
            &mut world,
            "Doomed",
            Good::Entertainment,
            Money::new(3),
            Money::new(20),
            1,
            "boss",
        );
        let worker = hire_at(&mut world, house, "w");
        let mut quit_tick = None;
        let mut closed_tick = None;
        for tick in 1..=(CLOSE_INSOLVENT_TICKS + 1) {
            let mut report = TickReport::default();
            labor_market(&mut world, &mut report);
            pay_wages(&mut world, &mut report);
            invest(&mut world, &mut report);
            for event in &report.events {
                match event {
                    Event::Quit { .. } => quit_tick = Some(tick),
                    Event::Closed { .. } => closed_tick = Some(tick),
                    _ => {}
                }
            }
        }
        // The cheaper correction fires first — by a wide margin at this
        // shortfall size, which is the ordering the trigger is tuned for.
        let quit = quit_tick.expect("the worker never walked out");
        let closed = closed_tick.expect("the venue never closed");
        assert!(
            quit < closed,
            "worker churn must precede closure (quit t{quit}, closed t{closed})"
        );
        assert!(world.house(house).unwrap().business.is_none());
        // Nothing was left to pay the frozen debt with, and the ex-worker
        // keeps whatever payroll managed before the coffer ran dry.
        for metal in Metal::ALL {
            assert_eq!(world.accounts.balance_of(venue, metal), Money::ZERO);
        }
        assert!(world.agent(worker).is_some(), "a quitter stays in town");
        world.accounts.audit();
    }

    /// The first playable loop, end to end: one farm, one worker, one
    /// unemployed agent, seeded exactly like worldgen (wage bill on the
    /// business; wallet + one day's goods per agent). Every tick audits.
    /// Since pack 4 the story ends differently: the idle agent's ruin
    /// (07-19: nobody saves the unemployed) now plays out as emigration
    /// — broke by t1, hungry from ~t5, gone by ~t9.
    #[test]
    fn minimal_economy_feeds_the_worker_and_the_idle_leaves_town() {
        let (mut world, farm_house, worker, idle) = seeded_minimal_economy();
        for _ in 0..10 {
            tick(&mut world); // audit runs inside — any §8 break panics here
        }
        // the worldgen seed (3 × 35) is the ENTIRE money supply, forever
        // — the audit pins it there every tick, departures included
        assert_eq!(world.accounts.total_minted(Metal::Gold), Money::new(105));
        assert_eq!(world.accounts.total_money(Metal::Gold), Money::new(105));
        // the worker keeps earning, eating, and holding stock
        assert!(world.accounts.balance_of(worker, Metal::Gold) > Money::ZERO);
        assert!(held(&world, worker, Good::Food) > 0);
        // the idle agent earned nothing, drained their wallet, went
        // hungry, and emigrated (pack 4's push rule) — penniless, so
        // the sweep moved nothing
        assert!(world.agent(idle).is_none());
        assert_eq!(world.accounts.balance_of(idle, Metal::Gold), Money::ZERO);
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Gold),
            Money::ZERO
        );
        // overproduction piles up on the shelf (40/tick made, ~10 eaten)
        assert!(stock_of(&world, farm_house) > 0);
    }

    #[test]
    fn sell_out_raises_the_price_for_the_next_tick() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(2),
            Money::new(35),
            "f",
        );
        set_stock(&mut world, farm_house, 10);
        world.accounts.mint(worker, Metal::Gold, Money::new(20));
        goods_market(&mut world, &mut TickReport::default());
        // the whole shelf sold at the OLD price (10 × 2 = 20 coins)…
        assert_eq!(held(&world, worker, Good::Food), 10);
        assert_eq!(world.accounts.balance_of(farm, Metal::Gold), Money::new(20));
        // …and the new price only exists after the phase: 2 + max(1, 2/10)
        assert_eq!(price_of(&world, farm_house), Money::new(3));
        world.accounts.audit();
    }

    #[test]
    fn poor_sales_lower_the_price_saturating_at_the_floor() {
        let mut world = World::new();
        let (farm_house, _, _) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(5),
            Money::new(35),
            "f",
        );
        let (stall_house, _, _) = staffed_business(
            &mut world,
            "Stall",
            Good::Entertainment,
            Money::new(1),
            Money::new(35),
            "s",
        );
        set_stock(&mut world, farm_house, 50);
        set_stock(&mut world, stall_house, 50);
        // nobody has money → 0 of 50 sold everywhere
        goods_market(&mut world, &mut TickReport::default());
        assert_eq!(price_of(&world, farm_house), Money::new(4));
        // a floor-price seller stays at the floor
        assert_eq!(price_of(&world, stall_house), Money::new(1));
    }

    #[test]
    fn empty_shelf_gives_no_price_signal() {
        let mut world = World::new();
        let (farm_house, _, _) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(7),
            Money::new(35),
            "f",
        );
        // stock 0 → offered 0: the price holds, NOT treated as poor sales
        goods_market(&mut world, &mut TickReport::default());
        assert_eq!(price_of(&world, farm_house), Money::new(7));
    }

    /// Adds a second Labourer to a `staffed_business` fixture, widening
    /// the slot's headcount to match (pack 2: multi-worker).
    fn second_worker(world: &mut World, house: HouseId, name: &str) -> AgentId {
        world
            .house_mut(house)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .roles
            .get_mut(&Role::Labourer)
            .unwrap()
            .headcount = 2;
        let worker = world.spawn_agent(name, None, Some(house));
        world.agent_mut(worker).unwrap().employed_role = Some(Role::Labourer);
        worker
    }

    #[test]
    fn produce_scales_with_staff_count() {
        let mut world = World::new();
        let (farm_house, _, _) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        second_worker(&mut world, farm_house, "g");
        produce(&mut world, &mut TickReport::default());
        assert_eq!(
            stock_of(&world, farm_house),
            2 * Good::Food.production_rate()
        );
    }

    #[test]
    fn payroll_pays_every_employee_in_ascending_order() {
        let mut world = World::new();
        let (farm_house, farm, first) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        let second = second_worker(&mut world, farm_house, "g");
        // coffers cover one and a half wages — the shared pot drains in
        // ascending-id order, so the first is whole and the second short
        world.accounts.mint(farm, Metal::Gold, Money::new(52));
        let mut report = TickReport::default();
        pay_wages(&mut world, &mut report);
        assert_eq!(
            world.accounts.balance_of(first, Metal::Gold),
            Money::new(35)
        );
        assert_eq!(
            world.accounts.balance_of(second, Metal::Gold),
            Money::new(17)
        );
        assert_eq!(owed(&world, farm_house, first), Money::ZERO);
        assert_eq!(owed(&world, farm_house, second), Money::new(18));
        assert_eq!(
            report.events,
            vec![
                Event::WagePaid {
                    business: farm,
                    worker: first,
                    amount: Money::new(35),
                },
                Event::WagePaid {
                    business: farm,
                    worker: second,
                    amount: Money::new(17),
                },
                Event::PayrollShort {
                    business: farm,
                    worker: second,
                    remaining: Money::new(18),
                },
            ]
        );
        world.accounts.audit();
    }

    // --- Pack-1 emission tests: each behavior phase narrates what it did ---

    #[test]
    fn produce_emits_produced_for_staffed_only() {
        let mut world = World::new();
        let (_, farm, _) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        let idle_house = world.add_house("Idle", vec![]);
        // owner off-premises: the business must stay genuinely unstaffed
        let landlord = world.spawn_agent("landlord", None, None);
        world
            .create_business(
                idle_house,
                landlord,
                Good::Luxury,
                Money::new(5),
                HashMap::new(),
            )
            .unwrap();
        let mut report = TickReport::default();
        produce(&mut world, &mut report);
        assert_eq!(
            report.events,
            vec![Event::Produced {
                business: farm,
                good: Good::Food,
                units: Good::Food.production_rate(),
            }]
        );
    }

    #[test]
    fn funded_wages_emit_paid_only() {
        let mut world = World::new();
        let (_, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        world.accounts.mint(farm, Metal::Gold, Money::new(50));
        let mut report = TickReport::default();
        pay_wages(&mut world, &mut report);
        assert_eq!(
            report.events,
            vec![Event::WagePaid {
                business: farm,
                worker,
                amount: Money::new(35),
            }]
        );
    }

    #[test]
    fn underfunded_wages_emit_paid_and_shortfall() {
        let mut world = World::new();
        let (_, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        world.accounts.mint(farm, Metal::Gold, Money::new(10));
        let mut report = TickReport::default();
        pay_wages(&mut world, &mut report);
        assert_eq!(
            report.events,
            vec![
                Event::WagePaid {
                    business: farm,
                    worker,
                    amount: Money::new(10),
                },
                Event::PayrollShort {
                    business: farm,
                    worker,
                    remaining: Money::new(25),
                },
            ]
        );
    }

    #[test]
    fn broke_wages_emit_shortfall_only() {
        let mut world = World::new();
        let (_, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        let mut report = TickReport::default();
        pay_wages(&mut world, &mut report);
        assert_eq!(
            report.events,
            vec![Event::PayrollShort {
                business: farm,
                worker,
                remaining: Money::new(35),
            }]
        );
    }

    #[test]
    fn sales_and_price_moves_are_narrated() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(2),
            Money::new(35),
            "f",
        );
        set_stock(&mut world, farm_house, 10);
        world.accounts.mint(worker, Metal::Gold, Money::new(20));
        let mut report = TickReport::default();
        goods_market(&mut world, &mut report);
        // apply first (at the snapshot price), then the write-back
        assert_eq!(
            report.events,
            vec![
                Event::Sold {
                    business: farm,
                    buyer: worker,
                    good: Good::Food,
                    units: 10,
                    price: Money::new(2),
                },
                Event::PriceMoved {
                    business: farm,
                    good: Good::Food,
                    from: Money::new(2),
                    to: Money::new(3),
                },
            ]
        );
    }

    #[test]
    fn held_prices_and_empty_shelves_emit_nothing() {
        let mut world = World::new();
        // empty shelf: no signal, price holds
        staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(7),
            Money::new(35),
            "f",
        );
        // floor-price seller with poor sales: lowered-but-floored is a hold
        let (stall_house, _, _) = staffed_business(
            &mut world,
            "Stall",
            Good::Entertainment,
            Money::new(1),
            Money::new(35),
            "s",
        );
        set_stock(&mut world, stall_house, 50);
        let mut report = TickReport::default();
        goods_market(&mut world, &mut report);
        assert_eq!(report.events, vec![]);
    }

    #[test]
    fn hunger_counts_short_ticks_saturating_and_resets_when_fed() {
        let mut world = World::new();
        let agent = world.spawn_agent("a", None, None);
        assert_eq!(world.agent(agent).unwrap().hunger, 0); // starts fed
        // short every tick: the counter climbs one per consume
        consume(&mut world, &mut TickReport::default());
        consume(&mut world, &mut TickReport::default());
        assert_eq!(world.agent(agent).unwrap().hunger, 2);
        // saturates at u8::MAX instead of wrapping
        world.agent_mut(agent).unwrap().hunger = u8::MAX;
        consume(&mut world, &mut TickReport::default());
        assert_eq!(world.agent(agent).unwrap().hunger, u8::MAX);
        // one fully-fed tick resets it to zero
        world
            .agent_mut(agent)
            .unwrap()
            .inventory
            .insert(Good::Food, Good::Food.consumption_rate());
        consume(&mut world, &mut TickReport::default());
        assert_eq!(world.agent(agent).unwrap().hunger, 0);
    }

    #[test]
    fn consume_narrates_going_hungry() {
        let mut world = World::new();
        let short = world.spawn_agent("short", None, None);
        let fed = world.spawn_agent("fed", None, None);
        world
            .agent_mut(short)
            .unwrap()
            .inventory
            .insert(Good::Food, Good::Food.consumption_rate() - 1);
        world
            .agent_mut(fed)
            .unwrap()
            .inventory
            .insert(Good::Food, Good::Food.consumption_rate());
        let mut report = TickReport::default();
        consume(&mut world, &mut report);
        assert_eq!(report.events, vec![Event::WentHungry { agent: short }]);
    }

    /// Flattened observable state, for comparing two runs. Deterministic
    /// order: agents then business-hosting houses, in world order.
    fn digest(world: &World) -> Vec<String> {
        let mut lines = Vec::new();
        for agent in &world.agents {
            let goods: Vec<String> = Good::ALL
                .iter()
                .map(|g| format!("{g}:{}", agent.inventory.get(g).copied().unwrap_or(0)))
                .collect();
            lines.push(format!(
                "{} {} {}",
                agent.name,
                world.accounts.balance_of(agent.id, Metal::Gold),
                goods.join(","),
            ));
        }
        for house in &world.houses {
            if let Some(b) = &house.business {
                lines.push(format!(
                    "{} {} {} {} {}",
                    house.address,
                    b.price,
                    b.stock,
                    world.accounts.balance_of(b.id, Metal::Gold),
                    b.owed_total(),
                ));
            }
        }
        lines
    }

    /// The minimal 07-19 economy, seeded exactly like worldgen: one
    /// staffed farm (wage bill pre-funded), one unemployed agent, wallet
    /// plus one day's goods each. Returns (world, farm house, worker,
    /// idle).
    fn seeded_minimal_economy() -> (World, HouseId, AgentId, AgentId) {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        let idle = world.spawn_agent("idle", None, None);
        world.accounts.mint(farm, Metal::Gold, Money::new(35));
        for id in [worker, idle] {
            world.accounts.mint(id, Metal::Gold, Money::new(35));
            let agent = world.agent_mut(id).unwrap();
            for good in Good::ALL {
                agent.inventory.insert(good, good.consumption_rate());
            }
        }
        (world, farm_house, worker, idle)
    }

    #[test]
    fn zero_wage_slot_emits_nothing() {
        let mut world = World::new();
        staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::ZERO,
            "f",
        );
        let mut report = TickReport::default();
        pay_wages(&mut world, &mut report);
        // nothing is owed, so there is no shortfall to narrate
        assert_eq!(report.events, vec![]);
    }

    // --- Pack-3 labor-market tests: phase 1 hires, quits, and floats wages ---

    /// A business with one open Labourer slot at `wage` and no workers —
    /// the labor market's raw material. Returns (house, business account).
    fn open_slot_business(
        world: &mut World,
        address: &str,
        product: Good,
        wage: Money,
    ) -> (HouseId, AgentId) {
        let house = world.add_house(address, vec![]);
        // Inert owner-on-premises: workplace set, employed_role None —
        // both labor decides skip them (employed ⇒ no application,
        // roleless ⇒ no quit), `staff_in_role` ignores them, and payroll
        // accrues them nothing — so the slot stays genuinely open. ONE
        // caveat: produce counts staff via the workplace-based
        // `employees_of`, so full-tick tests on this fixture produce at
        // staff+1 — don't assert stock or Produced units here without
        // accounting for it. (Off-premises is not an option: an
        // unemployed landlord would apply for the open slot.)
        let landlord = world.spawn_agent("landlord", None, Some(house));
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage,
                headcount: 1,
                unfilled_ticks: 0,
            },
        );
        let business = world
            .create_business(house, landlord, product, Money::new(1), roles)
            .expect("fresh house, spawned owner");
        (house, business)
    }

    #[test]
    fn hiring_fills_the_slot_and_moves_no_money() {
        let mut world = World::new();
        let (house, farm) = open_slot_business(&mut world, "Farm", Good::Food, Money::new(35));
        let unemployed = world.spawn_agent("u", None, None);
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        assert_eq!(
            report.events,
            vec![Event::Hired {
                agent: unemployed,
                business: farm,
                role: Role::Labourer,
                wage: Money::new(35),
            }]
        );
        let agent = world.agent(unemployed).unwrap();
        assert_eq!(agent.workplace, Some(house));
        assert_eq!(agent.employed_role, Some(Role::Labourer));
        // phase 1's money-op row is "none": nothing entered the books
        assert_eq!(world.accounts.total_money(Metal::Gold), Money::ZERO);
        world.accounts.audit();
    }

    #[test]
    fn unfilled_slot_raises_the_wage_only_when_affordable() {
        // nobody to hire — the vacancy is the raise signal
        let mut world = World::new();
        let (farm_house, farm) = open_slot_business(&mut world, "Farm", Good::Food, Money::new(35));
        // one coin short of the stepped bill (38 × headcount 1): held
        world.accounts.mint(farm, Metal::Gold, Money::new(37));
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        assert_eq!(report.events, vec![]);
        // exactly the stepped bill: the raise posts, effective next tick
        world.accounts.mint(farm, Metal::Gold, Money::new(1));
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        assert_eq!(
            report.events,
            vec![Event::WageMoved {
                business: farm,
                role: Role::Labourer,
                from: Money::new(35),
                to: Money::new(38),
            }]
        );
        let wage = world
            .house(farm_house)
            .unwrap()
            .business
            .as_ref()
            .unwrap()
            .roles[&Role::Labourer]
            .wage;
        assert_eq!(wage, Money::new(38));
    }

    #[test]
    fn stale_takejob_dies_on_live_headcount() {
        let mut world = World::new();
        let (farm_house, farm) = open_slot_business(&mut world, "Farm", Good::Food, Money::new(35));
        let first = world.spawn_agent("a", None, None);
        let second = world.spawn_agent("b", None, None);
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        // both raced the one slot off the same snapshot; ascending id wins,
        // the loser's stale intent dies on the live headcount re-check —
        // and becomes the queue that lowers the wage for next tick
        assert_eq!(
            report.events,
            vec![
                Event::Hired {
                    agent: first,
                    business: farm,
                    role: Role::Labourer,
                    wage: Money::new(35),
                },
                Event::WageMoved {
                    business: farm,
                    role: Role::Labourer,
                    from: Money::new(35),
                    to: Money::new(32),
                },
            ]
        );
        assert_eq!(world.agent(first).unwrap().workplace, Some(farm_house));
        // nothing about the loser changed
        let loser = world.agent(second).unwrap();
        assert_eq!(loser.workplace, None);
        assert_eq!(loser.employed_role, None);
        assert_eq!(world.accounts.total_money(Metal::Gold), Money::ZERO);
    }

    #[test]
    fn hire_earns_role_wage_next_pay_wages() {
        let mut world = World::new();
        let (_, farm) = open_slot_business(&mut world, "Farm", Good::Food, Money::new(35));
        world.accounts.mint(farm, Metal::Gold, Money::new(50));
        let unemployed = world.spawn_agent("u", None, None);
        let report = tick(&mut world);
        // hired in phase 1, paid in phase 3 — the same tick's payroll
        // sees the employed_role write (a workplace-only hire would
        // never earn: `roleless_worker_earns_nothing`)
        assert!(report.events.contains(&Event::Hired {
            agent: unemployed,
            business: farm,
            role: Role::Labourer,
            wage: Money::new(35),
        }));
        assert!(report.events.contains(&Event::WagePaid {
            business: farm,
            worker: unemployed,
            amount: Money::new(35),
        }));
    }

    #[test]
    fn quit_on_arrears_fires_at_n_preserves_owed_to_and_clears_role() {
        let mut world = World::new();
        let wage = Money::new(35);
        let (farm_house, farm, worker) =
            staffed_business(&mut world, "Farm", Good::Food, Money::new(1), wage, "f");
        // a broke farm: every payroll accrues the full wage as arrears.
        // Amounts are written N-relative so tuning QUIT_ARREARS_BILLS
        // never touches this test's logic.
        for _ in 0..QUIT_ARREARS_BILLS {
            pay_wages(&mut world, &mut TickReport::default());
        }
        // exactly N bills owed is endured — the strictly-greater rule
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        assert_eq!(report.events, vec![]);
        assert_eq!(world.agent(worker).unwrap().workplace, Some(farm_house));
        // one more unpaid tick crosses the threshold
        pay_wages(&mut world, &mut TickReport::default());
        let past_threshold = wage.times(QUIT_ARREARS_BILLS + 1);
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        assert_eq!(
            report.events,
            vec![Event::Quit {
                agent: worker,
                business: farm,
                owed: past_threshold,
            }]
        );
        let quitter = world.agent(worker).unwrap();
        assert_eq!(quitter.workplace, None);
        assert_eq!(quitter.employed_role, None);
        // the debt survives the walkout (settlement is pack 4's)
        assert_eq!(owed(&world, farm_house, worker), past_threshold);
        // and the deadbeat exclusion holds: the only open slot belongs to
        // the employer still owing them, so the quitter stays out
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        assert_eq!(report.events, vec![]);
        assert_eq!(world.agent(worker).unwrap().workplace, None);
    }

    #[test]
    fn quitter_re_enters_the_pool_next_tick_not_same_tick() {
        let mut world = World::new();
        let wage = Money::new(35);
        // the deadbeat: broke, arrears pushed past the threshold
        let (_, deadbeat, worker) = staffed_business(
            &mut world,
            "Deadbeat Hall",
            Good::Entertainment,
            Money::new(2),
            wage,
            "w",
        );
        for _ in 0..=QUIT_ARREARS_BILLS {
            pay_wages(&mut world, &mut TickReport::default());
        }
        // an independent solvent employer with room (headcount 2), plus a
        // second unemployed agent so the same tick carries a quit AND an
        // unrelated hire — pinning the quits-before-hires event order
        let solvent_house = world.add_house("Solvent & Sons", vec![]);
        let solvent_landlord = world.spawn_agent("landlord", None, Some(solvent_house));
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(30),
                headcount: 2,
                unfilled_ticks: 0,
            },
        );
        let solvent = world
            .create_business(
                solvent_house,
                solvent_landlord,
                Good::Food,
                Money::new(1),
                roles,
            )
            .unwrap();
        let bystander = world.spawn_agent("b", None, None);
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        // the quitter does NOT apply this tick, even with an open slot
        // they could take — they re-enter the pool next tick; the
        // bystander's hire proves the slot was genuinely open, and the
        // full-vector equality pins quits indexing before hires
        assert_eq!(
            report.events,
            vec![
                Event::Quit {
                    agent: worker,
                    business: deadbeat,
                    owed: wage.times(QUIT_ARREARS_BILLS + 1),
                },
                Event::Hired {
                    agent: bystander,
                    business: solvent,
                    role: Role::Labourer,
                    wage: Money::new(30),
                },
            ]
        );
        // next tick the quitter applies: the deadbeat's reopened slot
        // posts the better wage but is excluded, so the solvent
        // business wins — the exclusion bars the creditor, nothing more
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        assert_eq!(
            report.events,
            vec![Event::Hired {
                agent: worker,
                business: solvent,
                role: Role::Labourer,
                wage: Money::new(30),
            }]
        );
        assert_eq!(world.agent(worker).unwrap().workplace, Some(solvent_house));
    }

    #[test]
    fn same_tick_raise_lands_in_that_payroll_hired_wage_is_the_snapshot() {
        // the Erratum's reading, pinned by divergence: phase-3 payroll
        // reads the live slot wage, while Event::Hired carries the
        // snapshot wage the agent applied at
        let mut world = World::new();
        let house = world.add_house("Farm", vec![]);
        let farm_landlord = world.spawn_agent("landlord", None, Some(house));
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(35),
                headcount: 2,
                unfilled_ticks: 0,
            },
        );
        let farm = world
            .create_business(house, farm_landlord, Good::Food, Money::new(1), roles)
            .unwrap();
        world.accounts.mint(farm, Metal::Gold, Money::new(200));
        let unemployed = world.spawn_agent("u", None, None);
        let report = tick(&mut world);
        assert!(report.events.contains(&Event::Hired {
            agent: unemployed,
            business: farm,
            role: Role::Labourer,
            wage: Money::new(35),
        }));
        // the still-open second slot raised 35 → 38 at the end of phase
        // 1, and that same tick's payroll pays 38
        assert!(report.events.contains(&Event::WagePaid {
            business: farm,
            worker: unemployed,
            amount: Money::new(38),
        }));
    }

    #[test]
    fn floor_wage_queue_clamp_is_a_held_wage() {
        let mut world = World::new();
        let (_, shop) = open_slot_business(&mut world, "Shop", Good::Food, Money::new(1));
        let first = world.spawn_agent("a", None, None);
        world.spawn_agent("b", None, None);
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        // the loser's stale application queues, but the floor clamp is a
        // hold — Hired only, no WageMoved (the price side's precedent:
        // lowered-but-floored emits nothing)
        assert_eq!(
            report.events,
            vec![Event::Hired {
                agent: first,
                business: shop,
                role: Role::Labourer,
                wage: Money::new(1),
            }]
        );
    }

    #[test]
    fn wage_writeback_steers_only_the_next_matching() {
        let mut world = World::new();
        let house = world.add_house("Farm", vec![]);
        let farm_landlord = world.spawn_agent("landlord", None, Some(house));
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(35),
                headcount: 2,
                unfilled_ticks: 0,
            },
        );
        let farm = world
            .create_business(house, farm_landlord, Good::Food, Money::new(1), roles)
            .unwrap();
        world.accounts.mint(farm, Metal::Gold, Money::new(200));
        let first = world.spawn_agent("a", None, None);
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        // hired at the snapshot wage; the still-open second slot raises
        // AFTER matching (38 × 2 = 76 ≤ 200, affordable)
        assert_eq!(
            report.events,
            vec![
                Event::Hired {
                    agent: first,
                    business: farm,
                    role: Role::Labourer,
                    wage: Money::new(35),
                },
                Event::WageMoved {
                    business: farm,
                    role: Role::Labourer,
                    from: Money::new(35),
                    to: Money::new(38),
                },
            ]
        );
        // the raise is only visible to the NEXT decide
        let second = world.spawn_agent("b", None, None);
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        assert_eq!(
            report.events,
            vec![Event::Hired {
                agent: second,
                business: farm,
                role: Role::Labourer,
                wage: Money::new(38),
            }]
        );
    }

    #[test]
    fn settled_labor_market_emits_nothing() {
        // fully staffed, funded, nobody unemployed, no arrears: phase 1
        // is silent — no held-wage or no-op events
        let mut world = World::new();
        let (_, farm, _) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        world.accounts.mint(farm, Metal::Gold, Money::new(100));
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        assert_eq!(report.events, vec![]);
    }

    // --- Pack-4 migration tests: the town gains and loses people ---

    #[test]
    fn departed_narrates_name_and_swept_metals() {
        let mut world = World::new();
        // a Food seller posts the destitution reference price
        staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(2),
            Money::new(35),
            "f",
        );
        let leaver = world.spawn_agent("petra", None, None);
        world.accounts.mint(leaver, Metal::Gold, Money::new(1)); // below the cheapest Food
        world.accounts.mint(leaver, Metal::Silver, Money::new(3));
        world.accounts.mint(leaver, Metal::Copper, Money::new(5));
        world.agent_mut(leaver).unwrap().hunger = DEPART_HUNGER_TICKS;
        let mut report = TickReport::default();
        sinks(&mut world, &mut report);
        assert_eq!(
            report.events,
            vec![Event::Departed {
                agent: leaver,
                name: "petra".to_string(),
                took: vec![
                    (Metal::Gold, Money::new(1)),
                    (Metal::Silver, Money::new(3)),
                    (Metal::Copper, Money::new(5)),
                ],
            }]
        );
        assert!(world.agent(leaver).is_none());
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Silver),
            Money::new(3)
        );
        world.accounts.audit();
    }

    #[test]
    fn owner_emigration_forces_liquidation_no_orphans_on_either_id() {
        // Amendment 19's path, unreachable in the shipped town (its owners
        // are employed and solvent), so it is proven here or it ships
        // untested. The leaver is simultaneously the owner AND a creditor
        // of their own firm — the underdetermined-flow case the
        // ClosureReceipt exists for: their wallet delta is settlement
        // PLUS proceeds, and no delta taken around the whole command can
        // separate the two.
        let mut world = World::new();
        // A surviving Food seller, so phase 7 has a price to judge
        // destitution against and someone to owe the leaver money.
        let (_bakery_house, bakery, _baker) = landlord_owner_business(
            &mut world,
            "Bakery",
            Good::Food,
            Money::new(5),
            Money::new(10),
            1,
            "baker",
        );
        // The leaver owns the mill and works there — the shipped
        // owner-operator shape.
        let mill_house = world.add_house("Mill", vec![]);
        let leaver = world.spawn_agent("mira", None, Some(mill_house));
        world.agent_mut(leaver).unwrap().employed_role = Some(Role::Labourer);
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(20),
                headcount: 2,
                unfilled_ticks: 0,
            },
        );
        let mill = world
            .create_business(mill_house, leaver, Good::Luxury, Money::new(4), roles)
            .expect("fresh house, spawned owner");
        let hand = hire_at(&mut world, mill_house, "hand");
        world.accounts.mint(mill, Metal::Gold, Money::new(50));
        world.accounts.mint(mill, Metal::Silver, Money::new(6));
        {
            let ledger = &mut world
                .house_mut(mill_house)
                .unwrap()
                .business
                .as_mut()
                .unwrap()
                .owed_to;
            ledger.insert(leaver, Money::new(40));
            ledger.insert(hand, Money::new(30));
        }
        // ...and an unrelated firm owes the leaver too, so the pure
        // Amendment-17 path still runs alongside the liquidation.
        world.accounts.mint(bakery, Metal::Gold, Money::new(100));
        world
            .house_mut(_bakery_house)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .owed_to
            .insert(leaver, Money::new(25));
        world.agent_mut(leaver).unwrap().hunger = DEPART_HUNGER_TICKS;

        let external_before = world.accounts.balance_of(world.external_id, Metal::Gold);
        let mut report = TickReport::default();
        sinks(&mut world, &mut report);

        assert_eq!(
            report.events,
            vec![
                // step 0, from the receipt: creditors ascending — the
                // owner takes 40 of the 50g coffer, the hand the rest
                Event::Settled {
                    business: mill,
                    agent: leaver,
                    amount: Money::new(40),
                },
                Event::Settled {
                    business: mill,
                    agent: hand,
                    amount: Money::new(10),
                },
                Event::LaidOff {
                    agent: leaver,
                    business: mill,
                },
                Event::LaidOff {
                    agent: hand,
                    business: mill,
                },
                Event::Closed {
                    business: mill,
                    house: mill_house,
                    owner: leaver,
                    // the id resolves to nothing by emission time — this
                    // is why the event carries the name
                    owner_name: "mira".to_string(),
                    proceeds: vec![
                        (Metal::Gold, Money::ZERO), // creditors took it all
                        (Metal::Silver, Money::new(6)),
                        (Metal::Copper, Money::ZERO),
                    ],
                },
                // then Amendment 17, for the firm the leaver does NOT own
                Event::Settled {
                    business: bakery,
                    agent: leaver,
                    amount: Money::new(25),
                },
                // 40 own-arrears + 25 A17 + 6s of liquidation proceeds
                Event::Departed {
                    agent: leaver,
                    name: "mira".to_string(),
                    took: vec![
                        (Metal::Gold, Money::new(65)),
                        (Metal::Silver, Money::new(6)),
                        (Metal::Copper, Money::ZERO),
                    ],
                },
            ],
            "forced liquidation narrates in causal order, from the receipts"
        );
        // No BOGUS A17 Settled for the mill: without the owned-firm
        // exclusion the snapshot would report its whole 50g coffer delta
        // as one settlement to the leaver.
        assert_eq!(
            report
                .events
                .iter()
                .filter(|event| matches!(
                    event,
                    Event::Settled {
                        business, amount, ..
                    } if *business == mill && *amount == Money::new(50)
                ))
                .count(),
            0
        );
        // Both dead ids are empty on every metal — the per-account proof
        // the totals-only audit cannot make.
        for dead in [mill, leaver] {
            for metal in Metal::ALL {
                assert_eq!(
                    world.accounts.balance_of(dead, metal),
                    Money::ZERO,
                    "orphan balance parked on {dead:?}"
                );
            }
        }
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Gold),
            external_before.plus(Money::new(65))
        );
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Silver),
            Money::new(6)
        );
        // The hand keeps their partial settlement and their freedom.
        assert_eq!(world.accounts.balance_of(hand, Metal::Gold), Money::new(10));
        assert_eq!(world.agent(hand).unwrap().workplace, None);
        // The freed house is a landing pad.
        assert!(world.house(mill_house).unwrap().business.is_none());
        assert!(world.occupants_of(mill_house).is_empty());
        world.accounts.audit();
    }

    #[test]
    fn settlement_is_narrated_before_the_departure() {
        // The leaver must NOT own the firm, or this stops pinning
        // Amendment 17 and becomes a forced-liquidation test (which
        // `owner_emigration_forces_liquidation_no_orphans_on_either_id`
        // covers separately).
        let mut world = World::new();
        let (farm_house, farm, _owner) = landlord_owner_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(2),
            Money::new(35),
            1,
            "boss",
        );
        let worker = hire_at(&mut world, farm_house, "f");
        world.accounts.mint(farm, Metal::Gold, Money::new(20));
        world
            .house_mut(farm_house)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .owed_to
            .insert(worker, Money::new(50));
        world.agent_mut(worker).unwrap().hunger = DEPART_HUNGER_TICKS;
        let mut report = TickReport::default();
        sinks(&mut world, &mut report);
        // min(coffer 20, owed 50) settles and rides out with the sweep;
        // the 30 remainder is written off silently (Amendment 17)
        assert_eq!(
            report.events,
            vec![
                Event::Settled {
                    business: farm,
                    agent: worker,
                    amount: Money::new(20),
                },
                Event::Departed {
                    agent: worker,
                    name: "f".to_string(),
                    took: vec![
                        (Metal::Gold, Money::new(20)),
                        (Metal::Silver, Money::ZERO),
                        (Metal::Copper, Money::ZERO),
                    ],
                },
            ]
        );
        assert_eq!(world.accounts.balance_of(farm, Metal::Gold), Money::ZERO);
        assert!(
            world
                .house(farm_house)
                .unwrap()
                .business
                .as_ref()
                .unwrap()
                .owed_to
                .is_empty()
        );
        world.accounts.audit();
    }

    #[test]
    fn hungry_but_solvent_and_broke_but_fed_agents_stay() {
        let mut world = World::new();
        staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(2),
            Money::new(35),
            "f",
        );
        // hunger past the threshold, but they can still afford Food
        let solvent = world.spawn_agent("solvent", None, None);
        world.accounts.mint(solvent, Metal::Gold, Money::new(10));
        world.agent_mut(solvent).unwrap().hunger = DEPART_HUNGER_TICKS;
        // destitute, but not yet worn down
        let fed = world.spawn_agent("fed", None, None);
        world.agent_mut(fed).unwrap().hunger = DEPART_HUNGER_TICKS - 1;
        // the strict boundary: holding EXACTLY the cheapest price still
        // buys one unit — they stay (a <= regression would evict them)
        let boundary = world.spawn_agent("boundary", None, None);
        world.accounts.mint(boundary, Metal::Gold, Money::new(2));
        world.agent_mut(boundary).unwrap().hunger = DEPART_HUNGER_TICKS;
        let mut report = TickReport::default();
        sinks(&mut world, &mut report);
        assert_eq!(report.events, vec![]);
        assert!(world.agent(solvent).is_some());
        assert!(world.agent(fed).is_some());
        assert!(world.agent(boundary).is_some());
    }

    #[test]
    fn vacancy_age_counts_open_ticks_and_resets_on_fill() {
        let mut world = World::new();
        let (house, _) = open_slot_business(&mut world, "Farm", Good::Food, Money::new(35));
        let age = |world: &World| {
            world.house(house).unwrap().business.as_ref().unwrap().roles[&Role::Labourer]
                .unfilled_ticks
        };
        for expected in 1..=3u32 {
            labor_market(&mut world, &mut TickReport::default());
            assert_eq!(age(&world), expected);
        }
        // a hire fills the slot — the age resets with the same write-back
        world.spawn_agent("u", None, None);
        labor_market(&mut world, &mut TickReport::default());
        assert_eq!(age(&world), 0);
    }

    #[test]
    fn arrival_lands_takes_the_home_and_gets_the_stake() {
        let mut world = World::new();
        let (_, farm) = open_slot_business(&mut world, "Farm", Good::Food, Money::new(35));
        let cottage = world.add_house("5 Weir Cottage", vec![]);
        world
            .accounts
            .mint(world.external_id, Metal::Gold, Money::new(200));
        // age the slot to the pull threshold (nobody local to hire)
        for _ in 0..VACANCY_PULL_TICKS {
            labor_market(&mut world, &mut TickReport::default());
        }
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        let newcomer = world
            .agent_by_name("Mara")
            .expect("the table's first name")
            .id;
        assert_eq!(
            report.events,
            vec![Event::Arrived {
                agent: newcomer,
                name: "Mara".to_string(),
                home: cottage,
            }]
        );
        assert_eq!(world.agent(newcomer).unwrap().home, Some(cottage));
        assert_eq!(world.agent(newcomer).unwrap().workplace, None);
        assert_eq!(world.accounts.balance_of(newcomer, Metal::Gold), GRUBSTAKE);
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Gold),
            Money::new(200).minus(GRUBSTAKE)
        );
        world.accounts.audit();

        // and they join the applicant pool NEXT tick — hired at the very
        // slot whose vacancy pulled them
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        assert!(report.events.contains(&Event::Hired {
            agent: newcomer,
            business: farm,
            role: Role::Labourer,
            wage: Money::new(35),
        }));
    }

    #[test]
    fn arrivals_stall_on_drained_external_and_on_zero_vacancy() {
        // bound 1: the fund cannot stake — no arrival, however old the slot
        let mut world = World::new();
        open_slot_business(&mut world, "Farm", Good::Food, Money::new(35));
        world.add_house("5 Weir Cottage", vec![]);
        world.accounts.mint(
            world.external_id,
            Metal::Gold,
            GRUBSTAKE.minus(Money::new(1)),
        );
        for _ in 0..=VACANCY_PULL_TICKS {
            labor_market(&mut world, &mut TickReport::default());
        }
        assert!(world.agent_by_name("Mara").is_none());
        assert_eq!(world.arrivals, 0);
        // only the fixture's landlord — nobody arrived
        assert_eq!(world.agents.len(), 1);

        // bound 2: no vacant residence — the only spare hosts a business
        let mut world = World::new();
        open_slot_business(&mut world, "Farm", Good::Food, Money::new(35));
        world
            .accounts
            .mint(world.external_id, Metal::Gold, Money::new(500));
        for _ in 0..=VACANCY_PULL_TICKS {
            labor_market(&mut world, &mut TickReport::default());
        }
        assert!(world.agent_by_name("Mara").is_none());
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Gold),
            Money::new(500)
        );
    }

    #[test]
    fn stake_failure_leaves_a_valid_penniless_newcomer() {
        // the apply's §8.5 robustness, exercised directly: External
        // cannot cover the stake, the arrival still lands whole (the
        // open slot keeps the live labor-demand re-check satisfied)
        let mut world = World::new();
        open_slot_business(&mut world, "Farm", Good::Food, Money::new(35));
        let cottage = world.add_house("5 Weir Cottage", vec![]);
        let mut report = TickReport::default();
        apply_labor_intent(
            &mut world,
            Intent::Arrive {
                name: "Ivo".to_string(),
                home: cottage,
            },
            &mut HashMap::new(),
            &mut HashMap::new(),
            &mut report,
        );
        let newcomer = world.agent_by_name("Ivo").expect("arrived").id;
        for metal in Metal::ALL {
            assert_eq!(world.accounts.balance_of(newcomer, metal), Money::ZERO);
        }
        assert_eq!(world.agent(newcomer).unwrap().home, Some(cottage));
        assert!(matches!(report.events[..], [Event::Arrived { .. }]));
        world.accounts.audit();
    }

    #[test]
    fn local_applicants_beat_the_pull_and_arrivals_apply_last() {
        // one aged open slot, a local applicant racing the pull: the
        // hire applies first and the arrival dies on the live
        // labor-demand re-check — no immigrant, External untouched
        let mut world = World::new();
        let (house, farm) = open_slot_business(&mut world, "Farm", Good::Food, Money::new(35));
        world.add_house("5 Weir Cottage", vec![]);
        world
            .accounts
            .mint(world.external_id, Metal::Gold, Money::new(500));
        world
            .house_mut(house)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .roles
            .get_mut(&Role::Labourer)
            .unwrap()
            .unfilled_ticks = VACANCY_PULL_TICKS;
        let local = world.spawn_agent("local", None, None);
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        assert_eq!(
            report.events,
            vec![Event::Hired {
                agent: local,
                business: farm,
                role: Role::Labourer,
                wage: Money::new(35),
            }]
        );
        assert_eq!(world.arrivals, 0);
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Gold),
            Money::new(500)
        );

        // with TWO open slots the demand survives the local's hire, and
        // the arrival applies after every hire — the phase-1 order pin
        let mut world = World::new();
        let house = world.add_house("Farm", vec![]);
        let farm_landlord = world.spawn_agent("landlord", None, Some(house));
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(35),
                headcount: 2,
                unfilled_ticks: VACANCY_PULL_TICKS,
            },
        );
        let farm = world
            .create_business(house, farm_landlord, Good::Food, Money::new(1), roles)
            .unwrap();
        let cottage = world.add_house("5 Weir Cottage", vec![]);
        world
            .accounts
            .mint(world.external_id, Metal::Gold, Money::new(500));
        let local = world.spawn_agent("local", None, None);
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        let newcomer = world.agent_by_name("Mara").expect("pulled").id;
        assert_eq!(
            report.events,
            vec![
                Event::Hired {
                    agent: local,
                    business: farm,
                    role: Role::Labourer,
                    wage: Money::new(35),
                },
                Event::Arrived {
                    agent: newcomer,
                    name: "Mara".to_string(),
                    home: cottage,
                },
            ]
        );
    }

    #[test]
    fn arrive_pull_skips_arrears_carrying_venues() {
        // The pack-4 handoff, resolved by rule: an aged slot at a venue
        // that owes back wages pulls nobody. Same slot, ledger clear —
        // it pulls. The recorded deadbeat-recruitment bug, closed.
        // Two slots, one filled: the vacancy ages while the venue owes
        // its sitting worker. Everyone in this world is employed, so no
        // local applicant can beat the pull and mask the rule under test.
        let mut world = World::new();
        let (house, _venue, _owner) = landlord_owner_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(2),
            Money::new(35),
            2,
            "boss",
        );
        let worker = hire_at(&mut world, house, "w");
        world.add_house("5 Weir Cottage", vec![]);
        world
            .accounts
            .mint(world.external_id, Metal::Gold, Money::new(500));
        world
            .house_mut(house)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .owed_to
            .insert(worker, Money::new(1)); // one coin is enough

        for _ in 0..=VACANCY_PULL_TICKS {
            let mut report = TickReport::default();
            labor_market(&mut world, &mut report);
            assert!(
                !report
                    .events
                    .iter()
                    .any(|event| matches!(event, Event::Arrived { .. })),
                "a venue owing back wages must not pull a newcomer"
            );
        }
        // The slot HAS aged — it is the arrears, not the age, refusing.
        assert!(
            world.house(house).unwrap().business.as_ref().unwrap().roles[&Role::Labourer]
                .unfilled_ticks
                >= VACANCY_PULL_TICKS
        );
        // Clear the ledger and the very same slot pulls.
        world
            .house_mut(house)
            .unwrap()
            .business
            .as_mut()
            .unwrap()
            .owed_to
            .clear();
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        assert!(
            report
                .events
                .iter()
                .any(|event| matches!(event, Event::Arrived { .. })),
            "the same aged slot must pull once the venue owes nothing"
        );
        world.accounts.audit();
    }

    #[test]
    fn departed_workers_slot_ages_into_a_pull() {
        // the full chain at unit granularity: a worker departs, their
        // slot opens and ages, the pull answers, the newcomer takes the
        // very job the leaver freed
        // The owner is the venue's inert landlord, NOT the leaver: were
        // the leaver the owner, forced liquidation would detach the venue
        // at the moment of departure and no slot would be left to age.
        let mut world = World::new();
        let (farm_house, farm, _owner) = landlord_owner_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(2),
            Money::new(35),
            1,
            "boss",
        );
        let worker = hire_at(&mut world, farm_house, "f");
        let cottage = world.add_house("5 Weir Cottage", vec![]);
        world
            .accounts
            .mint(world.external_id, Metal::Gold, Money::new(500));
        world.agent_mut(worker).unwrap().hunger = DEPART_HUNGER_TICKS;
        let mut report = TickReport::default();
        sinks(&mut world, &mut report);
        assert!(world.agent(worker).is_none());
        for _ in 0..VACANCY_PULL_TICKS {
            labor_market(&mut world, &mut TickReport::default());
        }
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        let newcomer = world.agent_by_name("Mara").expect("pulled").id;
        assert!(report.events.contains(&Event::Arrived {
            agent: newcomer,
            name: "Mara".to_string(),
            home: cottage,
        }));
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        assert!(report.events.contains(&Event::Hired {
            agent: newcomer,
            business: farm,
            role: Role::Labourer,
            wage: Money::new(35),
        }));
    }

    #[test]
    fn hunger_has_a_single_writer() {
        // every behavior phase EXCEPT consume runs; none may touch the
        // counter — the Depart rule's meaning depends on it
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(2),
            Money::new(35),
            "f",
        );
        world.accounts.mint(farm, Metal::Gold, Money::new(100));
        set_stock(&mut world, farm_house, 50);
        world.agent_mut(worker).unwrap().hunger = DEPART_HUNGER_TICKS - 2;
        let mut report = TickReport::default();
        labor_market(&mut world, &mut report);
        produce(&mut world, &mut report);
        pay_wages(&mut world, &mut report);
        goods_market(&mut world, &mut report);
        sinks(&mut world, &mut report);
        assert_eq!(world.agent(worker).unwrap().hunger, DEPART_HUNGER_TICKS - 2);
    }

    #[test]
    fn the_last_grubstake_spends_and_then_the_pull_stalls() {
        // the passing boundary of the drain bound: External holding
        // exactly one stake funds exactly one arrival, then dries
        let mut world = World::new();
        let house = world.add_house("Farm", vec![]);
        let farm_landlord = world.spawn_agent("landlord", None, Some(house));
        let mut roles = HashMap::new();
        roles.insert(
            Role::Labourer,
            RoleSlot {
                wage: Money::new(35),
                headcount: 2,
                unfilled_ticks: VACANCY_PULL_TICKS,
            },
        );
        world
            .create_business(house, farm_landlord, Good::Food, Money::new(1), roles)
            .unwrap();
        world.add_house("5 Weir Cottage", vec![]);
        world.add_house("6 Weir Cottage", vec![]);
        world
            .accounts
            .mint(world.external_id, Metal::Gold, GRUBSTAKE);
        labor_market(&mut world, &mut TickReport::default());
        assert_eq!(world.arrivals, 1);
        assert_eq!(
            world.accounts.balance_of(world.external_id, Metal::Gold),
            Money::ZERO
        );
        // the second slot keeps aging and a cottage stands vacant, but
        // the dry fund never stakes another arrival
        for _ in 0..(2 * VACANCY_PULL_TICKS) {
            labor_market(&mut world, &mut TickReport::default());
        }
        assert_eq!(world.arrivals, 1);
        world.accounts.audit();
    }

    /// Amendment 15's contract, as far as a test can pin it: a run that
    /// drops every report ends in exactly the state of one that keeps
    /// them. Both arms execute the same code, so what this actually pins
    /// is cross-instance determinism (no state-affecting iteration
    /// order); observer-effect regressions are caught by the per-phase
    /// state tests above.
    #[test]
    fn tick_report_is_pure_observation() {
        let (mut kept, _, _, _) = seeded_minimal_economy();
        let (mut dropped, _, _, _) = seeded_minimal_economy();
        let mut observed = 0;
        for _ in 0..3 {
            observed += tick(&mut kept).events.len();
            tick(&mut dropped);
        }
        assert!(observed > 0); // the live phases really do narrate
        assert_eq!(digest(&kept), digest(&dropped));
    }
}
