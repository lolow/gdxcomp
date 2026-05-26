//! Benchmarks for the three hot paths in the display pipeline:
//!   1. Open a GDX file (metadata only)
//!   2. Read one symbol's records from disk
//!   3. End-to-end build_view (refine + read + aggregate)
//!
//! Run with:
//!   cargo bench -p gdxcomp-core
//!
//! Requires the example files to be present in gdx_examples/:
//!   results_ssp2_bau_devel.gdx

use criterion::{criterion_group, criterion_main, Criterion};
use gdxcomp_core::{build_view, refine_setup, DisplaySetup, LoadedFile};
use std::path::PathBuf;

fn example_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../gdx_examples/results_ssp2_bau_devel.gdx")
}

fn bench_open_metadata(c: &mut Criterion) {
    let path = example_path();
    if !path.exists() {
        eprintln!("Skipping bench_open_metadata: example file not found");
        return;
    }
    c.bench_function("open_metadata", |b| {
        b.iter(|| LoadedFile::open(&path).unwrap())
    });
}

fn bench_read_records(c: &mut Criterion) {
    let path = example_path();
    if !path.exists() {
        eprintln!("Skipping bench_read_records: example file not found");
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
        eprintln!("Skipping bench_build_view: example file not found");
        return;
    }
    let file = LoadedFile::open(&path).unwrap();
    let files = vec![file];

    let mut setup = DisplaySetup::for_symbol("ykali");
    setup.x_dim = 0;
    // Pre-refine so bench measures only build_view, not the extra distinct_keys read.
    let setup = refine_setup(&files, &setup).unwrap();

    c.bench_function("build_view_ykali", |b| {
        b.iter(|| build_view(&files, &setup).unwrap())
    });
}

fn bench_refine_and_build(c: &mut Criterion) {
    let path = example_path();
    if !path.exists() {
        eprintln!("Skipping bench_refine_and_build: example file not found");
        return;
    }
    let file = LoadedFile::open(&path).unwrap();
    let files = vec![file];

    c.bench_function("refine_and_build_view_ykali", |b| {
        b.iter(|| {
            let mut setup = DisplaySetup::for_symbol("ykali");
            setup.x_dim = 0;
            let setup = refine_setup(&files, &setup).unwrap();
            build_view(&files, &setup).unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_open_metadata,
    bench_read_records,
    bench_build_view,
    bench_refine_and_build
);
criterion_main!(benches);
