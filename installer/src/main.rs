mod backend;
mod controller;
mod logging;
mod state;
mod ui;

use anyhow::Result;

fn main() -> Result<()> {
    logging::init()?;
    controller::run()
}
