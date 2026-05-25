import { useCallback, useEffect, useMemo, useState } from "react";
import { api } from "./api";
import { ChartView } from "./components/ChartView";
import { DataTable } from "./components/DataTable";
import { FileBar } from "./components/FileBar";
import { FilterPanel } from "./components/FilterPanel";
import { MappingPanel } from "./components/MappingPanel";
import { SetupToolbar } from "./components/SetupToolbar";
import { SymbolPicker } from "./components/SymbolPicker";
import type { DisplaySetup, FileMeta, PlotView, SymbolMeta } from "./types";
import { defaultSetup } from "./types";

export function App() {
  const [files, setFiles] = useState<FileMeta[]>([]);
  const [symbols, setSymbols] = useState<SymbolMeta[]>([]);
  const [setup, setSetup] = useState<DisplaySetup | null>(null);
  // plotSetup is the setup actually submitted to getView; null = nothing plotted yet.
  const [plotSetup, setPlotSetup] = useState<DisplaySetup | null>(null);
  const [view, setView] = useState<PlotView | null>(null);
  const [tab, setTab] = useState<"chart" | "table">("chart");
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

  // Fire getView only when the user explicitly clicks Plot.
  useEffect(() => {
    if (!plotSetup || !plotSetup.symbol || files.length === 0) {
      setView(null);
      return;
    }
    let cancelled = false;
    api
      .getView(plotSetup)
      .then(({ view: v, setup: eff }) => {
        if (cancelled) return;
        setView(v);
        setError(null);
        // Reflect refined defaults (e.g. auto x-axis filter) back into the controls.
        setSetup(eff);
      })
      .catch((e) => {
        if (!cancelled) {
          setView(null);
          setError(String(e));
        }
      });
    return () => {
      cancelled = true;
    };
  }, [plotSetup]); // eslint-disable-line react-hooks/exhaustive-deps

  function handlePlot() {
    if (setup) setPlotSetup({ ...setup });
  }

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

  async function importSetup(loaded: DisplaySetup) {
    try {
      if (loaded.files.length > 0) {
        await api.openGdx(loaded.files);
        await syncFromBackend();
      }
      setSetup(loaded);
    } catch (e) {
      setError(String(e));
    }
  }

  const fetchKeys = useCallback(
    (dim: number) => (setup ? api.distinctKeys(setup.symbol, dim) : Promise.resolve([])),
    [setup?.symbol],
  );

  const canPlot = Boolean(setup?.symbol && files.length > 0);

  return (
    <div className="app">
      <header className="bar">
        <h1>
          gdxcomp<span className="sub">plot &amp; compare GDX</span>
        </h1>
        <SetupToolbar
          setup={setup}
          filePaths={files.map((f) => f.path)}
          onImport={importSetup}
          onError={setError}
        />
      </header>

      <div className="col left">
        <FileBar files={files} onOpen={handleOpen} onOpenFolder={handleOpenFolder} onRemove={handleRemove} />
        <SymbolPicker symbols={symbols} selected={setup?.symbol ?? null} onSelect={selectSymbol} />
      </div>

      <div className="center">
        <div className="plot-toolbar">
          <button className="primary" disabled={!canPlot} onClick={handlePlot}>
            Update
          </button>
          <div className="toggle-group">
            <button className={tab === "chart" ? "on" : ""} onClick={() => setTab("chart")}>Chart</button>
            <button className={tab === "table" ? "on" : ""} onClick={() => setTab("table")}>Table</button>
          </div>
          {error && <span className="error-inline">{error}</span>}
        </div>
        <div className="plot-wrap">
          {view ? (
            tab === "chart" ? <ChartView view={view} /> : <DataTable view={view} />
          ) : (
            <div className="empty">
              {files.length === 0
                ? "Add one or more GDX files to begin."
                : !setup?.symbol
                  ? "Pick a symbol to plot."
                  : "Click Plot to render the chart."}
            </div>
          )}
        </div>
      </div>

      <div className="col right">
        {currentSymbol && setup ? (
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
        )}
      </div>
    </div>
  );
}
