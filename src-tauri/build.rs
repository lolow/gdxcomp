fn main() {
    tauri_build::build();
    emit_gdxcclib_rpath();
}

/// Emit rpath linker args so the gdxcomp binary can find libgdxcclib64.so
/// at runtime without requiring LD_LIBRARY_PATH.
///
/// cargo:rustc-link-arg from a *dependency's* build.rs is not propagated to
/// the final binary's linker (Cargo limitation). Emitting it here — from the
/// application's own build.rs — actually reaches the linker.
///
/// Two rpaths are emitted:
///  1. The absolute build-cache directory (for `cargo run` / running in-place).
///  2. $ORIGIN (for a bundled .so placed next to the installed binary).
fn emit_gdxcclib_rpath() {
    // OUT_DIR = .../target/<profile>/build/<crate>-<hash>/out
    // Three levels up → .../target/<profile>
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let Some(profile_dir) = out_dir.ancestors().nth(3) else {
        return;
    };

    // Search build artefacts for the gdx-sys output directory.
    let build_dir = profile_dir.join("build");
    let Ok(entries) = std::fs::read_dir(&build_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let candidate = entry.path().join("out/build/libgdxcclib64.so");
        if candidate.exists() {
            let libdir = entry.path().join("out/build");
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libdir.display());
            println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
            return;
        }
    }
}
