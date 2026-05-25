//! Tauri commands: the only surface the frontend talks to.
//!
//! All GDX access and view computation happens here via [`gdxcomp_core`].
//! Loaded files are cached in app state so re-plotting never re-reads the disk.

use std::path::PathBuf;
use std::sync::Mutex;

use gdxcomp_core::{
    build_view, common_symbols, DisplaySetup, LoadedFile, PlotView, SymbolMeta,
};
use serde::Serialize;
use tauri::State;

/// In-memory cache of the currently selected files, in user-chosen order.
#[derive(Default)]
pub struct AppState {
    files: Mutex<Vec<LoadedFile>>,
}

/// Lightweight per-file summary sent to the UI.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMeta {
    pub label: String,
    pub path: String,
    pub symbols: Vec<SymbolMeta>,
}

impl From<&LoadedFile> for FileMeta {
    fn from(f: &LoadedFile) -> Self {
        FileMeta {
            label: f.label.clone(),
            path: f.path.to_string_lossy().into_owned(),
            symbols: f.symbols.clone(),
        }
    }
}

type CmdResult<T> = Result<T, String>;

fn snapshot(files: &[LoadedFile]) -> Vec<FileMeta> {
    files.iter().map(FileMeta::from).collect()
}

/// Load one or more GDX files (skipping any already loaded) and return the full
/// current selection.
#[tauri::command]
pub fn open_gdx(paths: Vec<String>, state: State<AppState>) -> CmdResult<Vec<FileMeta>> {
    let mut files = state.files.lock().unwrap();
    for path in paths {
        let path = PathBuf::from(path);
        if files.iter().any(|f| f.path == path) {
            continue;
        }
        let loaded = LoadedFile::open(&path).map_err(|e| format!("{path:?}: {e}"))?;
        files.push(loaded);
    }
    Ok(snapshot(&files))
}

/// Remove a file from the selection by its path.
#[tauri::command]
pub fn remove_gdx(path: String, state: State<AppState>) -> Vec<FileMeta> {
    let mut files = state.files.lock().unwrap();
    let path = PathBuf::from(path);
    files.retain(|f| f.path != path);
    snapshot(&files)
}

/// The current file selection.
#[tauri::command]
pub fn list_files(state: State<AppState>) -> Vec<FileMeta> {
    let files = state.files.lock().unwrap();
    snapshot(&files)
}

/// Symbols comparable across every selected file (same name, dim and kind).
#[tauri::command]
pub fn common_symbols_cmd(state: State<AppState>) -> Vec<SymbolMeta> {
    let files = state.files.lock().unwrap();
    common_symbols(&files)
}

/// Distinct UEL labels in dimension `dim` of `symbol`, unioned across files —
/// used to populate the filter controls.
#[tauri::command]
pub fn distinct_keys(symbol: String, dim: usize, state: State<AppState>) -> Vec<String> {
    let files = state.files.lock().unwrap();
    let mut out: Vec<String> = Vec::new();
    for f in files.iter() {
        for k in f.distinct_keys(&symbol, dim) {
            if !out.contains(&k) {
                out.push(k);
            }
        }
    }
    out
}

/// Build the chart + table for the given display setup.
#[tauri::command]
pub fn get_view(setup: DisplaySetup, state: State<AppState>) -> CmdResult<PlotView> {
    let files = state.files.lock().unwrap();
    build_view(&files, &setup).map_err(|e| e.to_string())
}

/// Write a display setup to `path` as JSON.
#[tauri::command]
pub fn save_setup(path: String, setup: DisplaySetup) -> CmdResult<()> {
    let json = setup.to_json().map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("{path}: {e}"))
}

/// Read a display setup from a JSON file at `path`.
#[tauri::command]
pub fn load_setup(path: String) -> CmdResult<DisplaySetup> {
    let json = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
    DisplaySetup::from_json(&json).map_err(|e| e.to_string())
}
