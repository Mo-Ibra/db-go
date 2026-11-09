use std::sync::{Arc, Mutex};

use db_go_core::Database;

#[derive(Default)]
pub struct AppState {
    pub db: Arc<Mutex<Option<Database>>>,
}
