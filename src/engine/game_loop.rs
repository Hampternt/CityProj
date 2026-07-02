//! The interactive shell: clear, render, read a command, tick. All
//! simulation behavior lives in `sim::tick` — this file only draws frames
//! and reads input. Loop mechanics are unchanged: Enter advances, q quits;
//! typing an agent's name inspects it.

use std::io::{self, Write};

use crate::housing::HouseId;
use crate::sim;
use crate::world::World;

/// One parsed line of user input at the tick prompt.
enum Command {
    Advance,
    Quit,
    Inspect(String),
}

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

/// A hand-seeded template world to step through: three people, two houses.
/// Nothing mints money yet, so every balance stays 0 until a faucet exists
/// (the mint job — money only ever enters through earned paths, no genesis).
fn template_world() -> World {
    let mut world = World::new();
    let alice = world.spawn_agent("alice", None, None);
    let bob = world.spawn_agent("bob", None, None);
    let carol = world.spawn_agent("carol", None, None);
    let mill = world.add_house("1 Mill Lane", vec![alice]);
    let kiln = world.add_house("2 Kiln Row", vec![bob]);
    world.agent_mut(alice).expect("just spawned").home = Some(mill);
    world.agent_mut(bob).expect("just spawned").home = Some(kiln);
    world.agent_mut(carol).expect("just spawned").home = Some(kiln);
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
    }

    println!("agents:");
    for agent in &world.agents {
        println!(
            "  {} — balance {} · home {}",
            agent.name,
            world.accounts.balance_of(agent.id),
            describe_house(world, agent.home),
        );
    }
}

/// Resolves a list of agent ids to their names (unknown ids are skipped).
fn names_of(world: &World, ids: &[crate::agent::AgentId]) -> Vec<String> {
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
        }
        None => println!("no agent named '{name}'"),
    }
    print!("press Enter to continue... ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    let _ = io::stdin().read_line(&mut line);
}
