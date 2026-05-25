//! End-to-end pipeline test against a real GAMS file (the same code path the
//! Tauri backend runs). Ignored by default; run with:
//!   GDX_TEST_FILE=/opt/gams40/apifiles/GAMS/trnsport.gdx \
//!     cargo test -p gdxcomp-core --test real_file -- --ignored

use gdxcomp_core::{build_view, common_symbols, DisplaySetup, Field, LoadedFile};

fn load() -> LoadedFile {
    let path = std::env::var("GDX_TEST_FILE")
        .expect("set GDX_TEST_FILE to a trnsport.gdx path to run this test");
    LoadedFile::open(path).expect("open real gdx")
}

#[test]
#[ignore = "requires trnsport.gdx via GDX_TEST_FILE"]
fn loads_and_lists_real_symbols() {
    let file = load();
    let names: Vec<&str> = file.symbols.iter().map(|s| s.name.as_str()).collect();
    for expected in ["i", "j", "a", "c", "d", "x", "z"] {
        assert!(names.contains(&expected), "missing symbol {expected}");
    }
    // With a single file every symbol is trivially "common".
    assert_eq!(
        common_symbols(std::slice::from_ref(&file)).len(),
        file.symbols.len()
    );
}

#[test]
#[ignore = "requires trnsport.gdx via GDX_TEST_FILE"]
fn builds_overlay_view_for_parameter_c() {
    let file = load();
    let files = vec![file];

    let mut setup = DisplaySetup::for_symbol("c");
    setup.x_dim = 0; // plants
    let view = build_view(&files, &setup).unwrap();

    // Markets: new-york, chicago, topeka -> one series each.
    assert_eq!(view.traces.len(), 3);
    let ny = view
        .traces
        .iter()
        .find(|t| t.name.ends_with("new-york"))
        .unwrap();
    let i = ny.x.iter().position(|x| x == "seattle").unwrap();
    assert!((ny.y[i] - 0.225).abs() < 1e-9);
}

#[test]
#[ignore = "requires trnsport.gdx via GDX_TEST_FILE"]
fn variable_field_selection_works() {
    let file = load();
    let files = vec![file];

    let mut setup = DisplaySetup::for_symbol("x"); // variable, dim 2
    setup.x_dim = 0;
    setup.field = Field::Level;
    let level = build_view(&files, &setup).unwrap();
    assert!(!level.traces.is_empty());

    // Switching the field must change the view without error.
    setup.field = Field::Marginal;
    let marginal = build_view(&files, &setup).unwrap();
    assert_eq!(marginal.field, Field::Marginal);
}
