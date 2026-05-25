//! Tauri commands: the only surface the frontend talks to.
//!
//! All GDX access and view computation happens here via [`gdxcomp_core`].
//! Files are cached in app state (metadata only); records are read lazily per
//! `get_view` call and not held in memory between calls.

use std::path::PathBuf;
use std::sync::Mutex;

use gdxcomp_core::{
    build_view, common_symbols, refine_setup, DisplaySetup, LoadedFile, PlotView, SymbolMeta,
};
use serde::Serialize;
use tauri::State;

/// In-memory cache of the currently selected files (metadata only).
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

/// Result of `get_view`: the rendered plot plus the effective setup that was
/// actually used (may differ from the input if `refine_setup` added defaults).
/// The UI stores `setup` back so the filter panel reflects auto-selections.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetViewResult {
    pub view: PlotView,
    pub setup: DisplaySetup,
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

/// Load all `.gdx` files found directly inside `path` (non-recursive).
/// Files already in the selection are skipped. Returns the full updated selection.
#[tauri::command]
pub fn open_folder(path: String, state: State<AppState>) -> CmdResult<Vec<FileMeta>> {
    let dir = PathBuf::from(&path);
    let mut gdx_paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("{path}: {e}"))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("gdx"))
        .collect();
    gdx_paths.sort();

    let mut files = state.files.lock().unwrap();
    for gdx_path in gdx_paths {
        if files.iter().any(|f| f.path == gdx_path) {
            continue;
        }
        let loaded =
            LoadedFile::open(&gdx_path).map_err(|e| format!("{}: {e}", gdx_path.display()))?;
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
        let keys = f.distinct_keys(&symbol, dim).unwrap_or_default();
        for k in keys {
            if !out.contains(&k) {
                out.push(k);
            }
        }
    }
    out
}

/// Build the chart + table for the given display setup.
///
/// Applies `refine_setup` before building so that an unfiltered series dimension
/// defaults to the first available UEL. Returns the effective setup alongside the
/// view so the UI can update its filter state.
#[tauri::command]
pub fn get_view(setup: DisplaySetup, state: State<AppState>) -> CmdResult<GetViewResult> {
    let files = state.files.lock().unwrap();
    let effective = refine_setup(&files, &setup).map_err(|e| e.to_string())?;
    let view = build_view(&files, &effective).map_err(|e| e.to_string())?;
    Ok(GetViewResult {
        view,
        setup: effective,
    })
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
