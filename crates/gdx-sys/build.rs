//! Builds the vendored GAMS-dev/gdx C library (`libgdxcclib64`) with CMake and
//! emits the link directives so downstream crates can call the C API directly.
//!
//! The produced shared library is self-contained (it bundles its own zlib) and
//! does **not** require a GAMS installation at runtime.

use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest.join("third_party/gdx");

    if !src.join("CMakeLists.txt").exists() {
        panic!(
            "GAMS-dev/gdx submodule missing at {}.\n\
             Run: git submodule update --init --recursive",
            src.display()
        );
    }

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    // Build only the C-API shared library target; skip tests/examples/tools.
    // CMAKE_POLICY_VERSION_MINIMUM is needed so CMake 4.x accepts the bundled
    // zlib's very old `cmake_minimum_required`.
    let mut cfg = cmake::Config::new(&src);
    cfg.define("NO_TESTS", "ON")
        .define("NO_EXAMPLES", "ON")
        .define("NO_TOOLS", "ON")
        .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5");

    // When cross-compiling for Windows from Linux, point cmake at MinGW.
    if target_os == "windows" && cfg!(target_os = "linux") {
        let prefix = match target_arch.as_str() {
            "x86_64" => "x86_64-w64-mingw32",
            "i686" => "i686-w64-mingw32",
            _ => "x86_64-w64-mingw32",
        };
        cfg.define("CMAKE_SYSTEM_NAME", "Windows")
            .define("CMAKE_C_COMPILER", format!("{prefix}-gcc"))
            .define("CMAKE_CXX_COMPILER", format!("{prefix}-g++"))
            .define("CMAKE_RC_COMPILER", format!("{prefix}-windres"));
    }

    let dst = cfg.build_target("gdxcclib64").build();

    // With `build_target`, artifacts land in the CMake binary dir (`<dst>/build`).
    let libdir = dst.join("build");

    println!("cargo:rustc-link-search=native={}", libdir.display());
    println!("cargo:rustc-link-lib=dylib=gdxcclib64");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match target_os.as_str() {
        "linux" => {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libdir.display());
            // $ORIGIN = directory containing the executable (ELF rpath token).
            println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
        }
        "macos" => {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libdir.display());
            // @loader_path = directory containing the loading binary (macOS equivalent).
            println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");
        }
        _ => {
            // Windows: DLL must reside alongside the executable; no rpath concept.
        }
    }

    // Expose the directory holding the shared library to dependent crates as
    // DEP_GDXCCLIB64_LIBDIR (via the `links` key) so the app can bundle it.
    println!("cargo:libdir={}", libdir.display());

    println!(
        "cargo:rerun-if-changed={}",
        src.join("CMakeLists.txt").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        src.join("generated/gdxcclib.cpp").display()
    );
}
