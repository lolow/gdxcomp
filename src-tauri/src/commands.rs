//! Tauri commands: the only surface the frontend talks to.
//!
//! All GDX access and view computation happens here via [`gdxcomp_core`].
//! Files are cached in app state (metadata only); records are read lazily per
//! `get_view` call and not held in memory between calls.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use gdxcomp_core::{
    build_view, common_symbols, refine_setup, DisplaySetup, LoadedFile, PlotView, SymbolMeta,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

// ---------------------------------------------------------------------------
// Scenario name helpers
// ---------------------------------------------------------------------------

fn common_prefix_chars(labels: &[String]) -> usize {
    if labels.is_empty() {
        return 0;
    }
    labels[1..]
        .iter()
        .fold(labels[0].chars().count(), |len, s| {
            labels[0]
                .chars()
                .zip(s.chars())
                .take_while(|(a, b)| a == b)
                .count()
                .min(len)
        })
}

fn common_suffix_chars(labels: &[String]) -> usize {
    if labels.is_empty() {
        return 0;
    }
    let first_rev: Vec<char> = labels[0].chars().rev().collect();
    labels[1..].iter().fold(first_rev.len(), |len, s| {
        let rev: Vec<char> = s.chars().rev().collect();
        first_rev
            .iter()
            .zip(rev.iter())
            .take_while(|(a, b)| a == b)
            .count()
            .min(len)
    })
}

fn distinctive_names(labels: &[String]) -> Vec<String> {
    if labels.len() <= 1 {
        return labels.to_vec();
    }
    let prefix = common_prefix_chars(labels);
    let suffix = common_suffix_chars(labels);
    labels
        .iter()
        .map(|label| {
            let chars: Vec<char> = label.chars().collect();
            let total = chars.len();
            let end = total.saturating_sub(suffix);
            let start = prefix.min(end);
            let d: String = chars[start..end].iter().collect();
            let d = d.trim_matches(|c: char| c == '_' || c == '-' || c == '.');
            if d.is_empty() {
                label.clone()
            } else {
                d.to_string()
            }
        })
        .collect()
}

fn recompute_scenarios(entries: &mut [FileEntry]) {
    let labels: Vec<String> = entries.iter().map(|e| e.file.label.clone()).collect();
    let names = distinctive_names(&labels);
    for (entry, name) in entries.iter_mut().zip(names.iter()) {
        if !entry.customized {
            entry.scenario = name.clone();
        }
    }
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct FileEntry {
    file: LoadedFile,
    scenario: String,
    customized: bool,
}

/// In-memory cache of the currently selected files (metadata only).
#[derive(Default)]
pub struct AppState {
    entries: Mutex<Vec<FileEntry>>,
}

impl AppState {
    pub fn with_files(files: Vec<LoadedFile>) -> Self {
        let mut entries: Vec<FileEntry> = files
            .into_iter()
            .map(|file| {
                let scenario = file.label.clone();
                FileEntry {
                    file,
                    scenario,
                    customized: false,
                }
            })
            .collect();
        recompute_scenarios(&mut entries);
        AppState {
            entries: Mutex::new(entries),
        }
    }
}

/// Parse CLI arguments and return pre-loaded files for the initial app state.
pub fn load_cli_args() -> Vec<LoadedFile> {
    let raw: Vec<PathBuf> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .flat_map(|arg| {
            let p = PathBuf::from(&arg);
            if p.is_dir() {
                let mut gdx: Vec<PathBuf> = std::fs::read_dir(&p)
                    .map(|rd| {
                        rd.filter_map(|e| e.ok())
                            .map(|e| e.path())
                            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("gdx"))
                            .collect()
                    })
                    .unwrap_or_default();
                gdx.sort();
                gdx
            } else if p.is_file() {
                vec![p]
            } else {
                if !arg.is_empty() {
                    eprintln!("gdxcomp: {arg}: no such file or directory");
                }
                vec![]
            }
        })
        .collect();

    let mut files: Vec<LoadedFile> = Vec::new();
    for path in raw {
        if files.iter().any(|f| f.path == path) {
            continue;
        }
        match LoadedFile::open(&path) {
            Ok(loaded) => files.push(loaded),
            Err(e) => eprintln!("gdxcomp: {}: {e}", path.display()),
        }
    }
    files
}

// ---------------------------------------------------------------------------
// IPC types
// ---------------------------------------------------------------------------

/// Lightweight per-file summary sent to the UI.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMeta {
    pub label: String,
    pub scenario: String,
    pub path: String,
    pub symbols: Vec<SymbolMeta>,
}

impl From<&FileEntry> for FileMeta {
    fn from(e: &FileEntry) -> Self {
        FileMeta {
            label: e.file.label.clone(),
            scenario: e.scenario.clone(),
            path: e.file.path.to_string_lossy().into_owned(),
            symbols: e.file.symbols.clone(),
        }
    }
}

/// Result of `get_view`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetViewResult {
    pub view: PlotView,
    pub setup: DisplaySetup,
}

type CmdResult<T> = Result<T, String>;

fn snapshot(entries: &[FileEntry]) -> Vec<FileMeta> {
    entries.iter().map(FileMeta::from).collect()
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn open_gdx(paths: Vec<String>, state: State<AppState>) -> CmdResult<Vec<FileMeta>> {
    let mut entries = state.entries.lock().unwrap();
    for path in paths {
        let path = PathBuf::from(path);
        if entries.iter().any(|e| e.file.path == path) {
            continue;
        }
        let file = LoadedFile::open(&path).map_err(|e| format!("{path:?}: {e}"))?;
        let scenario = file.label.clone();
        entries.push(FileEntry {
            file,
            scenario,
            customized: false,
        });
    }
    recompute_scenarios(&mut entries);
    Ok(snapshot(&entries))
}

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

    let mut entries = state.entries.lock().unwrap();
    for gdx_path in gdx_paths {
        if entries.iter().any(|e| e.file.path == gdx_path) {
            continue;
        }
        let file =
            LoadedFile::open(&gdx_path).map_err(|e| format!("{}: {e}", gdx_path.display()))?;
        let scenario = file.label.clone();
        entries.push(FileEntry {
            file,
            scenario,
            customized: false,
        });
    }
    recompute_scenarios(&mut entries);
    Ok(snapshot(&entries))
}

#[tauri::command]
pub fn remove_gdx(path: String, state: State<AppState>) -> Vec<FileMeta> {
    let mut entries = state.entries.lock().unwrap();
    let path = PathBuf::from(path);
    entries.retain(|e| e.file.path != path);
    recompute_scenarios(&mut entries);
    snapshot(&entries)
}

#[tauri::command]
pub fn clear_files(state: State<AppState>) -> Vec<FileMeta> {
    state.entries.lock().unwrap().clear();
    vec![]
}

#[tauri::command]
pub fn list_files(state: State<AppState>) -> Vec<FileMeta> {
    let entries = state.entries.lock().unwrap();
    snapshot(&entries)
}

#[tauri::command]
pub fn rename_scenario(path: String, scenario: String, state: State<AppState>) -> Vec<FileMeta> {
    let mut entries = state.entries.lock().unwrap();
    let path = PathBuf::from(path);
    if let Some(entry) = entries.iter_mut().find(|e| e.file.path == path) {
        entry.scenario = scenario;
        entry.customized = true;
    }
    snapshot(&entries)
}

#[tauri::command]
pub fn reset_scenarios(state: State<AppState>) -> Vec<FileMeta> {
    let mut entries = state.entries.lock().unwrap();
    for entry in entries.iter_mut() {
        entry.customized = false;
    }
    recompute_scenarios(&mut entries);
    snapshot(&entries)
}

#[tauri::command]
pub fn common_symbols_cmd(state: State<AppState>) -> Vec<SymbolMeta> {
    let entries = state.entries.lock().unwrap();
    let files: Vec<LoadedFile> = entries.iter().map(|e| e.file.clone()).collect();
    common_symbols(&files)
}

#[tauri::command]
pub fn distinct_keys(symbol: String, dim: usize, state: State<AppState>) -> Vec<String> {
    let entries = state.entries.lock().unwrap();
    let mut out: Vec<String> = Vec::new();
    for e in entries.iter() {
        let keys = e.file.distinct_keys(&symbol, dim).unwrap_or_default();
        for k in keys {
            if !out.contains(&k) {
                out.push(k);
            }
        }
    }
    out
}

#[tauri::command]
pub fn get_view(setup: DisplaySetup, state: State<AppState>) -> CmdResult<GetViewResult> {
    let entries = state.entries.lock().unwrap();
    // Use scenario as the trace label by overriding file.label before the call.
    let files: Vec<LoadedFile> = entries
        .iter()
        .map(|e| {
            let mut f = e.file.clone();
            f.label = e.scenario.clone();
            f
        })
        .collect();
    let effective = refine_setup(&files, &setup).map_err(|e| e.to_string())?;
    let view = build_view(&files, &effective).map_err(|e| e.to_string())?;
    Ok(GetViewResult {
        view,
        setup: effective,
    })
}

// ---------------------------------------------------------------------------
// Session persistence
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub files: Vec<String>,
    pub last_symbol: Option<String>,
}

fn session_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("session.json"))
}

#[tauri::command]
pub fn save_session(session: Session, app: AppHandle) {
    if let Some(path) = session_path(&app) {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string(&session) {
            let _ = fs::write(path, json);
        }
    }
}

#[tauri::command]
pub fn load_session(app: AppHandle) -> Option<Session> {
    let path = session_path(&app)?;
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

// ---------------------------------------------------------------------------
// Parameter utilities
// ---------------------------------------------------------------------------

/// Read a 1-dim parameter as a {key -> level_value} map from the first file
/// that contains the symbol.
#[tauri::command]
pub fn read_param_map(symbol: String, state: State<AppState>) -> HashMap<String, f64> {
    let entries = state.entries.lock().unwrap();
    for e in entries.iter() {
        if let Ok(records) = e.file.read_records(&symbol) {
            let map: HashMap<String, f64> = records
                .into_iter()
                .filter_map(|r| {
                    let key = r.keys.into_iter().next()?;
                    let val = r.values[0];
                    val.is_finite().then_some((key, val))
                })
                .collect();
            if !map.is_empty() {
                return map;
            }
        }
    }
    HashMap::new()
}
