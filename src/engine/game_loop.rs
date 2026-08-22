//! The interactive shell: clear, render, read a command, tick. All
//! simulation behavior lives in `sim::tick` — this file only draws frames
//! and reads input. Enter advances, q quits; `roster` lists every agent;
//! typing an agent's name (or a business's house address) inspects it.

use std::collections::HashMap;
use std::io::{self, Write};

use super::worldgen::town_world;
use crate::agent::{Agent, AgentId};
use crate::goods::Good;
use crate::housing::HouseId;
use crate::metal::Metal;
use crate::money::Money;
use crate::sim::{self, Event, TickReport};
use crate::terrain::Terrain;
use crate::world::World;

/// The shell's display terrain: generated once at startup, held alongside
/// — not inside — the economy `World` (no sim consumer yet). Fixed seed
/// so every run shows the same land. 64×64 vertices at cell 50 is a
/// ~320 m square sampled every 5 m (spec, `generate` contract).
const MAP_SEED: u64 = 20260728;
const MAP_VERTICES: u32 = 64;
const MAP_CELL_SIZE: i64 = 50;

/// One parsed line of user input at the tick prompt.
enum Command {
    /// Empty line: advance the simulation one tick.
    Advance,
    /// `q` (any case), EOF, or a read error: leave the loop.
    Quit,
    /// `map` (any case): export the terrain to map.json.
    Map,
    /// `roster` (any case): list every agent on one line each.
    Roster,
    /// Anything else is taken as an agent name (or business address) to
    /// inspect.
    Inspect(String),
}

/// Runs the shell until quit: draw a frame, read a command, act on it.
/// Starts from the hand-seeded [`town_world`]
/// (`worldgen::template_world` stays the small test fixture).
pub fn run() {
    let mut world = town_world();
    let terrain = Terrain::generate(MAP_SEED, MAP_VERTICES, MAP_VERTICES, MAP_CELL_SIZE);
    let mut tick_count: u64 = 0;
    let mut last_report = TickReport::default();
    // Shell-side memory of the feed: the last 3 lines each agent starred
    // in (manifest decision — presentation state, never world state).
    let mut history: HashMap<AgentId, Vec<String>> = HashMap::new();

    loop {
        // Redraw the frame in place so the display doesn't scroll downward.
        clear_screen();
        render(&world, tick_count, &last_report);

        match read_command(tick_count) {
            Command::Quit => break,
            Command::Advance => {
                let report = sim::tick(&mut world);
                update_history(&mut history, &world, &report);
                last_report = report;
                tick_count += 1;
            }
            Command::Roster => roster(&world),
            Command::Inspect(name) => inspect(&world, &history, &name),
            Command::Map => export_map(&terrain),
        }
    }
}

/// Folds one tick's events into each starring agent's last-3 buffer.
/// Business-only events (production, price moves) star nobody.
fn update_history(history: &mut HashMap<AgentId, Vec<String>>, world: &World, report: &TickReport) {
    for event in &report.events {
        let starring = match event {
            Event::Hired { agent, .. } | Event::Quit { agent, .. } => Some(*agent),
            Event::Arrived { agent, .. } => Some(*agent),
            Event::WagePaid { worker, .. } | Event::PayrollShort { worker, .. } => Some(*worker),
            Event::Sold { buyer, .. } => Some(*buyer),
            Event::WentHungry { agent } => Some(*agent),
            // profit is the owner's story
            Event::ProfitDrawn { owner, .. } => Some(*owner),
            // the leaver's id resolves to nothing once they are gone
            Event::Produced { .. }
            | Event::PriceMoved { .. }
            | Event::WageMoved { .. }
            | Event::Settled { .. }
            | Event::Departed { .. } => None,
        };
        if let Some(id) = starring {
            let lines = history.entry(id).or_default();
            if lines.len() == 3 {
                lines.remove(0);
            }
            lines.push(render_event(world, event));
        }
        // a leaver's buffer is unreachable once they're gone — drop it
        // so churny long runs don't leak dead entries
        if let Event::Departed { agent, .. } = event {
            history.remove(agent);
        }
    }
}

/// Clears the terminal and parks the cursor at the top-left, so each frame
/// redraws in place instead of scrolling. `\x1b[2J` erases the screen and
/// `\x1b[H` homes the cursor; we flush so it lands before anything prints.
fn clear_screen() {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();
}

/// Draws one stable frame: town header, money summary, the last tick's
/// event feed, then the houses/agents ledger.
fn render(world: &World, tick_count: u64, report: &TickReport) {
    let population = world.agents.len();
    let employed = world
        .agents
        .iter()
        .filter(|agent| agent.workplace.is_some())
        .count();
    let businesses = world.businesses().count();
    println!(
        "=== CityProj — tick {tick_count} · pop {population} · employed {employed} · unemployed {} · businesses {businesses} ===",
        population - employed
    );
    // D2 (pack 2): one line per metal, `Metal::ALL` order. A cross-metal
    // total does not exist — the core refuses to compute one.
    println!("money:");
    for metal in Metal::ALL {
        println!(
            "  {:<6} total={} minted={} burned={}",
            metal.to_string(),
            world.accounts.total_money(metal),
            world.accounts.total_minted(metal),
            world.accounts.total_burned(metal),
        );
    }
    println!(
        "reserved: mint {} · external {}",
        compact_balances(world, world.mint_id),
        compact_balances(world, world.external_id),
    );

    println!("last tick:");
    if report.events.is_empty() {
        if tick_count == 0 {
            println!("  (nothing yet — Enter advances the first tick)");
        } else {
            println!("  (a quiet tick)");
        }
    } else {
        for line in render_feed(world, report) {
            println!("  {line}");
        }
    }

    println!("houses:");
    for house in &world.houses {
        let owners = names_of(world, &house.owners);
        let occupants = names_of(world, &world.occupants_of(house.id));
        println!(
            "  {} — owners: {} · occupants: {}",
            house.address,
            or_none(&owners),
            or_none(&occupants),
        );
        if let Some(business) = &house.business {
            println!(
                "    sells {} @{} · owner {} · stock {} · balance {} · owed {}",
                business.product,
                business.price,
                agent_name(world, business.owner),
                business.stock,
                compact_balances(world, business.id),
                business.owed_total(),
            );
        }
    }

    // At town scale the per-agent ledger outgrew the frame — `roster`
    // carries the full list, a name inspects one (pack 2 readability).
    println!("agents: {population} — `roster` lists them · a name inspects");
}

/// The feed at town scale (pack 2): the routine aggregates to one line
/// per business — wages and sales — while the exceptional (shortfalls,
/// price moves, hunger) stays individual. Category order follows phase
/// order; within a category, first-seen order (which IS phase iteration
/// order). Presentation only: events stay granular for tests and the
/// inspect buffer.
fn render_feed(world: &World, report: &TickReport) -> Vec<String> {
    // (business, workers paid, total gold)
    let mut wages: Vec<(AgentId, u32, Money)> = Vec::new();
    // (business, good, snapshot price, units, buyers)
    let mut sales: Vec<(AgentId, Good, Money, u32, u32)> = Vec::new();
    // Labor events stay individual — each is notable — and arrive
    // already in phase-internal order (quits, hires, wage moves).
    let mut labor = Vec::new();
    let mut produced = Vec::new();
    let mut shorts = Vec::new();
    let mut moves = Vec::new();
    let mut hungry = Vec::new();
    // Phase-6 draws land between hunger and the leavers, per phase order.
    let mut draws = Vec::new();
    // Phase-7 departures close the feed, in event order (settlements
    // immediately before their leaver).
    let mut leavers = Vec::new();
    for event in &report.events {
        match event {
            Event::Hired { .. }
            | Event::Quit { .. }
            | Event::WageMoved { .. }
            | Event::Arrived { .. } => labor.push(render_event(world, event)),
            Event::Settled { .. } | Event::Departed { .. } => {
                leavers.push(render_event(world, event))
            }
            Event::Produced { .. } => produced.push(render_event(world, event)),
            Event::WagePaid {
                business, amount, ..
            } => {
                if let Some(entry) = wages.iter_mut().find(|(id, ..)| *id == *business) {
                    entry.1 += 1;
                    entry.2 = entry.2.plus(*amount);
                } else {
                    wages.push((*business, 1, *amount));
                }
            }
            Event::PayrollShort { .. } => shorts.push(render_event(world, event)),
            Event::Sold {
                business,
                good,
                units,
                price,
                ..
            } => {
                if let Some(entry) = sales
                    .iter_mut()
                    .find(|(id, g, ..)| *id == *business && *g == *good)
                {
                    entry.3 += *units;
                    entry.4 += 1;
                } else {
                    sales.push((*business, *good, *price, *units, 1));
                }
            }
            Event::PriceMoved { .. } => moves.push(render_event(world, event)),
            Event::WentHungry { .. } => hungry.push(render_event(world, event)),
            Event::ProfitDrawn { .. } => draws.push(render_event(world, event)),
        }
    }
    let mut lines = labor;
    lines.extend(produced);
    for (business, workers, total) in wages {
        lines.push(format!(
            "{} paid {workers} worker{} {total}g",
            business_address(world, business),
            if workers == 1 { "" } else { "s" },
        ));
    }
    lines.extend(shorts);
    for (business, good, price, units, buyers) in sales {
        lines.push(format!(
            "{} sold {units} {good} to {buyers} buyer{} @ {price}g",
            business_address(world, business),
            if buyers == 1 { "" } else { "s" },
        ));
    }
    lines.extend(moves);
    lines.extend(hungry);
    lines.extend(draws);
    lines.extend(leavers);
    lines
}

/// One feed line for one event. The match is exhaustive on purpose — a
/// new `Event` variant fails compilation here instead of silently missing
/// from the feed. All feed amounts are gold this milestone, rendered `35g`.
fn render_event(world: &World, event: &Event) -> String {
    match event {
        Event::Quit {
            agent,
            business,
            owed,
        } => format!(
            "{} quit {} (still owed {owed}g)",
            agent_name(world, *agent),
            business_address(world, *business),
        ),
        Event::Hired {
            agent,
            business,
            role,
            wage,
        } => format!(
            "{} hired at {} as {role} @ {wage}g",
            agent_name(world, *agent),
            business_address(world, *business),
        ),
        Event::WageMoved {
            business,
            role,
            from,
            to,
        } => {
            let verb = if to > from { "raised" } else { "lowered" };
            format!(
                "{} {verb} {role} wages to {to}g",
                business_address(world, *business)
            )
        }
        Event::Produced {
            business,
            good,
            units,
        } => format!(
            "{} produced {units} {good}",
            business_address(world, *business)
        ),
        Event::WagePaid {
            business,
            worker,
            amount,
        } => format!(
            "{} paid {} {amount}g",
            business_address(world, *business),
            agent_name(world, *worker),
        ),
        Event::PayrollShort {
            business,
            worker,
            remaining,
        } => format!(
            "{} still owes {} {remaining}g in wages",
            business_address(world, *business),
            agent_name(world, *worker),
        ),
        Event::Sold {
            business,
            buyer,
            good,
            units,
            price,
        } => format!(
            "{} bought {units} {good} @ {price}g from {}",
            agent_name(world, *buyer),
            business_address(world, *business),
        ),
        Event::PriceMoved {
            business,
            good,
            from,
            to,
        } => {
            let verb = if to > from { "raised" } else { "lowered" };
            format!(
                "{} {verb} {good} to {to}g",
                business_address(world, *business)
            )
        }
        Event::WentHungry { agent } => {
            format!("{} went hungry", agent_name(world, *agent))
        }
        Event::ProfitDrawn {
            business,
            owner,
            amount,
        } => format!(
            "{} paid {} {amount}g profit",
            business_address(world, *business),
            agent_name(world, *owner),
        ),
        Event::Arrived { name, home, .. } => format!(
            "{name} arrived seeking work, settling at {}",
            world
                .house(*home)
                .map(|house| house.address.clone())
                .unwrap_or_else(|| "(unknown house)".to_string()),
        ),
        Event::Settled {
            business, amount, ..
        } => format!(
            "{} paid out {amount}g of back wages to a leaver",
            business_address(world, *business),
        ),
        Event::Departed { name, took, .. } => {
            let holdings: Vec<String> = took
                .iter()
                .filter(|(_, amount)| *amount > crate::money::Money::ZERO)
                .map(|(metal, amount)| format!("{amount}{}", metal_tag(*metal)))
                .collect();
            if holdings.is_empty() {
                format!("{name} left town penniless")
            } else {
                format!("{name} left town (took {})", holdings.join(" "))
            }
        }
    }
}

/// A business has no name of its own — it renders as its house's address.
fn business_address(world: &World, id: AgentId) -> String {
    world
        .businesses()
        .find(|(_, business)| business.id == id)
        .map(|(house, _)| house.address.clone())
        .unwrap_or_else(|| "(unknown business)".to_string())
}

fn agent_name(world: &World, id: AgentId) -> String {
    world
        .agent(id)
        .map(|agent| agent.name.clone())
        .unwrap_or_else(|| "(unknown agent)".to_string())
}

/// One-letter tag for the compact balance form (D3). A match, so a new
/// metal fails compilation here instead of silently missing a column.
fn metal_tag(metal: Metal) -> &'static str {
    match metal {
        Metal::Gold => "g",
        Metal::Silver => "s",
        Metal::Copper => "c",
    }
}

/// D3 (pack 2): one account's balances, every metal always shown, compact:
/// `g:35 s:10 c:20` — visible zeros beat columns that come and go.
fn compact_balances(world: &World, id: AgentId) -> String {
    Metal::ALL
        .iter()
        .map(|&metal| {
            format!(
                "{}:{}",
                metal_tag(metal),
                world.accounts.balance_of(id, metal)
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Resolves a list of agent ids to their names (unknown ids are skipped).
fn names_of(world: &World, ids: &[AgentId]) -> Vec<String> {
    ids.iter()
        .filter_map(|id| world.agent(*id))
        .map(|agent| agent.name.clone())
        .collect()
}

fn or_none(names: &[String]) -> String {
    if names.is_empty() {
        "(none)".to_string()
    } else {
        names.join(", ")
    }
}

fn describe_house(world: &World, id: Option<HouseId>) -> String {
    id.and_then(|house_id| world.house(house_id))
        .map(|house| house.address.clone())
        .unwrap_or_else(|| "none".to_string())
}

/// One line of pantry: `food 10 · entertainment 5 · luxury 2`.
fn describe_inventory(agent: &Agent) -> String {
    Good::ALL
        .iter()
        .map(|good| {
            let held = agent.inventory.get(good).copied().unwrap_or(0);
            format!("{good} {held}")
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Blocks until the user enters a command. EOF (e.g. Ctrl-D) and read
/// errors quit cleanly, same as before.
fn read_command(tick_count: u64) -> Command {
    print!(
        "[tick {tick_count}] Enter = advance · roster · <name> = inspect · map = export map.json · q = quit > "
    );
    // stdout is line-buffered; flush so the prompt shows before we block.
    let _ = io::stdout().flush();

    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => Command::Quit,
        Ok(_) => match line.trim() {
            "" => Command::Advance,
            quit if quit.eq_ignore_ascii_case("q") => Command::Quit,
            // Keywords shadow any agent literally so named — acceptable
            // for shell commands.
            map if map.eq_ignore_ascii_case("map") => Command::Map,
            roster if roster.eq_ignore_ascii_case("roster") => Command::Roster,
            name => Command::Inspect(name.to_string()),
        },
    }
}

/// Writes the terrain to map.json in the working directory. A write
/// failure prints an error and the sim continues (spec, map export
/// contract).
fn export_map(terrain: &Terrain) {
    match std::fs::write("map.json", terrain.to_json()) {
        Ok(()) => println!("wrote map.json — open tools/map_viewer.html and load it"),
        Err(error) => println!("could not write map.json: {error}"),
    }
    wait_for_enter();
}

/// Parks the output until Enter so the next clear-screen doesn't wipe it
/// before it can be read.
fn wait_for_enter() {
    print!("press Enter to continue... ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
}

/// One line per agent: job, employer, gold, pantry — and, for owners,
/// the venues they own (firm-lifecycle pack 1).
fn roster(world: &World) {
    println!("roster:");
    for agent in &world.agents {
        let job = match (agent.employed_role, agent.workplace) {
            (Some(role), workplace @ Some(_)) => {
                format!("{role} at {}", describe_house(world, workplace))
            }
            (None, workplace @ Some(_)) => {
                format!("at {} (no role)", describe_house(world, workplace))
            }
            _ => "unemployed".to_string(),
        };
        let owned: Vec<String> = world
            .businesses()
            .filter(|(_, business)| business.owner == agent.id)
            .map(|(house, _)| house.address.clone())
            .collect();
        let ownership = if owned.is_empty() {
            String::new()
        } else {
            format!(" · owns {}", owned.join(", "))
        };
        println!(
            "  {} — {job}{ownership} · gold {} · {}",
            agent.name,
            world.accounts.balance_of(agent.id, Metal::Gold),
            describe_inventory(agent),
        );
    }
    wait_for_enter();
}

/// Prints one agent's details (with their last 3 feed lines) or, when the
/// name matches no agent, a business by its house address; then waits for
/// Enter so the next clear-screen doesn't wipe it before it's read.
fn inspect(world: &World, history: &HashMap<AgentId, Vec<String>>, name: &str) {
    if let Some(agent) = world.agent_by_name(name) {
        println!("{}:", agent.name);
        println!("  balance   {}", compact_balances(world, agent.id));
        println!("  home      {}", describe_house(world, agent.home));
        println!("  workplace {}", describe_house(world, agent.workplace));
        println!("  goods     {}", describe_inventory(agent));
        if agent.hunger > 0 {
            println!("  hungry    {} tick(s) without enough food", agent.hunger);
        }
        println!("  recent:");
        match history.get(&agent.id) {
            Some(lines) if !lines.is_empty() => {
                for line in lines {
                    println!("    {line}");
                }
            }
            _ => println!("    (nothing yet)"),
        }
    } else if let Some((house_id, business_id)) = world
        .businesses()
        .find(|(house, _)| house.address.eq_ignore_ascii_case(name))
        .map(|(house, business)| (house.id, business.id))
    {
        let house = world.house(house_id).expect("found above");
        let business = house.business.as_ref().expect("found above");
        println!("{} (business):", house.address);
        println!("  sells   {} @{}g", business.product, business.price);
        println!("  owner   {}", agent_name(world, business.owner));
        println!("  stock   {}", business.stock);
        println!("  coffers {}", compact_balances(world, business_id));
        if business.owed_to.is_empty() {
            println!("  owes    (nothing)");
        } else {
            // owed_to is a HashMap — sort the lines for a stable display.
            let mut debts: Vec<String> = business
                .owed_to
                .iter()
                .map(|(worker, amount)| format!("{} {amount}g", agent_name(world, *worker)))
                .collect();
            debts.sort();
            println!("  owes    {}", debts.join(" · "));
        }
        let workers = names_of(world, &world.employees_of(house_id));
        println!("  workers {}", or_none(&workers));
    } else {
        println!("no agent or business named '{name}'");
    }
    wait_for_enter();
}
