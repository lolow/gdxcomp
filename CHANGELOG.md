# Changelog

All notable changes are documented here. Versions follow [semver](https://semver.org/).

---

## [Unreleased]

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
