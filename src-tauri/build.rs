fn main() {
    // Must run before tauri_build, which fails if a bundle resource is missing.
    stage_gdxcclib();
    tauri_build::build();
    emit_gdxcclib_rpath();
}

/// Copies the GDX shared library to a fixed, profile-independent path
/// (`bundled/`) that the platform bundle configs reference.
///
/// Windows has no rpath concept: `tauri.windows.conf.json` lists the DLL as a
/// resource, and Tauri places it next to the exe both for `tauri dev` (in
/// `target/<profile>`) and in the installers.
///
/// Linux: `tauri.linux.conf.json` installs the .so to `/usr/lib/gdxcomp/` in
/// the .deb/.rpm, which the `$ORIGIN/../lib/gdxcomp` rpath points at. Without
/// it the packages only run where the build cache still exists.
fn stage_gdxcclib() {
    let lib_filename = match std::env::var("CARGO_CFG_TARGET_OS")
        .unwrap_or_default()
        .as_str()
    {
        "windows" => "gdxcclib64.dll",
        "linux" => "libgdxcclib64.so",
        _ => return,
    };

    let libdir = std::env::var("DEP_GDXCCLIB64_LIBDIR").unwrap();
    let lib = std::path::Path::new(&libdir).join(lib_filename);
    let staged = std::path::Path::new("bundled");
    std::fs::create_dir_all(staged).unwrap();
    std::fs::copy(&lib, staged.join(lib_filename)).unwrap();
    println!("cargo:rerun-if-changed={}", lib.display());
}

/// Emit rpath linker args so the gdxcomp binary can find the GDX shared
/// library at runtime without requiring LD_LIBRARY_PATH / DYLD_LIBRARY_PATH.
///
/// cargo:rustc-link-arg from a *dependency's* build.rs is not propagated to
/// the final binary's linker (Cargo limitation). Emitting it here — from the
/// application's own build.rs — actually reaches the linker.
///
/// Rpaths emitted:
///  1. The absolute build-cache directory (for `cargo run` / running in-place).
///  2. An origin-relative token ($ORIGIN on Linux, @loader_path on macOS) for
///     a bundled library placed next to the installed binary.
///  3. Linux only: `$ORIGIN/../lib/gdxcomp`, where the .deb/.rpm install the
///     library (`/usr/bin/gdxcomp` → `/usr/lib/gdxcomp/`).
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
            if target_os == "linux" {
                println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib/gdxcomp");
            }
            return;
        }
    }
}
