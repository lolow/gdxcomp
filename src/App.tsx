import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "./api";
import { ChartView } from "./components/ChartView";
import { DataTable } from "./components/DataTable";
import { FileBar } from "./components/FileBar";
import { FilterPanel } from "./components/FilterPanel";
import { MappingPanel } from "./components/MappingPanel";
import { SymbolPicker } from "./components/SymbolPicker";
import type { DisplaySetup, FileMeta, PlotView, SymbolMeta } from "./types";
import { defaultSetup } from "./types";

export function App() {
  const [files, setFiles] = useState<FileMeta[]>([]);
  const [symbols, setSymbols] = useState<SymbolMeta[]>([]);
  const [setup, setSetup] = useState<DisplaySetup | null>(null);
  const [view, setView] = useState<PlotView | null>(null);
  const [loading, setLoading] = useState(false);
  const [tab, setTab] = useState<"chart" | "table">("chart");
  const [showZero, setShowZero] = useState(false);
  const [leftOpen, setLeftOpen] = useState(true);
  const [rightOpen, setRightOpen] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const syncFromBackend = useCallback(async () => {
    const f = await api.listFiles();
    setFiles(f);
    setSymbols(await api.commonSymbols());
  }, []);

  useEffect(() => {
    syncFromBackend().catch((e) => setError(String(e)));
  }, [syncFromBackend]);

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

  function selectSymbol(name: string) {
    setSetup(defaultSetup(name));
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

  return (
    <div className="app" style={{ gridTemplateColumns: gridCols }}>
      <header className="bar">
        <h1>
          gdxcomp<span className="sub">plot &amp; compare GDX</span>
        </h1>
        <div className="bar-right">
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
            <FileBar files={files} onOpen={handleOpen} onOpenFolder={handleOpenFolder} onRemove={handleRemove} />
            <SymbolPicker symbols={symbols} selected={setup?.symbol ?? null} onSelect={selectSymbol} />
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
        </div>
        <div className="plot-wrap">
          {view
            ? tab === "chart" ? <ChartView view={view} showZero={showZero} /> : <DataTable view={view} />
            : !loading && (
              <div className="empty">
                {files.length === 0
                  ? "Add one or more GDX files to begin."
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
              <MappingPanel symbol={currentSymbol} setup={setup} onChange={patchSetup} />
              <FilterPanel
                symbol={currentSymbol}
                setup={setup}
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
