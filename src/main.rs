#![warn(clippy::pedantic)]

pub mod alert;
pub mod app;
pub mod cli;
pub mod constants;
pub mod event;
pub mod log;
pub mod opensnitch_json;
pub mod opensnitch_proto;
pub mod operator_util;
pub mod serde_impl;
pub mod server;
pub mod ui;

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;

/// Main.
/// # Errors
/// Returns an error if there was bad input at init.
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let matches = cli::setup().get_matches();

    // Initialize logging if --logfile is provided
    let logfile = matches.get_one::<String>("logfile").cloned();
    match log::init(logfile.clone()) {
        Ok(true) => eprintln!("Logging to: {}", logfile.unwrap()),
        Ok(false) => {} // No logfile specified, that's fine
        Err(e) => {
            eprintln!("ERROR: {}", e);
            eprintln!("Continuing without logging...");
            std::thread::sleep(std::time::Duration::from_secs(3));
        }
    }
    log::info("opensnitch-tui starting");

    color_eyre::install()?;
    let terminal = ratatui::init();
    execute!(std::io::stdout(), EnableMouseCapture)?;

    let bind_addr = matches.get_one::<String>("ip_port").unwrap();
    log::info(&format!("Bind address: {}", bind_addr));

    let app = app::App::new(
        bind_addr,
        matches.get_one::<String>("default_action").unwrap(),
        matches.get_one::<String>("temp_rule_lifetime").unwrap(),
        matches.get_one::<u64>("dispo_seconds").unwrap(),
    )
    .expect("Initialization failed: ");

    log::info("App initialized, starting run loop");
    let result = app.run(terminal).await;
    let _ = execute!(std::io::stdout(), DisableMouseCapture);
    ratatui::restore();
    log::info("opensnitch-tui exiting");
    result
}
