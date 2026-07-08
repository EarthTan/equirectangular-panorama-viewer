mod app;
mod error;
mod file;
mod input;
mod loader;
mod renderer;
mod scene;
mod ui;
mod window;

use app::App;
use winit::event_loop::EventLoop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log::info!("pano-viewer starting");

    let cli_path = std::env::args().nth(1).map(std::path::PathBuf::from);

    let event_loop = EventLoop::new()?;
    let mut app = App::new()?;
    if let Some(p) = cli_path {
        app.set_pending_load(p);
    }
    event_loop.run_app(&mut app)?;

    Ok(())
}
