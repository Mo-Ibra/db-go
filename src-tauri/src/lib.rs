mod commands;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::initial_database,
            commands::open_database,
            commands::create_database,
            commands::create_table,
            commands::close_database,
            commands::list_tables,
            commands::describe_table,
            commands::load_page,
            commands::load_table_page,
            commands::run_query,
            commands::schema_map,
            commands::schema_graph,
            commands::count_rows,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DBGo");
}
