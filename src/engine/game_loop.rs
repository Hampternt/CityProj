//! The interactive shell: clear, render, read a command, tick. All
//! simulation behavior lives in `sim::tick` — this file only draws frames
//! and reads input. Loop mechanics are unchanged: Enter advances, q quits;
//! typing an agent's name inspects it.

use std::collections::HashMap;
use std::io::{self, Write};

use crate::agent::{Agent, AgentId};
use crate::business::RoleSlot;
use crate::goods::Good;
use crate::housing::HouseId;
use crate::metal::Metal;
use crate::money::Money;
use crate::role::Role;
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
    /// Anything else is taken as an agent name to inspect.
    Inspect(String),
}

/// Runs the shell until quit: draw a frame, read a command, act on it.
/// Starts from the hand-seeded [`template_world`].
pub fn run() {
    let mut world = template_world();
    let terrain = Terrain::generate(MAP_SEED, MAP_VERTICES, MAP_VERTICES, MAP_CELL_SIZE);
    let mut tick_count: u64 = 0;
    let mut last_report = TickReport::default();

    loop {
        // Redraw the frame in place so the display doesn't scroll downward.
        clear_screen();
        render(&world, tick_count, &last_report);

        match read_command(tick_count) {
            Command::Quit => break,
            Command::Advance => {
                last_report = sim::tick(&mut world);
                tick_count += 1;
            }
            Command::Inspect(name) => inspect(&world, &name),
            Command::Map => export_map(&terrain),
        }
    }
}

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
fn template_world() -> World {
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
    println!(
        "=== CityProj — tick {tick_count} · pop {population} · employed {employed} · unemployed {} ===",
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
        for event in &report.events {
            println!("  {}", render_event(world, event));
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
                "    sells {} @{} · stock {} · balance {} · owed {}",
                business.product,
                business.price,
                business.stock,
                compact_balances(world, business.id),
                business.owed_total(),
            );
        }
    }

    println!("agents:");
    for agent in &world.agents {
        println!(
            "  {} — balance {} · home {} · {}",
            agent.name,
            compact_balances(world, agent.id),
            describe_house(world, agent.home),
            describe_inventory(agent),
        );
    }
}

/// One feed line for one event. The match is exhaustive on purpose — a
/// new `Event` variant fails compilation here instead of silently missing
/// from the feed. All feed amounts are gold this milestone, rendered `35g`.
fn render_event(world: &World, event: &Event) -> String {
    match event {
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
        "[tick {tick_count}] Enter = advance · <agent name> = inspect · map = export map.json · q = quit > "
    );
    // stdout is line-buffered; flush so the prompt shows before we block.
    let _ = io::stdout().flush();

    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => Command::Quit,
        Ok(_) => match line.trim() {
            "" => Command::Advance,
            quit if quit.eq_ignore_ascii_case("q") => Command::Quit,
            // Shadows any agent literally named "map" — acceptable for a
            // debug command.
            map if map.eq_ignore_ascii_case("map") => Command::Map,
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
    print!("press Enter to continue... ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
}

/// Prints one agent's details, then waits for Enter so the next clear-screen
/// doesn't wipe them before they're read.
fn inspect(world: &World, name: &str) {
    match world.agent_by_name(name) {
        Some(agent) => {
            println!("{}:", agent.name);
            println!("  balance   {}", compact_balances(world, agent.id));
            println!("  home      {}", describe_house(world, agent.home));
            println!("  workplace {}", describe_house(world, agent.workplace));
            println!("  goods     {}", describe_inventory(agent));
        }
        None => println!("no agent named '{name}'"),
    }
    print!("press Enter to continue... ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
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
}
