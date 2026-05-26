fn main() {
    tauri_build::build();
    emit_gdxcclib_rpath();
}

/// Emit rpath linker args so the gdxcomp binary can find the GDX shared
/// library at runtime without requiring LD_LIBRARY_PATH / DYLD_LIBRARY_PATH.
///
/// cargo:rustc-link-arg from a *dependency's* build.rs is not propagated to
/// the final binary's linker (Cargo limitation). Emitting it here — from the
/// application's own build.rs — actually reaches the linker.
///
/// Two rpaths are emitted:
///  1. The absolute build-cache directory (for `cargo run` / running in-place).
///  2. An origin-relative token ($ORIGIN on Linux, @loader_path on macOS) for
///     a bundled library placed next to the installed binary.
///
/// Windows has no rpath concept; the DLL must be alongside the executable.
fn emit_gdxcclib_rpath() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    let (lib_filename, origin_token) = match target_os.as_str() {
        "linux" => ("libgdxcclib64.so", "$ORIGIN"),
        "macos" => ("libgdxcclib64.dylib", "@loader_path"),
        _ => return,
    };

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
        let candidate = entry.path().join(format!("out/build/{lib_filename}"));
        if candidate.exists() {
            let libdir = entry.path().join("out/build");
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libdir.display());
            println!("cargo:rustc-link-arg=-Wl,-rpath,{origin_token}");
            // On Windows, copy the DLL next to the executable so Tauri bundles it.
            if target_os == "windows" {
                let dll = entry.path().join("out/build/gdxcclib64.dll");
                if dll.exists() {
                    let _ = std::fs::copy(&dll, profile_dir.join("gdxcclib64.dll"));
                }
            }
            return;
        }
    }
}
