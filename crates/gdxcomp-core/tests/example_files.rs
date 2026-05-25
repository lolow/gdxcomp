//! Integration tests against the example IAM result files in gdx_examples/.
//! Run with:
//!   cargo test -p gdxcomp-core --test example_files -- --ignored
//!
//! The files are large (~18-20 MB each); loading metadata is fast (~1 s per file);
//! reading records for a specific symbol is fast (~1–2 s per file).

use gdxcomp_core::{
    build_view, common_symbols, refine_setup, CoreError, DisplaySetup, Field, LoadedFile,
};
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../gdx_examples")
}

fn load(name: &str) -> LoadedFile {
    let path = examples_dir().join(name);
    assert!(path.exists(), "example file not found: {}", path.display());
    LoadedFile::open(path.to_str().unwrap()).expect("open gdx")
}

#[test]
#[ignore = "requires gdx_examples/ (large files)"]
fn metadata_only_open_is_fast() {
    // Metadata load should NOT read any record data.
    let t = std::time::Instant::now();
    let file = load("results_ssp2_bau_devel.gdx");
    let elapsed = t.elapsed();
    // Symbol table has ~4500 entries; metadata read should complete in <5 s.
    assert!(
        elapsed.as_secs() < 5,
        "metadata open took {elapsed:?}, expected <5 s"
    );
    assert!(
        file.symbols.len() > 100,
        "expected many symbols, got {}",
        file.symbols.len()
    );
}

#[test]
#[ignore = "requires gdx_examples/ (large files)"]
fn bau_files_share_common_symbols() {
    let devel = load("results_ssp2_bau_devel.gdx");
    let master = load("results_ssp2_bau_master.gdx");
    let common = common_symbols(&[devel, master]);
    assert!(
        common.len() > 100,
        "expected many shared symbols, got {}",
        common.len()
    );
    let names: Vec<&str> = common.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"ykali"), "ykali missing from common set");
    assert!(
        names.contains(&"TEMP_REGION"),
        "TEMP_REGION missing from common set"
    );
}

#[test]
#[ignore = "requires gdx_examples/ (large files)"]
fn refine_setup_picks_first_series_value() {
    let devel = load("results_ssp2_bau_devel.gdx");
    let master = load("results_ssp2_bau_master.gdx");
    let files = vec![devel, master];

    let mut setup = DisplaySetup::for_symbol("ykali");
    setup.x_dim = 0;
    setup.series_dim = Some(1); // regions — 17 distinct values

    let refined = refine_setup(&files, &setup).unwrap();

    // A filter for dim 1 must have been added with exactly one value.
    let filter = refined
        .filters
        .get(&1)
        .expect("refine_setup must set a filter for the series dim");
    assert_eq!(filter.len(), 1, "expected exactly one default value");
}

#[test]
#[ignore = "requires gdx_examples/ (large files)"]
fn overlay_ykali_across_bau_files() {
    let devel = load("results_ssp2_bau_devel.gdx");
    let master = load("results_ssp2_bau_master.gdx");
    let files = vec![devel, master];

    let mut setup = DisplaySetup::for_symbol("ykali");
    setup.x_dim = 0;
    setup.series_dim = Some(1);

    // Use refine_setup so we get a single default region → 2 traces (1 region × 2 files).
    let setup = refine_setup(&files, &setup).unwrap();
    let view = build_view(&files, &setup).unwrap();
    assert_eq!(
        view.traces.len(),
        2,
        "expected 2 traces (1 region × 2 files)"
    );

    // Brazil, year "1": devel value from gdxdump = 1.15315947775526
    let brazil_devel = view
        .traces
        .iter()
        .find(|t| t.name.contains("brazil") && t.name.contains("devel"))
        .expect("trace for brazil/devel not found");
    let idx = brazil_devel
        .x
        .iter()
        .position(|x| x == "1")
        .expect("time period '1' not found");
    assert!(
        (brazil_devel.y[idx] - 1.15315947775526_f64).abs() < 1e-9,
        "brazil/devel year 1 value mismatch: got {}",
        brazil_devel.y[idx]
    );
}

#[test]
#[ignore = "requires gdx_examples/ (large files)"]
fn too_many_traces_without_filter_returns_error() {
    let devel = load("results_ssp2_bau_devel.gdx");
    let master = load("results_ssp2_bau_master.gdx");
    let files = vec![devel, master];

    // 17 regions × 2 files = 34 traces — above MAX_TRACES (30).
    let mut setup = DisplaySetup::for_symbol("ykali");
    setup.x_dim = 0;
    setup.series_dim = Some(1);
    // No filter, no refine_setup → must fail.
    let err = build_view(&files, &setup).expect_err("expected TooManyTraces error");
    assert!(
        matches!(err, CoreError::TooManyTraces { .. }),
        "unexpected error: {err}"
    );
}

#[test]
#[ignore = "requires gdx_examples/ (large files)"]
fn overlay_temp_region_variable_level() {
    let devel = load("results_ssp2_bau_devel.gdx");
    let master = load("results_ssp2_bau_master.gdx");
    let files = vec![devel, master];

    let mut setup = DisplaySetup::for_symbol("TEMP_REGION");
    setup.x_dim = 0;
    setup.series_dim = Some(1);
    setup.field = Field::Level;

    let setup = refine_setup(&files, &setup).unwrap();
    let view = build_view(&files, &setup).unwrap();

    assert!(!view.traces.is_empty());
    // brazil year 1 level from gdxdump = 23.2200031991411
    let brazil_devel = view
        .traces
        .iter()
        .find(|t| t.name.contains("brazil") && t.name.contains("devel"))
        .expect("trace for brazil/devel not found");
    let idx = brazil_devel
        .x
        .iter()
        .position(|x| x == "1")
        .expect("time period '1' not found");
    assert!(
        (brazil_devel.y[idx] - 23.2200031991411_f64).abs() < 1e-6,
        "TEMP_REGION brazil/devel year 1 mismatch: got {}",
        brazil_devel.y[idx]
    );
}

#[test]
#[ignore = "requires gdx_examples/ (large files)"]
fn all_four_files_share_ykali() {
    let files: Vec<LoadedFile> = [
        "results_ssp2_bau_devel.gdx",
        "results_ssp2_bau_master.gdx",
        "results_ssp2_curpol_devel.gdx",
        "results_ssp2_curpol_master.gdx",
    ]
    .iter()
    .map(|n| load(n))
    .collect();

    let common = common_symbols(&files);
    let names: Vec<&str> = common.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"ykali"),
        "ykali not common across all 4 files"
    );

    let mut setup = DisplaySetup::for_symbol("ykali");
    setup.x_dim = 0;
    setup.series_dim = Some(1);

    // refine_setup → 1 region; 4 files × 1 region = 4 traces.
    let setup = refine_setup(&files, &setup).unwrap();
    let view = build_view(&files, &setup).unwrap();
    assert_eq!(view.traces.len(), 4);
}
