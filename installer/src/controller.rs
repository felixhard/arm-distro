use anyhow::Result;

use std::sync::Arc;

use parking_lot::RwLock;

use crate::backend::Backend;
use crate::state::InstallerState;
use crate::ui::App;

pub fn run() -> Result<()> {
    let state = Arc::new(RwLock::new(InstallerState::default()));
    let backend = Backend::new(Arc::clone(&state));
    let app = App::new(state, backend)?;
    app.run()
}
