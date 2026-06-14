//! Benchmarks for the four hot paths in the display pipeline:
//!   1. Open one or many GDX files (metadata only)
//!   2. Read one symbol's records from disk
//!   3. distinct_keys across one or many files
//!   4. build_view (one-dim and multi-dim aggregation)
//!
//! Run with:
//!   cargo bench -p gdxcomp-core
//!
//! Fixtures (all optional, benches skip if missing):
//!   gdx_examples/results_ssp2_bau_devel.gdx
//!   gdx_examples/*.gdx                        (4 files)
//!   gdx_examples_more/*.gdx                   (15 more files)

use criterion::{criterion_group, criterion_main, Criterion};
use gdx::GdxFile;
use gdxcomp_core::{build_chart, build_view, refine_setup, DisplaySetup, LoadedFile, SymbolKind, SymbolMeta};
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../gdx_examples")
}

fn examples_more_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../gdx_examples_more")
}

fn example_path() -> PathBuf {
    examples_dir().join("results_ssp2_bau_devel.gdx")
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

/// Find a parameter/variable symbol present in all files and pick the one with
/// the highest record count from the first file (a stable proxy).
fn largest_shared(files: &[LoadedFile]) -> Option<&SymbolMeta> {
    let first = files.first()?;
    let mut candidates: Vec<&SymbolMeta> = first
        .symbols
        .iter()
        .filter(|s| s.kind != SymbolKind::Set && s.kind != SymbolKind::Alias)
        .filter(|s| files[1..].iter().all(|f| f.symbol(&s.name).is_some()))
        .collect();
    candidates.sort_by_key(|s| std::cmp::Reverse(s.records));
    candidates.first().copied()
}

/// Find a 3+ dim symbol present in all files (for two-dim aggregation bench).
fn three_dim_shared(files: &[LoadedFile]) -> Option<&SymbolMeta> {
    let first = files.first()?;
    first
        .symbols
        .iter()
        .filter(|s| s.dim >= 3 && s.kind != SymbolKind::Set && s.kind != SymbolKind::Alias)
        .find(|s| files[1..].iter().all(|f| f.symbol(&s.name).is_some()))
}

// ---------------------------------------------------------------------------
// Phase-0 baseline benches: single-file (kept for continuity).
// ---------------------------------------------------------------------------

fn bench_open_metadata(c: &mut Criterion) {
    let path = example_path();
    if !path.exists() {
        eprintln!("Skipping open_metadata: example file not found");
        return;
    }
    c.bench_function("open_metadata", |b| {
        b.iter(|| LoadedFile::open(&path).unwrap())
    });
}

fn bench_read_records(c: &mut Criterion) {
    let path = example_path();
    if !path.exists() {
        eprintln!("Skipping read_records_ykali: example file not found");
        return;
    }
    let file = LoadedFile::open(&path).unwrap();
    c.bench_function("read_records_ykali", |b| {
        b.iter(|| file.read_records("ykali").unwrap())
    });
}

fn bench_build_view(c: &mut Criterion) {
    let path = example_path();
    if !path.exists() {
        eprintln!("Skipping build_view_ykali: example file not found");
        return;
    }
    let files = vec![LoadedFile::open(&path).unwrap()];
    let mut setup = DisplaySetup::for_symbol("ykali");
    setup.x_dim = 0;
    let setup = refine_setup(&files, &setup).unwrap();
    c.bench_function("build_view_ykali", |b| {
        b.iter(|| build_view(&files, &setup).unwrap())
    });
}

fn bench_refine_and_build(c: &mut Criterion) {
    let path = example_path();
    if !path.exists() {
        eprintln!("Skipping refine_and_build_view_ykali: example file not found");
        return;
    }
    let files = vec![LoadedFile::open(&path).unwrap()];
    c.bench_function("refine_and_build_view_ykali", |b| {
        b.iter(|| {
            let mut setup = DisplaySetup::for_symbol("ykali");
            setup.x_dim = 0;
            let setup = refine_setup(&files, &setup).unwrap();
            build_view(&files, &setup).unwrap()
        })
    });
}

// ---------------------------------------------------------------------------
// Phase-0 extended benches.
// ---------------------------------------------------------------------------

fn bench_open_metadata_4files(c: &mut Criterion) {
    let paths = list_gdx(&examples_dir());
    if paths.is_empty() {
        eprintln!("Skipping open_metadata_4files: gdx_examples/ missing");
        return;
    }
    c.bench_function("open_metadata_4files", |b| {
        b.iter(|| {
            let _files: Vec<_> = paths.iter().map(|p| LoadedFile::open(p).unwrap()).collect();
        })
    });
}

fn bench_open_metadata_19files(c: &mut Criterion) {
    let mut paths = list_gdx(&examples_dir());
    paths.extend(list_gdx(&examples_more_dir()));
    if paths.len() < 5 {
        eprintln!("Skipping open_metadata_19files: needs gdx_examples_more/");
        return;
    }
    c.bench_function("open_metadata_19files", |b| {
        b.iter(|| {
            let _files: Vec<_> = paths.iter().map(|p| LoadedFile::open(p).unwrap()).collect();
        })
    });
}

fn bench_read_records_largest(c: &mut Criterion) {
    let path = example_path();
    if !path.exists() {
        return;
    }
    let file = LoadedFile::open(&path).unwrap();
    let files = vec![file];
    let symbol_name = largest_shared(&files).map(|s| s.name.clone());
    let Some(name) = symbol_name else {
        eprintln!("Skipping read_records_largest_symbol: no shared symbol");
        return;
    };
    eprintln!("read_records_largest_symbol: picked '{name}'");
    let file = &files[0];
    c.bench_function("read_records_largest_symbol", |b| {
        b.iter(|| file.read_records(&name).unwrap())
    });
}

fn bench_cold_read_largest_symbol(c: &mut Criterion) {
    let path = example_path();
    if !path.exists() {
        return;
    }
    let file = LoadedFile::open(&path).unwrap();
    let files = vec![file];
    let Some(name) = largest_shared(&files).map(|s| s.name.clone()) else {
        eprintln!("Skipping cold_read_largest_symbol: no shared symbol");
        return;
    };
    eprintln!("cold_read_largest_symbol: picked '{name}'");
    // Open a fresh GdxFile each iteration — bypasses the LoadedFile LRU cache,
    // so every iteration is a true cold FFI read.
    c.bench_function("cold_read_largest_symbol", |b| {
        b.iter(|| GdxFile::open(&path).unwrap().read(&name).unwrap())
    });
}

fn bench_distinct_keys_dim0_ykali(c: &mut Criterion) {
    let path = example_path();
    if !path.exists() {
        return;
    }
    let file = LoadedFile::open(&path).unwrap();
    c.bench_function("distinct_keys_dim0_ykali", |b| {
        b.iter(|| file.distinct_keys("ykali", 0).unwrap())
    });
}

fn bench_distinct_keys_dim0_19files(c: &mut Criterion) {
    let mut paths = list_gdx(&examples_dir());
    paths.extend(list_gdx(&examples_more_dir()));
    if paths.len() < 5 {
        eprintln!("Skipping distinct_keys_dim0_19files: needs gdx_examples_more/");
        return;
    }
    let files = load_all(&paths);
    c.bench_function("distinct_keys_dim0_19files", |b| {
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
            out
        })
    });
}

fn bench_build_view_aggregated_4files(c: &mut Criterion) {
    let paths = list_gdx(&examples_dir());
    if paths.len() < 4 {
        eprintln!("Skipping build_view_aggregated_4files: needs gdx_examples/");
        return;
    }
    let files = load_all(&paths);
    let mut setup = DisplaySetup::for_symbol("ykali");
    setup.x_dim = 0;
    // refine_setup sums non-x dims by default (test verifies this).
    let setup = refine_setup(&files, &setup).unwrap();
    c.bench_function("build_view_aggregated_4files", |b| {
        b.iter(|| build_view(&files, &setup).unwrap())
    });
}

fn bench_build_chart_aggregated_4files(c: &mut Criterion) {
    let paths = list_gdx(&examples_dir());
    if paths.len() < 4 {
        eprintln!("Skipping build_chart_aggregated_4files: needs gdx_examples/");
        return;
    }
    let files = load_all(&paths);
    let mut setup = DisplaySetup::for_symbol("ykali");
    setup.x_dim = 0;
    let setup = refine_setup(&files, &setup).unwrap();
    c.bench_function("build_chart_aggregated_4files", |b| {
        b.iter(|| build_chart(&files, &setup).unwrap())
    });
}

fn bench_build_view_2dim_aggregated_4files(c: &mut Criterion) {
    let paths = list_gdx(&examples_dir());
    if paths.len() < 4 {
        eprintln!("Skipping build_view_2dim_aggregated_4files: needs gdx_examples/");
        return;
    }
    let files = load_all(&paths);
    let Some(sym) = three_dim_shared(&files) else {
        eprintln!("Skipping build_view_2dim_aggregated_4files: no 3+ dim symbol");
        return;
    };
    let name = sym.name.clone();
    eprintln!("build_view_2dim_aggregated_4files: picked '{name}' (dim={})", sym.dim);
    let mut setup = DisplaySetup::for_symbol(&name);
    setup.x_dim = 0;
    let setup = refine_setup(&files, &setup).unwrap();
    c.bench_function("build_view_2dim_aggregated_4files", |b| {
        b.iter(|| build_view(&files, &setup).unwrap())
    });
}

criterion_group!(
    benches,
    bench_open_metadata,
    bench_read_records,
    bench_build_view,
    bench_refine_and_build,
    bench_open_metadata_4files,
    bench_open_metadata_19files,
    bench_read_records_largest,
    bench_cold_read_largest_symbol,
    bench_distinct_keys_dim0_ykali,
    bench_distinct_keys_dim0_19files,
    bench_build_view_aggregated_4files,
    bench_build_chart_aggregated_4files,
    bench_build_view_2dim_aggregated_4files,
);
criterion_main!(benches);
