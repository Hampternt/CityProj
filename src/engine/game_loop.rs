//! The main game loop, called from `main`.

use std::collections::HashMap;
use std::io::{self, Write};

use crate::engine::game_state::GameState;

/// Runs the main game loop. Empty for now — just the loop scaffold.
pub fn run() {
    // Initialise the game instance once, before the loop starts.
    let game = GameState::new();

    // Basic warehouse: total quantity of each good produced so far. Persists
    // across ticks and only counts upward. `u64` because it accumulates.
    let mut storage: HashMap<String, u64> = HashMap::new();

    let mut tick: u64 = 0;
    let mut running = true;
    while running {
        // Redraw the frame in place so the menu doesn't scroll downward.
        clear_screen();
        render(&game, &storage, tick);

        // Wait for the user before advancing. Returns false once stdin
        // closes (EOF, e.g. Ctrl-D) or the user types "q".
        running = wait_for_step(tick);
        if !running {
            break;
        }

        // Advance one tick: every factory produces, output lands in storage.
        produce_into_storage(&game, &mut storage);

        tick += 1;
    }
}

/// Clears the terminal and parks the cursor at the top-left, so each frame
/// redraws in place instead of scrolling. `\x1b[2J` erases the screen and
/// `\x1b[H` homes the cursor; we flush so it lands before anything else prints.
fn clear_screen() {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();
}

/// Runs every factory for one tick and adds its output to `storage`.
///
/// Yield per factory is `output × count` (each copy produces the output).
/// `*entry.or_insert(0) += ...` is the idiomatic "increment a map counter,
/// starting from zero if the key is new" — the equivalent of
/// `storage[good] = (storage[good] ?? 0) + amount` in JS.
fn produce_into_storage(game: &GameState, storage: &mut HashMap<String, u64>) {
    for factory in &game.factories {
        for (good, &amount) in &factory.outputs {
            let produced = amount as u64 * factory.count as u64;
            *storage.entry(good.clone()).or_insert(0) += produced;
        }
    }
}

/// Draws one stable frame: the factories and what they make, then the running
/// storage totals. Called after `clear_screen`, so it always redraws in place.
fn render(game: &GameState, storage: &HashMap<String, u64>, tick: u64) {
    println!("=== CityProj — tick {tick} ===");

    println!("factories:");
    for factory in &game.factories {
        let mut outs: Vec<_> = factory.outputs.iter().collect();
        outs.sort_by(|a, b| a.0.cmp(b.0));
        let listed: Vec<String> = outs
            .iter()
            .map(|(good, qty)| format!("{good} x{qty}"))
            .collect();
        println!("  {} (x{}) -> {}", factory.name, factory.count, listed.join(", "));
    }

    let mut goods: Vec<_> = storage.iter().collect();
    goods.sort_by(|a, b| a.0.cmp(b.0));
    print!("storage:");
    if goods.is_empty() {
        print!(" (empty)");
    }
    for (good, qty) in goods {
        print!(" {good}={qty}");
    }
    println!();
}

/// Blocks until the user presses Enter, prompting for the next tick.
///
/// Returns `false` to stop the loop when stdin reaches EOF or the user
/// enters "q"; `true` otherwise (including a bare Enter).
fn wait_for_step(tick: u64) -> bool {
    print!("[tick {tick}] press Enter to advance (q + Enter to quit)... ");
    // stdout is line-buffered; flush so the prompt shows before we block.
    let _ = io::stdout().flush();

    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) => false,                          // EOF (e.g. Ctrl-D)
        Ok(_) => !line.trim().eq_ignore_ascii_case("q"),
        Err(_) => false,                         // treat read errors as stop
    }
}
