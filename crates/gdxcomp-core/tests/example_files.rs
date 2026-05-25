//! Integration tests against the example IAM result files in gdx_examples/.
//! Run with:
//!   cargo test -p gdxcomp-core --test example_files -- --ignored

use gdxcomp_core::{
    build_view, common_symbols, refine_setup, DimAgg, DisplaySetup, Field, LoadedFile,
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
    let t = std::time::Instant::now();
    let file = load("results_ssp2_bau_devel.gdx");
    let elapsed = t.elapsed();
    assert!(
        elapsed.as_secs() < 5,
        "metadata open took {elapsed:?}, expected <5 s"
    );
    assert!(file.symbols.len() > 100);
}

#[test]
#[ignore = "requires gdx_examples/ (large files)"]
fn bau_files_share_common_symbols() {
    let devel = load("results_ssp2_bau_devel.gdx");
    let master = load("results_ssp2_bau_master.gdx");
    let common = common_symbols(&[devel, master]);
    assert!(common.len() > 100);
    let names: Vec<&str> = common.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"ykali"));
    assert!(names.contains(&"TEMP_REGION"));
}

#[test]
#[ignore = "requires gdx_examples/ (large files)"]
fn refine_setup_defaults_non_x_dims_to_sum() {
    let devel = load("results_ssp2_bau_devel.gdx");
    let master = load("results_ssp2_bau_master.gdx");
    let files = vec![devel, master];

    let mut setup = DisplaySetup::for_symbol("ykali");
    setup.x_dim = 0; // year; dim 1 = region

    let refined = refine_setup(&files, &setup).unwrap();

    // dim 1 (region) must be auto-defaulted to sum aggregation.
    assert_eq!(
        refined.dim_agg.get(&1).copied(),
        Some(DimAgg::Sum),
        "refine_setup must default non-x dim to sum"
    );
    // x-axis must be limited to first 5 periods.
    let x_filter = refined.filters.get(&0).expect("x-axis filter must be set");
    assert_eq!(x_filter.len(), 5);
}

#[test]
#[ignore = "requires gdx_examples/ (large files)"]
fn overlay_ykali_across_bau_files() {
    let devel = load("results_ssp2_bau_devel.gdx");
    let master = load("results_ssp2_bau_master.gdx");
    let files = vec![devel, master];

    let mut setup = DisplaySetup::for_symbol("ykali");
    setup.x_dim = 0;

    let setup = refine_setup(&files, &setup).unwrap();
    let view = build_view(&files, &setup).unwrap();

    // One trace per file.
    assert_eq!(view.traces.len(), 2);
    assert!(view.traces.iter().any(|t| t.name.contains("devel")));
    assert!(view.traces.iter().any(|t| t.name.contains("master")));
}

#[test]
#[ignore = "requires gdx_examples/ (large files)"]
fn overlay_temp_region_variable_level() {
    let devel = load("results_ssp2_bau_devel.gdx");
    let master = load("results_ssp2_bau_master.gdx");
    let files = vec![devel, master];

    let mut setup = DisplaySetup::for_symbol("TEMP_REGION");
    setup.x_dim = 0;
    setup.field = Field::Level;

    let setup = refine_setup(&files, &setup).unwrap();
    let view = build_view(&files, &setup).unwrap();

    assert_eq!(view.traces.len(), 2);
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
    assert!(common.iter().any(|s| s.name == "ykali"));

    let mut setup = DisplaySetup::for_symbol("ykali");
    setup.x_dim = 0;

    let setup = refine_setup(&files, &setup).unwrap();
    let view = build_view(&files, &setup).unwrap();
    // One trace per file.
    assert_eq!(view.traces.len(), 4);
}
