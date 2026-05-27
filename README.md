# gdxcomp

A small desktop app to **plot and compare GAMS GDX files** — load several GDX
files (e.g. scenario runs of the same model), pick a symbol, and overlay each
file as its own series with filtering, dimension mapping, and a data table.

![icon](assets/icon-source.png)

## What it does

- **Select** one or more `.gdx` files (or a folder) to compare.
- **Pick a symbol** — the picker lists only symbols that are comparable across
  *all* loaded files (same name, dimension, and kind). Sets are excluded.
- **Map & filter**: choose which index dimension goes on the x-axis; pick the
  value field (Level / Marginal / Lower / Upper / Scale for Variables &
  Equations); filter UELs per dimension or aggregate (sum / mean) over
  remaining dimensions.
- **Plot** as a line chart with markers; switch to the **data table** view.
- **Scenario names** are auto-derived from file stems (common prefix/suffix
  stripped); rename any scenario individually or reset all to defaults.
- **Session restore** — the last set of open files and selected symbol are
  persisted and offered for restore on the next launch.

### WITCH mode

When a GAMS-WITCH model GDX is detected (auto or via the mode toggle), extra
features activate:

- The time dimension `t` is mapped to calendar years via the `year(t)`
  parameter, or the fallback formula `2000 + 5 × val(t)`.
- The x-axis defaults to `t` (shown as **year(t)**) and uses a numeric linear
  scale.
- The year filter becomes a **range slider** (min / max year) instead of a
  checkbox list.

GDX is read through the official, MIT-licensed
[GAMS-dev/gdx](https://github.com/GAMS-dev/gdx) library, vendored and built
from source. **No GAMS installation is required** to run the app.

## Architecture

A Cargo workspace of focused crates plus a Tauri 2 + React shell:

| Crate / dir | Role |
|---|---|
| `crates/gdx-sys` | Raw FFI bindings; `build.rs` compiles the vendored GDX C++ library with CMake. |
| `crates/gdx` | Safe RAII wrapper (`GdxFile`, `GdxWriter`); serializes all FFI calls behind a global lock (the library is not thread-safe). |
| `crates/gdxcomp-core` | UI-independent logic: file model, `common_symbols`, `build_view`, `DisplaySetup`, `YearMapper`. |
| `src-tauri` | Tauri 2 backend: caches loaded files in app state, exposes typed commands. |
| `src/` | React + TypeScript + `react-plotly.js` frontend. |

The core crates form one workspace (testable without any GUI dependency);
`src-tauri` is a separate workspace so the GUI's system requirements don't
block core development.

## Prerequisites

- **Rust** ≥ 1.80 and **Cargo**.
- **CMake** and a **C++17 compiler** (to build the vendored GDX library).
- **Node** ≥ 18 and **npm**.
- **Tauri Linux deps**. On Fedora:

  ```sh
  sudo dnf install webkit2gtk4.1-devel javascriptcoregtk4.1-devel \
                   libsoup3-devel librsvg2-devel
  ```

  See the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for
  other platforms.

## Getting started

```sh
git clone --recurse-submodules https://github.com/lolow/gdxcomp
cd gdxcomp
# if you already cloned without --recurse-submodules:
git submodule update --init --recursive

npm install
npm run tauri dev      # builds the Rust backend + starts the app
```

To produce a distributable bundle:

```sh
npm run tauri build
```

> **Linux / Intel Xe GPU**: if the app crashes on startup with
> `free(): corrupted unsorted chunks`, disable WebKit's DMA-BUF renderer:
> ```sh
> WEBKIT_DISABLE_DMABUF_RENDERER=1 npm run tauri dev
> ```

## Testing

```sh
# Rust core (no GAMS install needed)
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check

# Optional: compare reader output against gdxdump on a real file
GDX_TEST_FILE=/opt/gams40/apifiles/GAMS/trnsport.gdx \
  cargo test -p gdxcomp-core --test real_file -- --ignored

# Frontend
npm run typecheck
npm test
```

## Releases

Pre-built Linux binaries (`.deb` and `.rpm`) are available on the
[Releases page](https://github.com/lolow/gdxcomp/releases).
`libgdxcclib64.so` is bundled alongside the executable via an `$ORIGIN` rpath.

## License

MIT. Bundled GDX sources (`crates/gdx-sys/third_party/gdx`) are © GAMS
Software GmbH / GAMS Development Corp., also MIT.
