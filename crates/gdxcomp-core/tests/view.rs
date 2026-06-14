//! Core logic tests over self-written GDX fixtures (no GAMS install needed).

use std::path::{Path, PathBuf};

use std::sync::Arc;

use gdx::{GdxWriter, Record, SymbolType};
use gdxcomp_core::{
    build_chart, build_table, build_view, common_symbols, DimAgg, DisplaySetup, Field, LoadedFile,
    SymbolKind,
};
use tempfile::TempDir;

fn par(keys: &[&str], v: f64) -> Record {
    Record {
        keys: keys.iter().map(|s| Arc::from(*s)).collect(),
        values: [v, 0.0, 0.0, 0.0, 0.0],
    }
}

fn write_scenario(path: &Path, scale: f64, extra: &str) {
    let mut w = GdxWriter::create(path, "test").unwrap();
    w.write_symbol(
        "i",
        "plants",
        1,
        SymbolType::Set,
        0,
        &[par(&["seattle"], 0.0), par(&["san-diego"], 0.0)],
    )
    .unwrap();
    w.write_symbol(
        "c",
        "cost",
        2,
        SymbolType::Parameter,
        0,
        &[
            par(&["seattle", "new-york"], 0.225 * scale),
            par(&["seattle", "chicago"], 0.153 * scale),
            par(&["san-diego", "new-york"], 0.225 * scale),
            par(&["san-diego", "chicago"], 0.162 * scale),
        ],
    )
    .unwrap();
    w.write_symbol(
        "a",
        "capacity",
        1,
        SymbolType::Parameter,
        0,
        &[par(&["seattle"], 350.0), par(&["san-diego"], 600.0)],
    )
    .unwrap();
    w.write_symbol(
        extra,
        "unique",
        1,
        SymbolType::Parameter,
        0,
        &[par(&["x"], 1.0)],
    )
    .unwrap();
    w.finish().unwrap();
}

fn two_files() -> (TempDir, Vec<LoadedFile>) {
    let dir = tempfile::tempdir().unwrap();
    let a: PathBuf = dir.path().join("base.gdx");
    let b: PathBuf = dir.path().join("scen.gdx");
    write_scenario(&a, 1.0, "onlyA");
    write_scenario(&b, 2.0, "onlyB");
    let files = vec![LoadedFile::open(&a).unwrap(), LoadedFile::open(&b).unwrap()];
    (dir, files)
}

fn trace<'a>(view: &'a gdxcomp_core::PlotView, name: &str) -> &'a gdxcomp_core::Trace {
    view.traces
        .iter()
        .find(|t| t.name == name)
        .unwrap_or_else(|| panic!("no trace {name}"))
}

#[test]
fn common_symbols_is_the_intersection() {
    let (_d, files) = two_files();
    let names: Vec<String> = common_symbols(&files).into_iter().map(|s| s.name).collect();
    assert_eq!(names, vec!["a", "c", "i"]);
}

#[test]
fn labels_come_from_file_stems() {
    let (_d, files) = two_files();
    assert_eq!(files[0].label, "base");
    assert_eq!(files[1].label, "scen");
}

#[test]
fn one_trace_per_file_with_market_summed() {
    let (_d, files) = two_files();
    let mut setup = DisplaySetup::for_symbol("c");
    setup.x_dim = 0; // plants
    setup.dim_agg.insert(1, DimAgg::Sum); // sum over markets

    let view = build_view(&files, &setup).unwrap();

    assert_eq!(view.kind, SymbolKind::Parameter);
    assert_eq!(view.x_label, "Dim1");
    assert_eq!(view.traces.len(), 2); // one per file

    let base = trace(&view, "base");
    assert!(base
        .x
        .iter()
        .zip(["seattle", "san-diego"])
        .all(|(a, b)| a == b));
    assert!((base.y[0] - 0.378).abs() < 1e-12); // 0.225 + 0.153
    assert!((base.y[1] - 0.387).abs() < 1e-12); // 0.225 + 0.162

    let scen = trace(&view, "scen");
    assert!((scen.y[0] - 0.756).abs() < 1e-12); // ×2
}

#[test]
fn one_trace_per_file_no_agg_all_records() {
    let (_d, files) = two_files();
    let mut setup = DisplaySetup::for_symbol("c");
    setup.x_dim = 0;
    // No dim_agg, no filter on dim 1 → all 4 records appear per file.
    let view = build_view(&files, &setup).unwrap();

    assert_eq!(view.traces.len(), 2);
    let base = trace(&view, "base");
    assert_eq!(base.x.len(), 4);
    assert_eq!(base.y.len(), 4);
}

#[test]
fn filters_restrict_records() {
    let (_d, files) = two_files();
    let mut setup = DisplaySetup::for_symbol("c");
    setup.x_dim = 0;
    setup.filters.insert(1, vec!["new-york".to_string()]);
    let view = build_view(&files, &setup).unwrap();

    let base = trace(&view, "base");
    assert!((base.y[0] - 0.225).abs() < 1e-12);
    assert!((base.y[1] - 0.225).abs() < 1e-12);
    assert_eq!(view.table.len(), 4);
}

#[test]
fn missing_symbol_is_an_error() {
    let (_d, files) = two_files();
    let setup = DisplaySetup::for_symbol("nope");
    let err = build_view(&files, &setup).unwrap_err();
    assert!(matches!(err, gdxcomp_core::CoreError::SymbolMissing(_)));
}

#[test]
fn x_dim_out_of_range_is_an_error() {
    let (_d, files) = two_files();
    let mut setup = DisplaySetup::for_symbol("a");
    setup.x_dim = 5;
    let err = build_view(&files, &setup).unwrap_err();
    assert!(matches!(err, gdxcomp_core::CoreError::DimOutOfRange { .. }));
}

#[test]
fn display_setup_json_roundtrips() {
    let mut setup = DisplaySetup::for_symbol("c");
    setup.x_dim = 0;
    setup.field = Field::Marginal;
    setup.dim_agg.insert(1, DimAgg::Mean);
    setup.filters.insert(0, vec!["seattle".to_string()]);
    setup.files = vec![PathBuf::from("a.gdx"), PathBuf::from("b.gdx")];

    let json = setup.to_json().unwrap();
    let back = DisplaySetup::from_json(&json).unwrap();
    assert_eq!(setup, back);
}

#[test]
fn build_chart_traces_match_build_view_agg() {
    let (_d, files) = two_files();
    let mut setup = DisplaySetup::for_symbol("c");
    setup.x_dim = 0;
    setup.dim_agg.insert(1, DimAgg::Sum);
    let view = build_view(&files, &setup).unwrap();
    let chart = build_chart(&files, &setup).unwrap();
    assert_eq!(chart.traces, view.traces);
    assert_eq!(chart.dim_names, view.dim_names);
}

#[test]
fn build_chart_traces_match_build_view_no_agg() {
    let (_d, files) = two_files();
    let mut setup = DisplaySetup::for_symbol("c");
    setup.x_dim = 0;
    let view = build_view(&files, &setup).unwrap();
    let chart = build_chart(&files, &setup).unwrap();
    assert_eq!(chart.traces, view.traces);
    assert_eq!(chart.dim_names, view.dim_names);
}

#[test]
fn build_table_matches_build_view_no_agg() {
    let (_d, files) = two_files();
    let mut setup = DisplaySetup::for_symbol("c");
    setup.x_dim = 0;
    let view = build_view(&files, &setup).unwrap();
    let tbl = build_table(&files, &setup).unwrap();
    assert_eq!(tbl.table, view.table);
    assert_eq!(tbl.dim_names, view.dim_names);
}

#[test]
fn build_table_matches_build_view_agg() {
    let (_d, files) = two_files();
    let mut setup = DisplaySetup::for_symbol("c");
    setup.x_dim = 0;
    setup.dim_agg.insert(1, DimAgg::Sum);
    let view = build_view(&files, &setup).unwrap();
    let tbl = build_table(&files, &setup).unwrap();
    assert_eq!(tbl.table, view.table);
}

#[test]
fn nonfinite_values_serialize_as_null() {
    // NaN (missing x in aggregated trace) must not appear in JSON output.
    let (_d, files) = two_files();
    let mut setup = DisplaySetup::for_symbol("c");
    setup.x_dim = 0;
    setup.dim_agg.insert(1, DimAgg::Sum);
    let view = build_view(&files, &setup).unwrap();
    let json = serde_json::to_string(&view).unwrap();
    assert!(!json.contains("NaN"));
}
