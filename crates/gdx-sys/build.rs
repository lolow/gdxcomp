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

    // Build only the C-API shared library target; skip tests/examples/tools.
    // CMAKE_POLICY_VERSION_MINIMUM is needed so CMake 4.x accepts the bundled
    // zlib's very old `cmake_minimum_required`.
    let dst = cmake::Config::new(&src)
        .define("NO_TESTS", "ON")
        .define("NO_EXAMPLES", "ON")
        .define("NO_TOOLS", "ON")
        .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
        .build_target("gdxcclib64")
        .build();

    // With `build_target`, artifacts land in the CMake binary dir (`<dst>/build`).
    let libdir = dst.join("build");

    println!("cargo:rustc-link-search=native={}", libdir.display());
    println!("cargo:rustc-link-lib=dylib=gdxcclib64");

    // Make the library discoverable when running tests / `cargo run` in dev,
    // where the .so lives in the build directory rather than next to the binary.
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libdir.display());
    // And next to the executable, for a bundled app that ships the .so alongside.
    println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");

    // Expose the directory holding libgdxcclib64.so to dependent crates as
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
