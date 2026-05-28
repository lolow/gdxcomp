# Changelog

All notable changes are documented here. Versions follow [semver](https://semver.org/).

---

## [Unreleased]

---

## [0.0.9] — 2026-05-28

### Fixed
- Chart x-axis (and y-axis on the second toggle) could render against a
  stale Plotly axis-range cache after switching units — the axis would
  drop to `-1..6` instead of the actual year range. The `<Plot>`
  element now uses a React `key` derived from `(symbol, unit,
  conversionFactor)`, forcing a full remount on unit change so Plotly
  reinitializes cleanly. Both axes also pin `autorange: true`.

---

## [0.0.8] — 2026-05-28

### Added
- Unit conversion `GtC ↔ GtCO2e` (×44/12) for carbon mass — applies whenever
  the unit contains `GtC` (word-boundary regex; `GtCe`/`GtCO2`/`GtCH4` are
  unaffected).
- Unit conversion `GTonC ↔ GtCO2e` (×44/12) for the verbose form
  (case-insensitive).
- Unit conversion `GtCe ↔ GtCO2e` (×44/12) — always applicable when the
  unit contains `GtCe`, independent of any e-dimension filter.
- Multiple unit conversions may now stack as toolbar buttons. Example:
  with `e=co2*` filter + `GtCe/yr`, the toolbar shows
  `GtCe/yr | GtCO2e/yr | Gt/yr` (each carries its own factor).
- WITCH mode now auto-snaps the x-axis to the `t` (year) dimension when
  WITCH is activated or auto-detected, if the current symbol has a `t`
  domain. Matches the behaviour already in `selectSymbol` for symbol
  changes.
- About dialog now shows the release codename and date inline with the
  version: `gdxcomp 0.0.8 (birding — 2026-05-28)`.

### Fixed
- `@testing-library/dom` is now an explicit dev dependency, fixing the
  Intel-Mac `npm run tauri build` failure reported in #1 (TS2305 on the
  `screen` re-export from RTL v16).

---

## [0.0.7] — 2026-05-28

### Performance sweep

Measured improvements (see `BENCH_BASELINE.md` for full numbers). All
changes are bench-justified; deferred items are documented inline.

#### Backend (Rust)
- Tuned `[profile.release]` (lto=fat, codegen-units=1, opt-level=3,
  strip=symbols) for both workspaces; `[profile.bench]` inherits release.
- `LoadedFile::distinct_keys` switched to `HashSet<&str> + Vec` (O(N²) → O(N)
  in the inner loop).
- O(1) symbol lookup: `name_index: HashMap<String, usize>` on `LoadedFile`
  and `GdxFile`. `ipc_common_symbols_19files`: **287 ms → 25 ms (11×)**.
- Bounded LRU record cache (`crates/gdxcomp-core/src/cache.rs`,
  `Mutex<LruCache<String, Arc<Vec<Rec>>>>`) shared via `Arc<RecordCache>`
  across `LoadedFile` clones. Capacity from `GDXCOMP_RECORD_CACHE_SIZE`
  (default 32). `build_view_aggregated_4files`: **10.58 ms → 574 µs (18×)**.
- `build_view`, `distinct_keys`, `YearMapper::new`, `read_param_map` now
  iterate `Arc<Vec<Rec>>` directly — no per-call deep clone.
- New `build_chart` / `build_table` + commands `get_chart_view` /
  `get_table_view`. Chart-tab IPC payload **−31%** (3.77 ms vs 5.48 ms).

#### Frontend
- Switched `plotly.js-dist-min` → `plotly.js-basic-dist-min`. Bundle **4.86 MB
  → 1.30 MB (−73%)**.
- Vite `manualChunks` + `React.lazy(ChartView)` + Suspense. Main-entry JS
  **4.86 MB → 159 kB (30×)** for first paint; Plotly fetched in parallel
  when the chart tab activates.
- Lazy table fetch on tab switch (was: every setup change sent the full
  table). Cached client-side; invalidated on setup change.
- Skip chart refetch on mode-only toggle when the current symbol has no
  `t` dimension.
- Client-side `distinctKeys` cache by `(symbol, dim)`, invalidated when
  the loaded-file set changes.
- Stable `DataTable` row keys so sort/filter doesn't churn React fibers.

#### Tooling
- Added `BENCH_BASELINE.md` with criterion median wall-clock for every
  bench; rows appended per phase, never overwritten.
- New `crates/gdxcomp-core/benches/ipc_loopback.rs` mirrors the Tauri
  command bodies (including `serde_json::to_string`).
- Extended `load_and_view.rs` with multi-file and pathological cases
  (4 / 19 files, largest-symbol, distinct-keys 19-files, aggregated builds).
- `scripts/bundle-size.sh` captures the frontend baseline.

---

## [0.0.6] — 2026-05-28

### Added
- About dialog (click "gdxcomp" in the header): shows version, author, and
  pixel art witch hat easter egg.

---

## [0.0.5] — 2026-05-27

### Added
- Unit conversion `T$/GtonCe → $/tCO2` (factor 1000×12/44) for carbon price
  symbols; toggle appears when unit matches `T$/GtonC*` pattern.
- "Remove all files" button in the file modal.
- Energy unit conversion `TWh ↔ EJ` available in all non-GDX modes.
- Y-axis title shows extracted unit `[…]` in all modes.

### Fixed
- Carbon price conversion regex enforces `T$` before `GtonC` to avoid false
  match on inverted ratios.
- Unit conversion toggles restricted to non-GDX modes.

---

## [0.0.4] — 2026-05-27

### Added
- WITCH mode: CH4 unit conversion `GtCe/yr → Mt/yr` using GWP from `emi_gwp`
  parameter (AR4 default = 25 if parameter absent).
- WITCH mode: N2O unit conversion `GtCe/yr → Mt/yr` using GWP from `emi_gwp`
  parameter (AR4 default = 298 if parameter absent).
- Energy unit conversion toggle `TWh ↔ EJ` for any symbol whose unit contains
  `TWh` (factor 3.6/1000); available in all modes.
- New backend command `read_param_map` to read any 1-dim parameter as a
  key→value map.
- Y-axis title shows the symbol unit in all modes (extracted from `[…]` in
  description).

### Fixed
- CH4 conversion now correctly applies the C→CO2 factor (44/12) before
  dividing by GWP.

---

## [0.0.3] — 2026-05-27

### Added
- WITCH mode: unit badge in plot toolbar showing unit extracted from `[]` in
  symbol description. Only visible for level / lower / upper fields.
- WITCH mode: CO2 unit conversion toggle `GtCe/yr → Gt/yr` (factor 44/12)
  when `e` dimension is filtered to `co2*` values.
- Session persistence — last open files and selected symbol are saved to the
  app data directory and offered for restore on next launch.

### Fixed
- WITCH mode activated on session restore (stale `mode` state no longer passed
  to `selectSymbol`).
- Year mapping now correctly gated on `mode = witch` in backend; switching to
  GDX mode restores raw UEL labels on x-axis.

---

## [0.0.2] — 2026-05-26

### Added
- WITCH mode: auto-detection from symbol list (`Q`, `Q_EMI`, `Q_FUEL`, `I`,
  `I_EN`); manual toggle in header bar.
- WITCH mode: `t` dimension mapped to calendar years via `year(t)` parameter
  or fallback formula `2000 + 5×val(t)`; linear numeric x-axis; x-axis title
  shows "year"; selector shows "year(t)".
- WITCH mode: year range slider (dual-thumb) for the `t` x-axis filter.
- Symbol title header showing name, description (hover tooltip), and active
  filter chips (`dim: value`, `dim: sum/mean`, field).
- GDX/WITCH mode field `mode: AppMode` added to `DisplaySetup` so the backend
  gates year mapping correctly.
- Scenario names: auto-computed by stripping common prefix/suffix from file
  stems; editable per-file; "Reset names" button restores defaults.
- Sets excluded from the symbol picker.
- Filter key lists sorted alphabetically.
- Release workflow simplified: CI-only on tag push; binaries built and uploaded
  locally.

### Fixed
- Range slider: transparent input tracks so the min thumb is never obscured by
  the max input's track.
- Sidebar collapse: columns set to `0px` / `display:none` when collapsed.
- Aggregated table rows now fill all dim columns correctly.

---

## [0.0.1] — 2026-05-26

### Added
- Initial release.
- Load one or more GDX files or folders; compare common symbols across files.
- Symbol picker with collapsible groups and kind badges (Parameter, Variable,
  Equation, Set).
- Mapping panel: x-axis dimension selector; value field selector
  (Level/Marginal/Lower/Upper/Scale for Variables & Equations).
- Filter panel: per-dimension UEL multi-select (x-dim) or single-select /
  sum / mean (non-x dims).
- Line chart with markers via `react-plotly.js`; 0-intercept toggle.
- Data table with sortable columns and per-column text filters.
- Chart/Table tab toggle; loading spinner overlay.
- Auto-refresh 400 ms after any setup change (no explicit Update button).
- Collapsible left (files/symbols) and right (mapping/filters) sidebars with
  SVG layout icons.
- File modal: scenario rename, file removal.
- Open folder loads all `.gdx` files in a directory.
- Command-line argument support: pass file/folder paths on launch.
- `rpath` wiring so `libgdxcclib64.so` is found next to the executable.
- `.deb` and `.rpm` Linux bundles.
