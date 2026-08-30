//! The fixed 9-phase tick (parent doc §6). A new mechanic lands INSIDE its
//! phase; adding or reordering phases requires amending the spec's phase
//! contract table. The conservation audit (§8.3) is unconditionally last.

use crate::agent::{Agent, AgentId};
use crate::goods::Good;
use crate::housing::HouseId;
use crate::market::{self, JobOffer, Offer};
use crate::metal::Metal;
use crate::money::Money;
use crate::role::Role;
use crate::world::World;
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
    /// Phase 7 apply (Amendment 17): a business settled what its coffer
    /// covered of a leaver's back wages, immediately before their sweep.
    /// The written-off remainder is silent bookkeeping — the preceding
    /// `PayrollShort`s already told that story.
    Settled {
        business: AgentId,
        agent: AgentId,
        amount: Money,
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
/// it makes the sim and worldgen's soak criterion name ONE symbol — a
/// soak that re-spelled this predicate would agree today and diverge
/// silently the first time it is tuned, which is a false green rather
/// than a failure.
///
/// Deliberately reads the `owed_total()` FIELD, never `PayrollShort`
/// events: a venue with no live staff accrues nothing and emits neither
/// `WagePaid` nor `PayrollShort` ever again, so an event-keyed trigger
/// would be permanently blind to exactly the zombie closure must kill
/// (measured on a forced fixture, pack-2 probe).
pub(crate) fn insolvent_now(owed_total: Money) -> bool {
    owed_total > Money::ZERO
}

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
    let slot_aged = snapshot.businesses().any(|(_, business)| {
        Role::ALL.iter().any(|role| {
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
            .find(|house| house.business.is_none() && snapshot.occupants_of(house.id).is_empty());
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
            let still_hiring = world.businesses().any(|(house, business)| {
                Role::ALL.iter().any(|&role| {
                    business
                        .roles
                        .get(&role)
                        .is_some_and(|slot| slot.headcount > staff_in_role(world, house.id, role))
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
        Intent::Buy { .. } | Intent::Depart { .. } => {
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
        world
            .house_mut(house_id)
            .expect("snapshotted from businesses()")
            .business
            .as_mut()
            .expect("snapshotted from businesses()")
            .price = adjusted;
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
        | Intent::Depart { .. } => {
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
fn invest(world: &mut World, report: &mut TickReport) {
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
        // Pack-1 interim tolerance (spec, draw contract — retired by
        // pack 2's forced liquidation): an owner removed by emigration
        // before `remove_agent` knows about firms leaves a dangling id;
        // the draw skips cleanly, no transfer, no event.
        if world.agent(owner).is_none() {
            continue;
        }
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
    // Food market means no destitution test (no price to be below —
    // unreachable in shipped worlds); the guard scopes the DECIDE, not
    // the phase, so future phase-7 mechanics (demurrage, imports)
    // appended below stay unconditionally reached.
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
            let creditors: Vec<(AgentId, Money)> = world
                .businesses()
                .filter(|(_, business)| {
                    business.owed_to.get(&agent).copied().unwrap_or(Money::ZERO) > Money::ZERO
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
            if world.remove_agent(agent).is_err() {
                return; // existence checked above — dies cleanly regardless
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
        | Intent::Arrive { .. } => {
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

    #[test]
    fn draw_skips_a_dangling_owner_cleanly() {
        // The pack-1 interim rule (spec, draw contract): an owner removed
        // before pack 2's forced liquidation leaves a dangling id — the
        // draw skips, no transfer, no event, no panic. Retired when
        // remove_agent learns to liquidate.
        let mut world = World::new();
        let (_, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(1),
            Money::new(35),
            "f",
        );
        world.accounts.mint(farm, Metal::Gold, Money::new(150));
        world.remove_agent(worker).unwrap(); // the owner emigrates
        let mut report = TickReport::default();
        invest(&mut world, &mut report);
        assert_eq!(report.events, vec![]);
        assert_eq!(
            world.accounts.balance_of(farm, Metal::Gold),
            Money::new(150)
        );
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
    fn settlement_is_narrated_before_the_departure() {
        let mut world = World::new();
        let (farm_house, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(2),
            Money::new(35),
            "f",
        );
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
    fn departed_workers_slot_ages_into_a_pull() {
        // the full chain at unit granularity: a worker departs, their
        // slot opens and ages, the pull answers, the newcomer takes the
        // very job the leaver freed
        let mut world = World::new();
        let (_, farm, worker) = staffed_business(
            &mut world,
            "Farm",
            Good::Food,
            Money::new(2),
            Money::new(35),
            "f",
        );
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
