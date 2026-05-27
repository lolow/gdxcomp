import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "./api";
import { ChartView } from "./components/ChartView";
import { DataTable } from "./components/DataTable";
import { FileBar } from "./components/FileBar";
import { FilterPanel } from "./components/FilterPanel";
import { MappingPanel } from "./components/MappingPanel";
import { SymbolPicker } from "./components/SymbolPicker";
import type { AppMode, DisplaySetup, FileMeta, PlotView, Session, SymbolMeta } from "./types";
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
  const [view, setView] = useState<PlotView | null>(null);
  const [loading, setLoading] = useState(false);
  const [tab, setTab] = useState<"chart" | "table">("chart");
  const [showZero, setShowZero] = useState(true);
  const [leftOpen, setLeftOpen] = useState(true);
  const [rightOpen, setRightOpen] = useState(true);
  const [error, setError] = useState<string | null>(null);
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

  // Auto-refresh chart 400 ms after any setup or file change.
  useEffect(() => {
    if (!setup?.symbol || files.length === 0) {
      setView(null);
      setLoading(false);
      return;
    }
    let cancelled = false;
    const timer = setTimeout(() => {
      setLoading(true);
      api
        .getView(setup)
        .then(({ view: v }) => {
          if (!cancelled) {
            setView(v);
            setError(null);
            setLoading(false);
          }
        })
        .catch((e) => {
          if (!cancelled) {
            setView(null);
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
    setView(null);
  }

  function patchSetup(patch: Partial<DisplaySetup>) {
    setSetup((prev) => {
      if (!prev) return prev;
      return { ...prev, ...patch };
    });
  }

  const fetchKeys = useCallback(
    (dim: number) => (setup ? api.distinctKeys(setup.symbol, dim) : Promise.resolve([])),
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

  // Unit toggle: e=co2* → GtCe→Gt (×44/12); e=ch4* → GtCe→Mt (×1000/gwp).
  const { unitOptions, conversionFactor: unitConversionFactor } = useMemo(() => {
    if (!currentUnit || !currentUnit.includes("Ce") || !currentSymbol || !setup) {
      return { unitOptions: null, conversionFactor: 1 };
    }
    const eDim = currentSymbol.domains.indexOf("e");
    if (eDim < 0) return { unitOptions: null, conversionFactor: 1 };
    const eFilter = setup.filters[String(eDim)];
    if (!eFilter || eFilter.length === 0) return { unitOptions: null, conversionFactor: 1 };

    if (eFilter.every((v) => v.toLowerCase().startsWith("co2"))) {
      return {
        unitOptions: [currentUnit, currentUnit.replace(/GtCe/, "Gt")],
        conversionFactor: 44 / 12,
      };
    }
    if (eFilter.every((v) => v.toLowerCase().startsWith("ch4"))) {
      // Look up GWP for the first filtered e value; fall back to AR4 default (25).
      const gwp = emiGwp[eFilter[0]] ?? 25;
      return {
        unitOptions: [currentUnit, currentUnit.replace(/GtCe/, "Mt")],
        conversionFactor: 1000 / gwp,
      };
    }
    return { unitOptions: null, conversionFactor: 1 };
  }, [currentUnit, currentSymbol, setup, emiGwp]);

  const displayUnit = unitOptions ? (unitChoice ?? unitOptions[0]) : currentUnit;
  const conversionFactor = unitOptions && displayUnit === unitOptions[1] ? unitConversionFactor : 1;

  return (
    <div className="app" style={{ gridTemplateColumns: gridCols }}>
      <header className="bar">
        <h1>
          gdxcomp<span className="sub">plot &amp; compare GDX</span>
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
            <FileBar files={files} onOpen={handleOpen} onOpenFolder={handleOpenFolder} onRemove={handleRemove} onRename={handleRename} onResetScenarios={handleResetScenarios} />
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
                {unitOptions.map((u) => (
                  <button key={u} className={displayUnit === u ? "on" : ""} onClick={() => setUnitChoice(u)}>{u}</button>
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
          {view
            ? tab === "chart" ? <ChartView view={view} showZero={showZero} unit={displayUnit} conversionFactor={conversionFactor} /> : <DataTable view={view} />
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
    </div>
  );
}
