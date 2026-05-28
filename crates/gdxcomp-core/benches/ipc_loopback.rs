//! IPC-loopback benchmarks: measure the same work the Tauri commands do,
//! including the JSON serialization that Tauri's IPC layer performs on the
//! return value. We bench in gdxcomp-core (not src-tauri) so we don't need
//! webkit2gtk in the bench environment.
//!
//! Run with:
//!   cargo bench -p gdxcomp-core --bench ipc_loopback
//!
//! Fixtures: gdx_examples/*.gdx (4 files); optional gdx_examples_more/ (19 files).

use criterion::{criterion_group, criterion_main, Criterion};
use gdxcomp_core::{
    build_chart, build_table, build_view, common_symbols, refine_setup, DisplaySetup, LoadedFile,
};
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../gdx_examples")
}
fn examples_more_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../gdx_examples_more")
}

fn list_gdx(dir: &PathBuf) -> Vec<PathBuf> {
    if !dir.exists() {
        return Vec::new();
    }
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("gdx"))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

fn load_all(paths: &[PathBuf]) -> Vec<LoadedFile> {
    paths.iter().map(|p| LoadedFile::open(p).unwrap()).collect()
}

/// Mirrors `common_symbols_cmd` body: clone files, intersect, serialize.
fn bench_ipc_common_symbols_4files(c: &mut Criterion) {
    let paths = list_gdx(&examples_dir());
    if paths.len() < 4 {
        return;
    }
    let files = load_all(&paths);
    c.bench_function("ipc_common_symbols_4files", |b| {
        b.iter(|| {
            let clones: Vec<LoadedFile> = files.to_vec();
            let symbols = common_symbols(&clones);
            let _json = serde_json::to_string(&symbols).unwrap();
        })
    });
}

fn bench_ipc_common_symbols_19files(c: &mut Criterion) {
    let mut paths = list_gdx(&examples_dir());
    paths.extend(list_gdx(&examples_more_dir()));
    if paths.len() < 5 {
        return;
    }
    let files = load_all(&paths);
    c.bench_function("ipc_common_symbols_19files", |b| {
        b.iter(|| {
            let clones: Vec<LoadedFile> = files.to_vec();
            let symbols = common_symbols(&clones);
            let _json = serde_json::to_string(&symbols).unwrap();
        })
    });
}

/// Mirrors `distinct_keys` body: per-file scan + Vec::contains accumulator + JSON.
fn bench_ipc_distinct_keys_4files(c: &mut Criterion) {
    let paths = list_gdx(&examples_dir());
    if paths.len() < 4 {
        return;
    }
    let files = load_all(&paths);
    c.bench_function("ipc_distinct_keys_4files", |b| {
        b.iter(|| {
            let mut out: Vec<String> = Vec::new();
            for f in &files {
                let keys = f.distinct_keys("ykali", 0).unwrap_or_default();
                for k in keys {
                    if !out.contains(&k) {
                        out.push(k);
                    }
                }
            }
            let _json = serde_json::to_string(&out).unwrap();
        })
    });
}

/// Mirrors `get_view` body: clone files, refine, build, serialize (full PlotView).
fn bench_ipc_get_view_4files(c: &mut Criterion) {
    let paths = list_gdx(&examples_dir());
    if paths.len() < 4 {
        return;
    }
    let files = load_all(&paths);
    let mut setup = DisplaySetup::for_symbol("ykali");
    setup.x_dim = 0;
    let setup = refine_setup(&files, &setup).unwrap();
    c.bench_function("ipc_get_view_4files", |b| {
        b.iter(|| {
            let clones: Vec<LoadedFile> = files.to_vec();
            let view = build_view(&clones, &setup).unwrap();
            let _json = serde_json::to_string(&view).unwrap();
        })
    });
}

/// Mirrors `get_chart_view` body: chart-only payload (no `table` field).
fn bench_ipc_get_chart_view_4files(c: &mut Criterion) {
    let paths = list_gdx(&examples_dir());
    if paths.len() < 4 {
        return;
    }
    let files = load_all(&paths);
    let mut setup = DisplaySetup::for_symbol("ykali");
    setup.x_dim = 0;
    let setup = refine_setup(&files, &setup).unwrap();
    c.bench_function("ipc_get_chart_view_4files", |b| {
        b.iter(|| {
            let clones: Vec<LoadedFile> = files.to_vec();
            let view = build_chart(&clones, &setup).unwrap();
            let _json = serde_json::to_string(&view).unwrap();
        })
    });
}

/// Mirrors `get_table_view` body: table-only payload.
fn bench_ipc_get_table_view_4files(c: &mut Criterion) {
    let paths = list_gdx(&examples_dir());
    if paths.len() < 4 {
        return;
    }
    let files = load_all(&paths);
    let mut setup = DisplaySetup::for_symbol("ykali");
    setup.x_dim = 0;
    let setup = refine_setup(&files, &setup).unwrap();
    c.bench_function("ipc_get_table_view_4files", |b| {
        b.iter(|| {
            let clones: Vec<LoadedFile> = files.to_vec();
            let view = build_table(&clones, &setup).unwrap();
            let _json = serde_json::to_string(&view).unwrap();
        })
    });
}

criterion_group!(
    benches,
    bench_ipc_common_symbols_4files,
    bench_ipc_common_symbols_19files,
    bench_ipc_distinct_keys_4files,
    bench_ipc_get_view_4files,
    bench_ipc_get_chart_view_4files,
    bench_ipc_get_table_view_4files,
);
criterion_main!(benches);
