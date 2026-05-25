//! Validation against a real GAMS-produced file (`trnsport.gdx`), whose values
//! are known from `gdxdump`. Ignored by default so the suite needs no GAMS
//! install; run with:
//!   GDX_TEST_FILE=/opt/gams40/apifiles/GAMS/trnsport.gdx \
//!     cargo test -p gdx --test oracle -- --ignored

use gdx::{GdxFile, SymbolType, ValueField};

fn open_sample() -> GdxFile {
    let path = std::env::var("GDX_TEST_FILE")
        .expect("set GDX_TEST_FILE to a trnsport.gdx path to run this test");
    GdxFile::open(path).expect("open sample")
}

#[test]
#[ignore = "requires trnsport.gdx via GDX_TEST_FILE"]
fn symbol_table_matches_gdxdump() {
    let file = open_sample();
    assert_eq!(file.symbols().len(), 12);

    let c = file.symbol("c").expect("parameter c");
    assert_eq!(c.kind, SymbolType::Parameter);
    assert_eq!(c.dim, 2);
    assert_eq!(c.records, 6);

    let x = file.symbol("x").expect("variable x");
    assert_eq!(x.kind, SymbolType::Variable);
    assert_eq!(x.dim, 2);
}

#[test]
#[ignore = "requires trnsport.gdx via GDX_TEST_FILE"]
fn parameter_values_match_gdxdump() {
    let file = open_sample();
    let c = file.read("c").unwrap();
    let find = |a: &str, b: &str| {
        c.iter()
            .find(|r| r.keys == vec![a.to_string(), b.to_string()])
            .map(|r| r.value(ValueField::Level))
            .expect("record present")
    };
    let approx = |got: f64, want: f64| assert!((got - want).abs() < 1e-9, "{got} != {want}");
    approx(find("seattle", "new-york"), 0.225);
    approx(find("seattle", "chicago"), 0.153);
    approx(find("san-diego", "topeka"), 0.126);
}

#[test]
#[ignore = "requires trnsport.gdx via GDX_TEST_FILE"]
fn special_eps_maps_to_zero() {
    // gdxdump shows the supply equation marginal for 'seattle' is Eps,
    // which our wrapper maps to 0.0.
    let file = open_sample();
    let supply = file.read("supply").unwrap();
    let seattle = supply
        .iter()
        .find(|r| r.keys == vec!["seattle".to_string()])
        .expect("seattle supply record");
    assert_eq!(seattle.value(ValueField::Marginal), 0.0);
}
