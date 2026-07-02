mod agent;
mod engine;
mod housing;
mod money;
mod sim;
mod world;

fn main() {
    engine::game_loop::run();
}
