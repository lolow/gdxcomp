# gdxcomp

A small desktop app to **plot and compare GAMS GDX files** — load several GDX
files (e.g. scenario runs of the same model), pick a symbol, and overlay each
file as its own series with filtering, dimension mapping, and a data table.
Display setups are saved/loaded as JSON.

![icon](assets/icon-source.png)

## What it does

- **Select** one or more `.gdx` files to compare.
- **Pick a symbol** (the picker lists symbols comparable across *all* loaded
  files — same name, dimension and kind).
- **Map & filter**: choose which index dimension goes on the x-axis, optionally
  split into series, pick the value field (Level/Marginal/Lower/Upper/Scale for
  Variables & Equations), filter UELs per dimension, and aggregate over the
  remaining dimensions.
- **Plot** as a line or grouped-bar chart, with the **data table** alongside.
- **Import/export** the display setup as JSON.

GDX is read through the official, MIT-licensed
[GAMS-dev/gdx](https://github.com/GAMS-dev/gdx) library, vendored and built from
source. **No GAMS installation is required** to run the app.

## Architecture

A Cargo workspace of focused crates plus a Tauri 2 + React shell:

| Crate / dir | Role |
|---|---|
| `crates/gdx-sys` | Raw FFI bindings; `build.rs` compiles the vendored GDX C library with CMake. |
| `crates/gdx` | Safe RAII wrapper (`GdxFile`, `GdxWriter`); serializes all FFI behind a global lock (the C library is not thread-safe). |
| `crates/gdxcomp-core` | UI-independent logic: file model, `common_symbols`, `build_view`, and the `DisplaySetup` JSON schema. |
| `src-tauri` | Tauri 2 backend: caches loaded files in state and exposes commands. |
| `src/` | React + TypeScript + `react-plotly.js` frontend. |

The core crates form one workspace (testable without any GUI dependency);
`src-tauri` is a separate workspace so the GUI's system requirements don't block
core development.

## Prerequisites

- **Rust** ≥ 1.80 and **Cargo**.
- **CMake** and a **C++17 compiler** (to build the vendored GDX library).
- **Node** ≥ 18 and **npm**.
- **Tauri Linux deps** (to build/run the desktop app). On Fedora:

  ```sh
  sudo dnf install webkit2gtk4.1-devel javascriptcoregtk4.1-devel \
                   libsoup3-devel librsvg2-devel
  ```

  (See the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for
  other platforms.)

## Getting started

```sh
git clone --recurse-submodules <repo-url>
cd gdxcomp
# if you already cloned without --recurse-submodules:
git submodule update --init --recursive

npm install
npm run tauri dev      # launches the app (builds the Rust backend + frontend)
```

To produce a distributable bundle:

```sh
npm run tauri build
```

## Testing

```sh
# Rust core (no GAMS install needed; uses self-written GDX fixtures)
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check

# Optional: validate the reader against a real GAMS file
GDX_TEST_FILE=/opt/gams40/apifiles/GAMS/trnsport.gdx \
  cargo test -p gdx --test oracle -- --ignored

# Frontend
npm run typecheck
npm test
```

## Notes & limitations

- Files are read **fully into memory** on load; very large GDX files will use
  proportional RAM.
- GDX special values are mapped to `f64`: Undefined/NA → `NaN` (shown as `—`),
  ±Infinity → ±∞, EPS → `0`.
- The FFI binds the library's lowercase `c__gdx*` symbols (the Linux/macOS
  export convention).
- For a bundled release, `libgdxcclib64.so` must ship next to the executable
  (the binary carries an `$ORIGIN` rpath); wiring it into the Tauri bundle's
  resources is a follow-up.

## License

MIT. Bundled GDX sources (`crates/gdx-sys/third_party/gdx`) are © GAMS Software
GmbH / GAMS Development Corp., also MIT.
