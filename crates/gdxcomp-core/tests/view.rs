//! Core logic tests over self-written GDX fixtures (no GAMS install needed).

use std::path::{Path, PathBuf};

use gdx::{GdxWriter, Record, SymbolType};
use gdxcomp_core::{
    build_view, common_symbols, Aggregation, ChartKind, DisplaySetup, Field, LoadedFile, SymbolKind,
};
use tempfile::TempDir;

fn par(keys: &[&str], v: f64) -> Record {
    Record {
        keys: keys.iter().map(|s| s.to_string()).collect(),
        values: [v, 0.0, 0.0, 0.0, 0.0],
    }
}

/// Write a two-plant / two-market scenario file. `scale` multiplies costs and
/// `extra` adds a symbol unique to this file (to exercise `common_symbols`).
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
    // "onlyA"/"onlyB" are excluded; result is sorted.
    assert_eq!(names, vec!["a", "c", "i"]);
}

#[test]
fn labels_come_from_file_stems() {
    let (_d, files) = two_files();
    assert_eq!(files[0].label, "base");
    assert_eq!(files[1].label, "scen");
}

#[test]
fn overlay_each_file_as_series_with_series_dim() {
    let (_d, files) = two_files();
    let mut setup = DisplaySetup::for_symbol("c");
    setup.x_dim = 0; // plants
    setup.series_dim = Some(1); // markets
    let view = build_view(&files, &setup).unwrap();

    assert_eq!(view.kind, SymbolKind::Parameter);
    assert_eq!(view.x_label, "Dim1"); // domains are "*" in fixtures
                                      // 2 files x 2 markets = 4 traces.
    assert_eq!(view.traces.len(), 4);

    let base_ny = trace(&view, "base / new-york");
    assert_eq!(base_ny.x, vec!["seattle", "san-diego"]);
    assert!((base_ny.y[0] - 0.225).abs() < 1e-12);
    assert!((base_ny.y[1] - 0.225).abs() < 1e-12);

    // Scenario file is scaled x2.
    let scen_chi = trace(&view, "scen / chicago");
    assert!((scen_chi.y[0] - 0.306).abs() < 1e-12); // 0.153 * 2
    assert!((scen_chi.y[1] - 0.324).abs() < 1e-12); // 0.162 * 2
}

#[test]
fn aggregation_collapses_unmapped_dimension() {
    let (_d, files) = two_files();
    let mut setup = DisplaySetup::for_symbol("c");
    setup.x_dim = 0; // plants; market dim is unmapped -> aggregated
    setup.series_dim = None;
    setup.aggregate = Aggregation::Sum;
    let view = build_view(&files, &setup).unwrap();

    // One trace per file.
    assert_eq!(view.traces.len(), 2);
    let base = trace(&view, "base");
    // seattle: 0.225 + 0.153 ; san-diego: 0.225 + 0.162
    assert!((base.y[0] - 0.378).abs() < 1e-12);
    assert!((base.y[1] - 0.387).abs() < 1e-12);

    setup.aggregate = Aggregation::Mean;
    let view = build_view(&files, &setup).unwrap();
    let base = trace(&view, "base");
    assert!((base.y[0] - 0.189).abs() < 1e-12); // (0.225+0.153)/2
}

#[test]
fn filters_restrict_records() {
    let (_d, files) = two_files();
    let mut setup = DisplaySetup::for_symbol("c");
    setup.x_dim = 0;
    setup.series_dim = None;
    setup.filters.insert(1, vec!["new-york".to_string()]); // only NY market
    let view = build_view(&files, &setup).unwrap();

    let base = trace(&view, "base");
    assert!((base.y[0] - 0.225).abs() < 1e-12); // seattle->NY only
    assert!((base.y[1] - 0.225).abs() < 1e-12); // san-diego->NY only
                                                // Table holds the filtered records only: 2 markets removed -> 2 rows/file.
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
    let mut setup = DisplaySetup::for_symbol("a"); // dim 1
    setup.x_dim = 5;
    let err = build_view(&files, &setup).unwrap_err();
    assert!(matches!(err, gdxcomp_core::CoreError::DimOutOfRange { .. }));
}

#[test]
fn display_setup_json_roundtrips() {
    let mut setup = DisplaySetup::for_symbol("c");
    setup.x_dim = 0;
    setup.series_dim = Some(1);
    setup.field = Field::Marginal;
    setup.chart = ChartKind::Bar;
    setup.aggregate = Aggregation::Mean;
    setup.filters.insert(0, vec!["seattle".to_string()]);
    setup.files = vec![PathBuf::from("a.gdx"), PathBuf::from("b.gdx")];

    let json = setup.to_json().unwrap();
    let back = DisplaySetup::from_json(&json).unwrap();
    assert_eq!(setup, back);
}

#[test]
fn nonfinite_values_serialize_as_null() {
    // A gap (missing x cell for a series) becomes NaN -> JSON null, which
    // Plotly renders as a break in the line.
    let (_d, files) = two_files();
    let mut setup = DisplaySetup::for_symbol("c");
    setup.x_dim = 1; // markets on x
    setup.series_dim = Some(0); // plants as series
    setup.filters.insert(0, vec!["seattle".to_string()]); // keep only seattle
    let view = build_view(&files, &setup).unwrap();
    let json = serde_json::to_string(&view).unwrap();
    // No NaN tokens leak into JSON.
    assert!(!json.contains("NaN"));
}
