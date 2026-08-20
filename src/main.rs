//! Geofront – mecha tactical city defence and management.

mod base;
mod characters;
mod combat;
mod render;
mod ui;
mod units;
mod world;

use combat::Mission;
use log::info;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        console_log::init_with_level(log::Level::Info).expect("logger");
    }

    info!("Geofront starting");

    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--smoke") {
        run_smoke();
        return;
    }

    // TODO: Blade engine + winit window setup (mirroring redline).
    // For now the combat core is usable via --smoke and unit tests later.
    println!("Geofront scaffold + combat MVP ready.");
    println!("  cargo run -- --smoke     # headless short mission");
    println!("Next: Blade engine loop + egui HUD.");
}

fn run_smoke() {
    info!("Running combat smoke test");
    let mut mission = Mission::new_skirmish();
    mission.smoke_run(6);
    for line in &mission.log {
        println!("{line}");
    }
    if mission.is_won() {
        info!("Smoke finished: WIN");
    } else if mission.is_lost() {
        info!("Smoke finished: LOSS");
    } else {
        info!("Smoke finished: ongoing (turn {})", mission.turn);
    }
}
