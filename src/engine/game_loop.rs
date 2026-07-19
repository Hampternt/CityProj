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
use crate::money::Money;
use crate::role::Role;
use crate::sim;
use crate::world::World;

/// One parsed line of user input at the tick prompt.
enum Command {
    /// Empty line: advance the simulation one tick.
    Advance,
    /// `q` (any case), EOF, or a read error: leave the loop.
    Quit,
    /// Anything else is taken as an agent name to inspect.
    Inspect(String),
}

/// Runs the shell until quit: draw a frame, read a command, act on it.
/// Starts from the hand-seeded [`template_world`].
pub fn run() {
    let mut world = template_world();
    let mut tick_count: u64 = 0;

    loop {
        // Redraw the frame in place so the display doesn't scroll downward.
        clear_screen();
        render(&world, tick_count);

        match read_command(tick_count) {
            Command::Quit => break,
            Command::Advance => {
                sim::tick(&mut world);
                tick_count += 1;
            }
            Command::Inspect(name) => inspect(&world, &name),
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
/// counts it (§8.4).
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
        world.accounts.mint(business, bill);
        let worker = world.spawn_agent(worker_name, Some(residence), Some(house));
        world.agent_mut(worker).expect("just spawned").employed_role = Some(Role::Labourer);
    }
    world.spawn_agent("dave", Some(residence), None); // unemployed, housed

    let everyone: Vec<AgentId> = world.agents.iter().map(|agent| agent.id).collect();
    for id in everyone {
        world.accounts.mint(id, Money::new(35));
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

/// Draws one stable frame: the money summary, then houses, then agents.
fn render(world: &World, tick_count: u64) {
    println!("=== CityProj — tick {tick_count} ===");
    println!(
        "money: total={} minted={} burned={}",
        world.accounts.total_money(),
        world.accounts.total_minted(),
        world.accounts.total_burned(),
    );
    println!(
        "reserved: mint balance {} · external balance {}",
        world.accounts.balance_of(world.mint_id),
        world.accounts.balance_of(world.external_id),
    );

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
                world.accounts.balance_of(business.id),
                business.owed_total(),
            );
        }
    }

    println!("agents:");
    for agent in &world.agents {
        println!(
            "  {} — balance {} · home {} · {}",
            agent.name,
            world.accounts.balance_of(agent.id),
            describe_house(world, agent.home),
            describe_inventory(agent),
        );
    }
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
    print!("[tick {tick_count}] Enter = advance · <agent name> = inspect · q = quit > ");
    // stdout is line-buffered; flush so the prompt shows before we block.
    let _ = io::stdout().flush();

    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => Command::Quit,
        Ok(_) => match line.trim() {
            "" => Command::Advance,
            quit if quit.eq_ignore_ascii_case("q") => Command::Quit,
            name => Command::Inspect(name.to_string()),
        },
    }
}

/// Prints one agent's details, then waits for Enter so the next clear-screen
/// doesn't wipe them before they're read.
fn inspect(world: &World, name: &str) {
    match world.agent_by_name(name) {
        Some(agent) => {
            println!("{}:", agent.name);
            println!("  balance   {}", world.accounts.balance_of(agent.id));
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
