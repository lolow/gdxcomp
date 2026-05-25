//! gdxcomp Tauri application entry point.

mod commands;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::open_gdx,
            commands::remove_gdx,
            commands::list_files,
            commands::common_symbols_cmd,
            commands::distinct_keys,
            commands::get_view,
            commands::save_setup,
            commands::load_setup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running gdxcomp");
}
