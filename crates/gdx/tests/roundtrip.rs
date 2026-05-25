//! Self-contained round-trip: write a GDX file with the writer, read it back,
//! and assert structure and values. Requires no external data or GAMS install.

use gdx::{GdxFile, GdxWriter, Record, SymbolType, ValueField};

fn rec(keys: &[&str], values: [f64; 5]) -> Record {
    Record {
        keys: keys.iter().map(|s| s.to_string()).collect(),
        values,
    }
}

fn write_fixture(path: &std::path::Path) {
    let mut w = GdxWriter::create(path, "gdxcomp-test").unwrap();

    // Set i (dim 1): two elements. Set element "value" is conventionally 0.
    w.write_symbol(
        "i",
        "plants",
        1,
        SymbolType::Set,
        0,
        &[rec(&["seattle"], [0.0; 5]), rec(&["san-diego"], [0.0; 5])],
    )
    .unwrap();

    // Parameter c (dim 2): the value lives in the Level (index 0) slot.
    w.write_symbol(
        "c",
        "transport cost",
        2,
        SymbolType::Parameter,
        0,
        &[
            rec(&["seattle", "new-york"], [0.225, 0.0, 0.0, 0.0, 0.0]),
            rec(&["seattle", "chicago"], [0.153, 0.0, 0.0, 0.0, 0.0]),
            rec(&["san-diego", "topeka"], [0.126, 0.0, 0.0, 0.0, 0.0]),
        ],
    )
    .unwrap();

    // Variable x (dim 1): carries all five fields.
    w.write_symbol(
        "x",
        "shipment",
        1,
        SymbolType::Variable,
        0,
        &[rec(&["seattle"], [50.0, 1.5, 0.0, 1e30, 1.0])],
    )
    .unwrap();

    w.finish().unwrap();
}

#[test]
fn write_then_read_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.gdx");
    write_fixture(&path);

    let file = GdxFile::open(&path).unwrap();

    // Symbol table.
    let names: Vec<&str> = file.symbols().iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["i", "c", "x"]);

    let set_i = file.symbol("i").unwrap();
    assert_eq!(set_i.kind, SymbolType::Set);
    assert_eq!(set_i.dim, 1);
    assert_eq!(set_i.records, 2);

    let par_c = file.symbol("c").unwrap();
    assert_eq!(par_c.kind, SymbolType::Parameter);
    assert_eq!(par_c.dim, 2);
    assert_eq!(par_c.records, 3);
    assert_eq!(par_c.text, "transport cost");

    let var_x = file.symbol("x").unwrap();
    assert_eq!(var_x.kind, SymbolType::Variable);
    assert!(var_x.kind.has_fields());

    // Parameter values (Level slot).
    let c = file.read("c").unwrap();
    assert_eq!(c.len(), 3);
    assert_eq!(c[0].keys, vec!["seattle", "new-york"]);
    assert!((c[0].value(ValueField::Level) - 0.225).abs() < 1e-12);
    assert!((c[2].value(ValueField::Level) - 0.126).abs() < 1e-12);

    // Variable fields.
    let x = file.read("x").unwrap();
    assert_eq!(x.len(), 1);
    assert_eq!(x[0].keys, vec!["seattle"]);
    assert!((x[0].value(ValueField::Level) - 50.0).abs() < 1e-12);
    assert!((x[0].value(ValueField::Marginal) - 1.5).abs() < 1e-12);
}

#[test]
fn missing_symbol_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fixture.gdx");
    write_fixture(&path);

    let file = GdxFile::open(&path).unwrap();
    let err = file.read("does-not-exist").unwrap_err();
    assert!(matches!(err, gdx::GdxError::SymbolNotFound(_)));
}

#[test]
fn open_nonexistent_errors() {
    let err = GdxFile::open("/nonexistent/path/to/file.gdx")
        .err()
        .expect("opening a missing file should fail");
    assert!(matches!(err, gdx::GdxError::OpenRead { .. }));
}
