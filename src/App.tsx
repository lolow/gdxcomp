import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api } from "./api";
import { AboutDialog } from "./components/AboutDialog";
import { DataTable } from "./components/DataTable";

// Lazy-load ChartView so Plotly's ~1 MB chunk is only fetched the first
// time the chart tab is shown.
const ChartView = lazy(() =>
  import("./components/ChartView").then((m) => ({ default: m.ChartView })),
);
import { FileBar } from "./components/FileBar";
import { FilterPanel } from "./components/FilterPanel";
import { MappingPanel } from "./components/MappingPanel";
import { SymbolPicker } from "./components/SymbolPicker";
import type {
  AppMode,
  ChartView as ChartViewData,
  DisplaySetup,
  FileMeta,
  Session,
  SymbolMeta,
  TableView as TableViewData,
} from "./types";
import { defaultSetup } from "./types";

const WITCH_SYMBOLS = new Set(["Q", "Q_EMI", "Q_FUEL", "I", "I_EN"]);
const UNIT_FIELDS = new Set(["level", "lower", "upper"]);

function extractUnit(text: string): string | null {
  const matches = text.match(/\[([^\]]+)\]/g);
  if (!matches) return null;
  return matches[matches.length - 1].slice(1, -1);
}

function detectMode(syms: SymbolMeta[]): AppMode {
  if (WITCH_SYMBOLS.size > 0 && syms.some((s) => WITCH_SYMBOLS.has(s.name))) return "witch";
  return "gdx";
}

export function App() {
  const [files, setFiles] = useState<FileMeta[]>([]);
  const [symbols, setSymbols] = useState<SymbolMeta[]>([]);
  const [setup, setSetup] = useState<DisplaySetup | null>(null);
  const [chartView, setChartView] = useState<ChartViewData | null>(null);
  const [tableView, setTableView] = useState<TableViewData | null>(null);
  const [loading, setLoading] = useState(false);
  const [tab, setTab] = useState<"chart" | "table">("chart");
  const [showZero, setShowZero] = useState(true);
  const [leftOpen, setLeftOpen] = useState(true);
  const [rightOpen, setRightOpen] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [aboutOpen, setAboutOpen] = useState(false);
  const [mode, setMode] = useState<AppMode>("gdx");
  const [savedSession, setSavedSession] = useState<Session | null>(null);
  const [unitChoice, setUnitChoice] = useState<string | null>(null);
  const [emiGwp, setEmiGwp] = useState<Record<string, number>>({});

  const syncFromBackend = useCallback(async () => {
    const f = await api.listFiles();
    setFiles(f);
    const syms = (await api.commonSymbols()).filter((s) => s.kind !== "set");
    setSymbols(syms);
    const detected = detectMode(syms);
    if (detected === "witch") setMode("witch");
  }, []);

  // On mount: load persisted session and offer to restore if no files are open.
  useEffect(() => {
    api.loadSession().then((s) => { if (s && s.files.length > 0) setSavedSession(s); }).catch(() => {});
    syncFromBackend().catch((e) => setError(String(e)));
  }, [syncFromBackend]);

  // Load emi_gwp parameter map whenever WITCH files are open.
  useEffect(() => {
    if (files.length === 0 || mode !== "witch") { setEmiGwp({}); return; }
    api.readParamMap("emi_gwp").then(setEmiGwp).catch(() => {});
  }, [files, mode]);

  // Persist session whenever the file list or selected symbol changes.
  useEffect(() => {
    if (files.length === 0) return;
    api.saveSession({ files: files.map((f) => f.path), lastSymbol: setup?.symbol ?? null }).catch(() => {});
  }, [files, setup?.symbol]);

  const currentSymbol = useMemo(
    () => (setup ? symbols.find((s) => s.name === setup.symbol) ?? null : null),
    [symbols, setup],
  );

  // When mode transitions GDX → WITCH (either via the manual toggle or via
  // auto-detection in syncFromBackend), snap the x-axis to the "t"
  // dimension if the current symbol has one. selectSymbol() already does
  // this on symbol selection; this effect covers the case where a symbol
  // is already loaded when WITCH mode is found/activated.
  const prevModeRef = useRef(mode);
  useEffect(() => {
    const prev = prevModeRef.current;
    prevModeRef.current = mode;
    if (prev === "witch" || mode !== "witch" || !currentSymbol || !setup) return;
    const tIdx = currentSymbol.domains.indexOf("t");
    if (tIdx >= 0 && setup.xDim !== tIdx) {
      patchSetup({ xDim: tIdx });
    }
  }, [mode, currentSymbol]); // eslint-disable-line react-hooks/exhaustive-deps

  // Lazy table fetch: only when the user is on the table tab AND we don't
  // have a cached TableView for the current setup. Cleared by the chart
  // refresh effect whenever setup changes.
  useEffect(() => {
    if (tab !== "table" || tableView || !setup?.symbol || files.length === 0) return;
    let cancelled = false;
    api
      .getTableView(setup)
      .then((tv) => {
        if (!cancelled) setTableView(tv);
      })
      .catch((e) => {
        if (!cancelled) setError(String(e));
      });
    return () => {
      cancelled = true;
    };
  }, [tab, setup, files, tableView]);

  // Auto-refresh chart 400 ms after any setup or file change.
  // Invalidates the cached table view so the next tab switch refetches.
  // Skips refetch when only `mode` changed and the symbol has no `t` dim
  // (mode only affects year mapping on `t`).
  const prevSetupRef = useRef<DisplaySetup | null>(null);
  useEffect(() => {
    if (!setup?.symbol || files.length === 0) {
      setChartView(null);
      setTableView(null);
      setLoading(false);
      prevSetupRef.current = setup;
      return;
    }
    const prev = prevSetupRef.current;
    prevSetupRef.current = setup;
    if (
      prev &&
      currentSymbol &&
      !currentSymbol.domains.includes("t") &&
      prev.mode !== setup.mode &&
      prev.symbol === setup.symbol &&
      prev.field === setup.field &&
      prev.xDim === setup.xDim &&
      JSON.stringify(prev.filters) === JSON.stringify(setup.filters) &&
      JSON.stringify(prev.dimAgg) === JSON.stringify(setup.dimAgg)
    ) {
      return;
    }
    let cancelled = false;
    setTableView(null);
    const timer = setTimeout(() => {
      setLoading(true);
      api
        .getChartView(setup)
        .then(({ view: v }) => {
          if (!cancelled) {
            setChartView(v);
            setError(null);
            setLoading(false);
          }
        })
        .catch((e) => {
          if (!cancelled) {
            setChartView(null);
            setError(String(e));
            setLoading(false);
          }
        });
    }, 400);
    return () => {
      cancelled = true;
      clearTimeout(timer);
      setLoading(false);
    };
  }, [setup, files]); // eslint-disable-line react-hooks/exhaustive-deps

  async function handleOpen(paths: string[]) {
    try {
      await api.openGdx(paths);
      await syncFromBackend();
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleOpenFolder(path: string) {
    try {
      await api.openFolder(path);
      await syncFromBackend();
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleRemove(path: string) {
    try {
      await api.removeGdx(path);
      await syncFromBackend();
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleClearFiles() {
    try {
      await api.clearFiles();
      setFiles([]);
      setSetup(null);
      setChartView(null);
      setTableView(null);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleRename(path: string, scenario: string) {
    try {
      const updated = await api.renameScenario(path, scenario);
      setFiles(updated);
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleResetScenarios() {
    try {
      const updated = await api.resetScenarios();
      setFiles(updated);
    } catch (e) {
      setError(String(e));
    }
  }

  async function restoreSession() {
    if (!savedSession) return;
    setSavedSession(null);
    try {
      await api.openGdx(savedSession.files);
      const f = await api.listFiles();
      setFiles(f);
      const syms = (await api.commonSymbols()).filter((s) => s.kind !== "set");
      setSymbols(syms);
      const detectedMode = detectMode(syms);
      setMode(detectedMode);
      if (savedSession.lastSymbol) selectSymbol(savedSession.lastSymbol, detectedMode);
    } catch (e) {
      setError(String(e));
    }
  }

  // Symbols sorted so the last-used one (from session) appears first.
  const sortedSymbols = useMemo(() => {
    if (!savedSession?.lastSymbol || setup) return symbols;
    const last = savedSession.lastSymbol;
    return [...symbols].sort((a, b) => (a.name === last ? -1 : b.name === last ? 1 : 0));
  }, [symbols, savedSession, setup]);

  function selectSymbol(name: string, withMode?: AppMode) {
    const m = withMode ?? mode;
    const sym = symbols.find((s) => s.name === name);
    let s = defaultSetup(name, m);
    if (m === "witch" && sym) {
      const tIdx = sym.domains.indexOf("t");
      if (tIdx >= 0) s = { ...s, xDim: tIdx };
    }
    setSetup(s);
    setChartView(null);
    setTableView(null);
  }

  function patchSetup(patch: Partial<DisplaySetup>) {
    setSetup((prev) => {
      if (!prev) return prev;
      return { ...prev, ...patch };
    });
  }

  // Client-side cache of distinctKeys results by (symbol, dim). Invalidates
  // when the loaded-file set changes (path identity comparison via JSON of
  // paths). Returns a stable promise per (symbol, dim) so repeat mounts of
  // FilterPanel don't trigger IPC.
  const distinctKeysCache = useRef<Map<string, Promise<string[]>>>(new Map());
  const filesKey = useMemo(() => files.map((f) => f.path).join(""), [files]);
  useEffect(() => {
    distinctKeysCache.current.clear();
  }, [filesKey]);
  const fetchKeys = useCallback(
    (dim: number) => {
      if (!setup) return Promise.resolve<string[]>([]);
      const key = `${setup.symbol}${dim}`;
      const cached = distinctKeysCache.current.get(key);
      if (cached) return cached;
      const p = api.distinctKeys(setup.symbol, dim);
      distinctKeysCache.current.set(key, p);
      return p;
    },
    [setup?.symbol],
  );

  const gridCols = `${leftOpen ? "280px" : "0px"} 1fr ${rightOpen ? "300px" : "0px"}`;

  const filterChips = useMemo(() => {
    if (!currentSymbol || !setup) return [];
    const chips: { key: string; label: string }[] = [];
    if (currentSymbol.kind === "variable" || currentSymbol.kind === "equation") {
      chips.push({ key: "field", label: setup.field });
    }
    for (let d = 0; d < currentSymbol.dim; d++) {
      if (d === setup.xDim) continue;
      const dimName = currentSymbol.domains[d] ?? `d${d}`;
      const filterVals = setup.filters[String(d)];
      if (filterVals && filterVals.length === 1) {
        chips.push({ key: `f${d}`, label: `${dimName}: ${filterVals[0]}` });
      } else {
        const agg = setup.dimAgg[String(d)];
        if (agg === "sum") chips.push({ key: `a${d}`, label: `${dimName}: sum` });
        else if (agg === "mean") chips.push({ key: `a${d}`, label: `${dimName}: mean` });
      }
    }
    return chips;
  }, [currentSymbol, setup]);

  const currentUnit = useMemo(() => {
    if (!currentSymbol?.text || !setup) return null;
    if (!UNIT_FIELDS.has(setup.field)) return null;
    return extractUnit(currentSymbol.text);
  }, [currentSymbol, setup?.field]);

  // When the symbol or base unit changes, reset any manual unit choice.
  useEffect(() => { setUnitChoice(null); }, [currentUnit]);

  // Unit conversions. Multiple may apply at once (e.g. GtCe with e=co2 gives
  // both Gt/yr and GtCO2e/yr as targets); we accumulate all matches as
  // `{label, factor}` options. The first option is always the base unit.
  const unitOptions = useMemo<{ label: string; factor: number }[] | null>(() => {
    if (!currentUnit || !currentSymbol || !setup || mode === "gdx") return null;

    const opts: { label: string; factor: number }[] = [{ label: currentUnit, factor: 1 }];
    const push = (label: string, factor: number) => {
      if (opts.some((o) => o.label === label)) return;
      opts.push({ label, factor });
    };

    // Carbon price: T$/GtonCe ↔ $/tCO2 (×1000×12/44)
    if (/T\$.*[Gg]ton[Cc]/i.test(currentUnit)) {
      push("$/tCO2", (1000 * 12) / 44);
    }

    // Energy: TWh ↔ EJ
    if (currentUnit.includes("TWh")) {
      push(currentUnit.replace("TWh", "EJ"), 3.6 / 1000);
    }

    // Mass of carbon: GtC ↔ GtCO2e (×44/12). Word boundaries so we don't
    // match GtCe / GtCO2 / GtCH4.
    if (/\bGtC\b/.test(currentUnit)) {
      push(currentUnit.replace(/\bGtC\b/, "GtCO2e"), 44 / 12);
    }
    // Same conversion, verbose form: GTonC ↔ GtCO2e (×44/12). Case-insensitive
    // because real units appear as "GTonC", "GtonC", etc. Word boundary still
    // excludes "GTonCe".
    if (/\bGTonC\b/i.test(currentUnit)) {
      push(currentUnit.replace(/\bGTonC\b/i, "GtCO2e"), 44 / 12);
    }

    // Carbon equivalent: GtCe ↔ GtCO2e (×44/12). Always applicable when
    // the unit contains GtCe; e-dim specific options below stack on top.
    if (/GtCe/.test(currentUnit)) {
      push(currentUnit.replace(/GtCe/, "GtCO2e"), 44 / 12);
    }

    // Emissions specific to the filtered e-dim value(s).
    const eDim = currentSymbol.domains.indexOf("e");
    const eFilter = eDim >= 0 ? setup.filters[String(eDim)] : undefined;
    if (currentUnit.includes("Ce") && eFilter && eFilter.length > 0) {
      if (eFilter.every((v) => v.toLowerCase().startsWith("co2"))) {
        push(currentUnit.replace(/GtCe/, "Gt"), 44 / 12);
      } else if (eFilter.every((v) => v.toLowerCase().startsWith("ch4"))) {
        // AR4 GWP default for CH4 = 25.
        const gwp = emiGwp[eFilter[0]] ?? 25;
        push(currentUnit.replace(/GtCe/, "Mt"), ((44 / 12) * 1000) / gwp);
      } else if (eFilter.every((v) => v.toLowerCase().startsWith("n2o"))) {
        // AR4 GWP default for N2O = 298.
        const gwp = emiGwp[eFilter[0]] ?? 298;
        push(currentUnit.replace(/GtCe/, "Mt"), ((44 / 12) * 1000) / gwp);
      }
    }

    return opts.length > 1 ? opts : null;
  }, [currentUnit, currentSymbol, setup, emiGwp]);

  const displayUnit = unitOptions ? (unitChoice ?? unitOptions[0].label) : currentUnit;
  const conversionFactor =
    unitOptions?.find((o) => o.label === displayUnit)?.factor ?? 1;

  return (
    <div className="app" style={{ gridTemplateColumns: gridCols }}>
      <header className="bar">
        <h1>
          <span className="app-name" onClick={() => setAboutOpen(true)} title="About gdxcomp">gdxcomp</span><span className="sub">plot &amp; compare GDX</span>
        </h1>
        <div className="bar-right">
          <div className="toggle-group mode-toggle">
            <button className={mode === "gdx" ? "on" : ""} onClick={() => { setMode("gdx"); patchSetup({ mode: "gdx" }); }}>GDX</button>
            <button className={mode === "witch" ? "on" : ""} onClick={() => { setMode("witch"); patchSetup({ mode: "witch" }); }}>WITCH</button>
          </div>
          <div className="panel-toggles">
            <button
              className={`ghost icon-btn${leftOpen ? " active" : ""}`}
              onClick={() => setLeftOpen((o) => !o)}
              title={leftOpen ? "Collapse files panel" : "Expand files panel"}
            >
              <svg width="18" height="16" viewBox="0 0 18 16" fill="none" xmlns="http://www.w3.org/2000/svg">
                <rect x="1" y="1" width="16" height="14" rx="2.5" stroke="currentColor" strokeWidth="1.5"/>
                <rect x="1" y="1" width="5" height="14" rx="2" fill="currentColor" fillOpacity={leftOpen ? 0.45 : 0}/>
                <line x1="6" y1="1" x2="6" y2="15" stroke="currentColor" strokeWidth="1"/>
              </svg>
            </button>
            <button
              className={`ghost icon-btn${rightOpen ? " active" : ""}`}
              onClick={() => setRightOpen((o) => !o)}
              title={rightOpen ? "Collapse controls panel" : "Expand controls panel"}
            >
              <svg width="18" height="16" viewBox="0 0 18 16" fill="none" xmlns="http://www.w3.org/2000/svg">
                <rect x="1" y="1" width="16" height="14" rx="2.5" stroke="currentColor" strokeWidth="1.5"/>
                <rect x="12" y="1" width="5" height="14" rx="2" fill="currentColor" fillOpacity={rightOpen ? 0.45 : 0}/>
                <line x1="12" y1="1" x2="12" y2="15" stroke="currentColor" strokeWidth="1"/>
              </svg>
            </button>
          </div>
        </div>
      </header>

      <div className={`col left${leftOpen ? "" : " collapsed"}`}>
        {leftOpen && (
          <>
            <FileBar files={files} onOpen={handleOpen} onOpenFolder={handleOpenFolder} onRemove={handleRemove} onRename={handleRename} onResetScenarios={handleResetScenarios} onClearFiles={handleClearFiles} />
            <SymbolPicker symbols={sortedSymbols} selected={setup?.symbol ?? null} onSelect={selectSymbol} />
          </>
        )}
      </div>

      <div className="center">
        <div className="plot-toolbar">
          <div className="toggle-group">
            <button className={tab === "chart" ? "on" : ""} onClick={() => setTab("chart")}>Chart</button>
            <button className={tab === "table" ? "on" : ""} onClick={() => setTab("table")}>Table</button>
          </div>
          {tab === "chart" && (
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={showZero}
                onChange={(e) => setShowZero(e.target.checked)}
              />
              0-intercept
            </label>
          )}
          {error && <span className="error-inline">{error}</span>}
          {tab === "chart" && displayUnit && (
            unitOptions ? (
              <div className="toggle-group unit-toggle">
                {unitOptions.map((o) => (
                  <button key={o.label} className={displayUnit === o.label ? "on" : ""} onClick={() => setUnitChoice(o.label)}>{o.label}</button>
                ))}
              </div>
            ) : (
              <span className="unit-display">{displayUnit}</span>
            )
          )}
        </div>
        {currentSymbol && (
          <div className="symbol-title" title={currentSymbol.text || undefined}>
            <span className="symbol-title-name">{currentSymbol.name}</span>
            {currentSymbol.text && <span className="symbol-title-text">{currentSymbol.text}</span>}
            {filterChips.length > 0 && (
              <span className="symbol-title-chips">
                {filterChips.map((c) => (
                  <span key={c.key} className="filter-chip">{c.label}</span>
                ))}
              </span>
            )}
          </div>
        )}
        {savedSession && files.length === 0 && (
          <div className="session-banner">
            <span>Reopen last session? {savedSession.files.map((p) => p.split(/[\\/]/).pop()).join(", ")}</span>
            <div className="session-banner-actions">
              <button onClick={restoreSession}>Restore</button>
              <button className="ghost" onClick={() => setSavedSession(null)}>Dismiss</button>
            </div>
          </div>
        )}
        <div className="plot-wrap">
          {tab === "chart"
            ? chartView
              ? (
                <Suspense fallback={<div className="loading-overlay"><div className="spinner" /></div>}>
                  <ChartView view={chartView} showZero={showZero} unit={displayUnit} conversionFactor={conversionFactor} />
                </Suspense>
              )
              : !loading && (
                <div className="empty">
                  {files.length === 0
                    ? savedSession
                      ? null
                      : "Add one or more GDX files to begin."
                    : !setup?.symbol
                      ? "Pick a symbol to plot."
                      : null}
                </div>
              )
            : tableView
              ? <DataTable view={tableView} />
              : !loading && <div className="empty">Loading table…</div>
          }
          {loading && (
            <div className="loading-overlay">
              <div className="spinner" />
            </div>
          )}
        </div>
      </div>

      <div className={`col right${rightOpen ? "" : " collapsed"}`}>
        {rightOpen && (
          currentSymbol && setup ? (
            <>
              <MappingPanel symbol={currentSymbol} setup={setup} mode={mode} onChange={patchSetup} />
              <FilterPanel
                symbol={currentSymbol}
                setup={setup}
                mode={mode}
                onChange={patchSetup}
                fetchKeys={fetchKeys}
              />
            </>
          ) : (
            <div className="empty">Mapping &amp; filters appear here once a symbol is selected.</div>
          )
        )}
      </div>

      {aboutOpen && <AboutDialog onClose={() => setAboutOpen(false)} />}
    </div>
  );
}
