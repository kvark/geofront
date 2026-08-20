//! Geofront – mecha tactical city defence and management.

mod base;
mod characters;
mod combat;
mod render;
mod ui;
mod units;
mod world;

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

    // TODO: Blade engine + winit window setup (mirroring redline)
    // For now, just a placeholder that compiles on both targets.
    println!("Geofront scaffold ready. Implement engine loop next.");
}
