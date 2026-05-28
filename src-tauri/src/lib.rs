//! gdxcomp Tauri application entry point.

mod commands;

use commands::{load_cli_args, AppState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let initial_files = load_cli_args();
    let state = if initial_files.is_empty() {
        AppState::default()
    } else {
        AppState::with_files(initial_files)
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::open_gdx,
            commands::open_folder,
            commands::remove_gdx,
            commands::clear_files,
            commands::list_files,
            commands::common_symbols_cmd,
            commands::distinct_keys,
            commands::get_view,
            commands::get_chart_view,
            commands::get_table_view,
            commands::rename_scenario,
            commands::reset_scenarios,
            commands::save_session,
            commands::load_session,
            commands::read_param_map,
        ])
        .run(tauri::generate_context!())
        .expect("error while running gdxcomp");
}
