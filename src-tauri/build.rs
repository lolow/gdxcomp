fn main() {
    // Must run before tauri_build, which fails if a bundle resource is missing.
    stage_gdxcclib_dll();
    tauri_build::build();
    emit_gdxcclib_rpath();
}

/// Windows has no rpath concept; the DLL must sit alongside the executable.
///
/// The DLL is copied to a fixed, profile-independent path (`bundled/`) that
/// `tauri.windows.conf.json` lists as a resource. Tauri then places it next to
/// the exe both for `tauri dev` (in `target/<profile>`) and in the installers.
fn stage_gdxcclib_dll() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() != "windows" {
        return;
    }

    let libdir = std::env::var("DEP_GDXCCLIB64_LIBDIR").unwrap();
    let dll = std::path::Path::new(&libdir).join("gdxcclib64.dll");
    let staged = std::path::Path::new("bundled");
    std::fs::create_dir_all(staged).unwrap();
    std::fs::copy(&dll, staged.join("gdxcclib64.dll")).unwrap();
    println!("cargo:rerun-if-changed={}", dll.display());
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
            return;
        }
    }
}
