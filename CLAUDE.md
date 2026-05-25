# gdxcomp — CLAUDE.md

Desktop app to **plot and compare GAMS GDX files** across multiple scenario runs.
Stack: Tauri 2 (Rust backend) + React 18 + TypeScript + react-plotly.js.

---

## Workspace layout

```
gdxcomp/
  Cargo.toml              # root workspace: crates/gdx-sys, crates/gdx, crates/gdxcomp-core
  crates/
    gdx-sys/              # raw FFI to vendored GAMS-dev/gdx (MIT C++17 lib)
      third_party/gdx/    # git submodule — GAMS-dev/gdx
      build.rs            # cmake build → libgdxcclib64.so; links lowercase c__gdx* symbols
      src/lib.rs          # hand-written FFI declarations (no bindgen at build time)
    gdx/                  # safe RAII wrapper; global Mutex in lock.rs (lib is not thread-safe)
    gdxcomp-core/         # pure domain logic: model, compare, filter, setup — no GUI deps
      src/{model,compare,setup,error,lib,view}.rs
      tests/{roundtrip,real_file,example_files,oracle}.rs
  src-tauri/              # Tauri 2 app — its OWN workspace (excluded from root)
    src/{lib,commands,main}.rs
  src/                    # React + Vite frontend
    components/           # FileBar, SymbolPicker, MappingPanel, FilterPanel,
                          # ChartView, DataTable, SetupToolbar
    api.ts, types.ts, App.tsx
  gdx_examples/           # real IAM result GDX files for manual/ignored testing
  .github/workflows/ci.yml
```

> **Why two workspaces?** `src-tauri` is excluded from the root workspace so
> `cargo test --workspace` works without `webkit2gtk` (no GUI deps needed for
> core logic tests).

---

## Commands

### Rust core (no GUI deps required)
```sh
cargo fmt --check                        # format check
cargo clippy --workspace --all-targets -- -D warnings   # must be warning-free
cargo test --workspace                   # all unit + integration tests
```

### Real-file tests (require opt/gams40 or gdx_examples/)
```sh
# trnsport.gdx from GAMS 40 install
GDX_TEST_FILE=/opt/gams40/apifiles/GAMS/trnsport.gdx \
  cargo test -p gdxcomp-core --test real_file -- --ignored

# Large IAM example files (~60 s)
cargo test -p gdxcomp-core --test example_files -- --ignored
```

### Frontend
```sh
npm run typecheck    # tsc --noEmit
npm test             # vitest run
npm run build        # tsc + vite build
```

### Tauri app
```sh
npm run tauri dev    # dev server (Vite + cargo run)
cargo build --manifest-path src-tauri/Cargo.toml   # compile backend only
```

---

## Environment

| Thing | Detail |
|---|---|
| Package manager | **`npm`** (`/usr/bin/npm`). `pnpm` is in the user's fish shell but NOT on the bash PATH — always use `npm`. |
| GAMS install | `/opt/gams40` — `gdxdump` and `trnsport.gdx` for dev/test only. Not a runtime dependency. |
| GDX library | Vendored `GAMS-dev/gdx` built from source via CMake; no GAMS install needed at runtime. |
| Fedora deps | `webkit2gtk4.1-devel javascriptcoregtk4.1-devel libsoup3-devel librsvg2-dev` |

---

## Architecture decisions (non-obvious)

**FFI binding strategy** — link `libgdxcclib64.so` directly and call lowercase
`c__gdx*` exported symbols (e.g. `c__gdxopenread`, `c__gdxdatareadstr`).  Do
NOT use the `gdxcc.c` dlopen wrapper or `gdxCreateD` (requires a GAMS system
dir).  `gdxcreate`/`gdxfree` use no system dir.

**Thread safety** — the library is built with `-DGC_NO_MUTEX` and is NOT
thread-safe.  Every FFI call acquires the process-global `Mutex` in
`crates/gdx/src/lock.rs`.  `Drop` on `GdxFile`/`GdxWriter` null-checks the
handle before locking to avoid double-lock.

**Special values** — GDX sentinels (EPS, NA, Undef, ±Inf) are read via
`c__gdxgetspecialvalues` and mapped: `EPS → 0.0`, `NA/Undef → f64::NAN`,
`PINF → f64::INFINITY`, `MINF → f64::NEG_INFINITY`.

**State ownership** — Tauri backend owns all `LoadedFile` state in
`Mutex<Vec<LoadedFile>>`; the React frontend is a pure view.  IPC types in
`types.ts` mirror Rust serde structs with `camelCase` rename.

**`gdxcomp-core` is the test boundary** — domain logic has no Tauri/GUI
dependency so it can be unit-tested with plain `cargo test`.

---

## Testing strategy

| Layer | How |
|---|---|
| `gdx-sys` | Compiled by `cargo test`; FFI link errors surface immediately |
| `gdx` | Self-contained write+read round-trip (`tests/roundtrip.rs`) |
| `gdxcomp-core` | Unit tests in `src/`; integration tests in `tests/` |
| Real files | Ignored by default; require `GDX_TEST_FILE` or local `gdx_examples/` |
| Frontend | Vitest (`npm test`); React Testing Library for components |
| CI | `rust-core` + `frontend` + `app` jobs in `.github/workflows/ci.yml` |

---

## Coding conventions

- **Clippy must pass** with `-D warnings` before every commit.
- **`cargo fmt`** is enforced in CI; run it before committing.
- **No comments** explaining what code does — only why (hidden constraints,
  workarounds, subtle invariants).
- **No speculative abstractions** — if a trait/enum/helper has one use site,
  keep it inline until a second use appears.
- **Errors via `thiserror`** — `GdxError` in `crates/gdx/src/error.rs`,
  `CoreError` in `crates/gdxcomp-core/src/error.rs`.
- **serde rename** — all IPC structs use `#[serde(rename_all = "camelCase")]`
  to match TypeScript conventions.
- **TypeScript** — `types.ts` is the single source of truth for IPC shapes;
  `api.ts` contains only typed `invoke` wrappers, no logic.

---

## Development rules

> Full reference: `.rules`. The condensed form below is what matters day-to-day.

### Think before coding
- State assumptions before writing: which crate, who owns state, sync vs async.
- When a request has multiple valid interpretations, name them — don't pick silently.
- Push back if a simpler approach exists.

### Simplicity first (Karpathy)
- Minimum code that solves the problem. Nothing speculative.
- No trait objects, generics, or helpers for a single call site.
- Three similar lines is better than a premature abstraction.
- No `async` unless the operation blocks; Tauri commands are sync here.

### Surgical changes (Karpathy)
- Touch only what the request requires. Don't reformat adjacent lines.
- Match the style of surrounding code.
- Every changed line must trace directly to the stated request.

### TDD workflow (Karpathy)
- Bug fix: write a failing test first, then fix, then `cargo test --workspace`.
- New feature: tests fail → implement minimum → tests pass → clippy clean.
- Refactor: tests green before AND after; no behaviour change = no test change.

### Git
- `cargo test --workspace` + `npm test` must pass before committing.
- One logical change per commit. Format: `type(scope): imperative summary`.
- Types: `feat`, `fix`, `refactor`, `test`, `chore`, `docs`.
- Never force-push `main`. Never `--no-verify` without documenting why.
