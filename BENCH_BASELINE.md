# gdxcomp benchmark baselines

Numbers captured per phase of the performance sweep
(see `/home/lolow/.claude/plans/i-would-like-to-smooth-twilight.md`).
Each row records criterion median wall-clock and short context. Commits should
append rows, never overwrite — drift over time is more informative than the
latest point.

Hardware reference: Fedora 44 / Linux 7.0.9 / Intel Xe iGPU. Compiled with
`[profile.bench] inherits = "release"` (so lto="fat", codegen-units=1,
opt-level=3, strip="symbols").

## Rust core benches (`cargo bench -p gdxcomp-core --bench load_and_view`)

| phase | bench | median | note |
|---|---|---|---|
| 0 | open_metadata | 3.249 ms | single 18 MB file, metadata only |
| 0 | read_records_ykali | 2.686 ms | 2-dim parameter |
| 0 | build_view_ykali | 3.002 ms | refined setup, 1 file |
| 0 | refine_and_build_view_ykali | 2.705 ms | refine + build, 1 file |
| 0 | open_metadata_4files | 13.96 ms | 4 files in gdx_examples/ |
| 0 | open_metadata_19files | 56.51 ms | 4 + 15 files (~278 MB total) |
| 0 | read_records_largest_symbol | 247.4 ms | picked `ABAT_CLASS` from ssp2_bau_devel |
| 0 | distinct_keys_dim0_ykali | 2.230 ms | proves O(N²) Vec::contains is small here |
| 0 | distinct_keys_dim0_19files | 39.14 ms | multi-file accumulator (`Vec::contains` x2) |
| 1.1 | distinct_keys_dim0_ykali | 2.087 ms | HashSet+Vec; -6% on low-K |
| 1.1 | distinct_keys_dim0_19files | 39.81 ms | unchanged (outer accumulator dominates) |
| 1.5 | (no change to load_and_view benches) | — | name_index targets common_symbols path |
| 2.1-3 | open_metadata | 3.019 ms | -7% (noise) |
| 2.1-3 | read_records_ykali | 60.87 µs | **−97.8% (45× faster)** — cache hit after warmup |
| 2.1-3 | build_view_ykali | 207.0 µs | **−91.5% (14.5×)** still has Vec<Rec> clone; 2.4 closes the gap |
| 2.1-3 | refine_and_build_view_ykali | 201.6 µs | **−92.9%** |
| 2.1-3 | open_metadata_4files | 16.82 ms | +20% — Mutex<LruCache> alloc per open |
| 2.1-3 | open_metadata_19files | 72.15 ms | +28% — same cause |
| 2.1-3 | read_records_largest_symbol | 200.4 ms | -19% — limited by Vec<Rec> clone of large symbol |
| 2.1-3 | distinct_keys_dim0_ykali | 68.80 µs | **−96.8% (32×)** |
| 2.1-3 | distinct_keys_dim0_19files | 1.284 ms | **−96.8% (30×)** |
| 2.1-3 | build_view_aggregated_4files | 800.5 µs | **−92%** (vs 10.58 ms baseline) |
| 2.4 | read_records_ykali | 37.33 µs | -38% vs 2.1-3 (Arc reused) |
| 2.4 | build_view_ykali | 164.0 µs | -20% vs 2.1-3 |
| 2.4 | refine_and_build_view_ykali | 176.0 µs | -18% vs 2.1-3 |
| 2.4 | read_records_largest_symbol | 203.9 ms | unchanged (public API still clones; Phase 3a may help) |
| 2.4 | distinct_keys_dim0_ykali | 8.856 µs | **−87% vs 2.1-3, −99.6% vs Phase 0 (252× faster overall)** |
| 2.4 | distinct_keys_dim0_19files | 183.4 µs | **−85% vs 2.1-3, −99.5% vs Phase 0 (213× faster overall)** |
| 2.4 | build_view_aggregated_4files | 574.2 µs | -28% vs 2.1-3 |
| 2.4 | build_view_2dim_aggregated_4files | 12.62 µs | **−99.9% vs Phase 0 (710× faster on cached path)** |
| 0 | build_view_aggregated_4files | 10.58 ms | dim_agg=Sum on non-x dim |
| 0 | build_view_2dim_aggregated_4files | 8.962 ms | picked `allerr` (dim=3), 2 agg dims |

## IPC-loopback benches (`cargo bench -p gdxcomp-core --bench ipc_loopback`)

| phase | bench | median | note |
|---|---|---|---|
| 0 | ipc_common_symbols_4files | 29.36 ms | clone files + intersect + serde_json |
| 0 | ipc_common_symbols_19files | 286.8 ms | the clone chain dominates |
| 0 | ipc_distinct_keys_4files | 11.11 ms | per-file scan + dedupe + JSON |
| 0 | ipc_get_view_4files | 16.08 ms | refine + build + JSON of full PlotView |
| 1.5 | ipc_common_symbols_4files | 4.785 ms | name_index O(1); **−83.7%** |
| 1.5 | ipc_common_symbols_19files | 27.99 ms | name_index O(1); **−90.2% (10× faster)** |
| 1.5 | ipc_distinct_keys_4files | 12.23 ms | within noise |
| 1.5 | ipc_get_view_4files | 16.76 ms | within noise |
| 5.1 | ipc_get_view_4files | 5.480 ms | -67% from cumulative cache + name_index |
| 5.1 | ipc_get_chart_view_4files | 3.770 ms | **−31% vs ipc_get_view_4files (chart-only payload)** |
| 5.1 | ipc_get_table_view_4files | 3.830 ms | table-only payload, fetched lazily on tab switch |
| 5.1 | ipc_distinct_keys_4files | 37.28 µs | -99.7% (cache + name_index in action) |
| 5.1 | ipc_common_symbols_19files | 25.11 ms | -91% vs Phase-0 |

## Frontend bundle (`./scripts/bundle-size.sh`)

| phase | asset | bytes | note |
|---|---|---|---|
| 0 | dist/assets/index-*.js | 4,856,322 | plotly.js-dist-min, ≈ 93% of bundle |
| 0 | dist/assets/index-*.css | 10,958 | |
| 0 | dist total | ~4.7 MB | |

---

## How to fill these in

```sh
cargo bench -p gdxcomp-core --bench load_and_view
cargo bench -p gdxcomp-core --bench ipc_loopback
./scripts/bundle-size.sh
```

Criterion writes detailed HTML to `target/criterion/*/report/index.html`.
The number that lives here is the **median** reported on the summary line
(middle value of the `[lo med hi]` triple).

---

## Phase-0 observations

- `read_records_largest_symbol` (247 ms for `ABAT_CLASS`) is the slowest hot
  path; per-record `Vec<String>` allocation in `reader.rs:194` is the main
  suspect → Phase 3a target.
- `ipc_common_symbols_19files` (287 ms) is dominated by the `Vec<LoadedFile>`
  clone of 19 files × 100+ SymbolMeta each → Phase 1.5 (name index) helps
  symbol lookup; metadata snapshot path could also stop cloning.
- `distinct_keys_dim0_19files` (39 ms) vs `distinct_keys_dim0_ykali` (2.2 ms)
  shows the multi-file accumulator's outer `Vec::contains` is the heavier
  knob → Phase 1.1 fixes both layers.
- `build_view_aggregated_4files` (10.6 ms) — Phase 1.2 (`IndexSet` x_order)
  and Phase 1.3 (`IndexMap` FileGroup) will move the needle once symbol size
  grows; `ykali` per-file is small so the O(N²) is not yet dominant.

### Phase 1.1 note

The plan predicted "p50 drop ≥ 5× on symbols with >1k distinct keys" — neither
`ykali` (~30 distinct years) nor any cheap-to-bench symbol in the corpus hits
that K. The HashSet+Vec change is algorithmically correct (O(N²)→O(N)) and
shows a small (-6%) win on `ykali`; the >5× win would only materialize on a
high-K symbol (e.g. one with thousands of distinct UELs in a dim). Outer
multi-file accumulator (`out.contains(&k)` in `commands.rs:308` and in the
bench) is a separate O(K²) layer not addressed by Phase 1.1 and is what makes
`_19files` net-flat in the table above.

### Phase 3a deferred (no measurable target)

Cold-cache FFI per-record cost (`buf_to_string` × dim × records) is what
Phase 3a targets. But after Phase 2 the cache absorbs ~all repeat reads,
and the bench `read_records_largest_symbol` (200 ms) is dominated by the
**deep `Vec<Rec>` clone** in the public `read_records()` API — not by FFI.
All internal callers use `read_records_arc()` (no clone), so the FFI
cost only shows up on first-symbol-load latency, which the current
benches can't isolate. Will revisit if a first-paint-latency bench
appears (e.g. via a Tauri startup-time hyperfine).

### Phase 1.2 deferred (not applied)

Tried `IndexSet<String>` for `x_order` in `build_view`:
`build_view_aggregated_4files` 10.58 → 13.22 ms (+25%), regression.
Tried `HashSet<String>` + `Vec<String>` parallel:
`build_view_aggregated_4files` 10.58 → 10.82 ms (+3%, within noise);
`build_view_2dim_aggregated_4files` 8.96 → 10.10 ms (+13%, real).

Both variants regress at the corpus's low K (~30 distinct year values per
x-dim). The Vec::contains is faster than any hash structure at K≈30 because
the linear scan fits in L1 cache and string compares of short year labels are
~5 ns each. The algorithmic O(N²)→O(N) win only materializes at K ≳ 200,
which our corpus doesn't have. Phase 1.2 reverted; will revisit if a high-K
build_view workload appears.
