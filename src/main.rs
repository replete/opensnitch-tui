use crate::app::App;

pub mod alert;
pub mod app;
pub mod constants;
pub mod event;
pub mod opensnitch_json;
pub mod opensnitch_proto;
pub mod operator_util;
pub mod serde_impl;
pub mod server;
pub mod ui;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    // abtodo some CLI flags for bind addr/port, default actions, etc.
    let app = App::new(
        String::from("127.0.0.1:50051"),
        String::from("deny"),
        String::from("12h"),
    )
    .expect("Initialization");
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}
