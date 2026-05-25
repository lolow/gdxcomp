import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
  const [view, setView] = useState<PlotView | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Tracks the JSON of the last setup that was sent to get_view. When get_view
  // returns an effective setup (refined defaults), we update state only if the
  // content actually changed — avoiding an infinite render loop.
  const lastSentSetupJson = useRef<string | null>(null);

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

  // Recompute the view whenever the setup or file set changes.
  useEffect(() => {
    if (!setup || !setup.symbol || files.length === 0) {
      setView(null);
      return;
    }
    let cancelled = false;
    const sentJson = JSON.stringify(setup);
    lastSentSetupJson.current = sentJson;

    api
      .getView(setup)
      .then(({ view: v, setup: effectiveSetup }) => {
        if (cancelled) return;
        setView(v);
        setError(null);
        // Sync back the effective setup (may have auto-selected series defaults).
        // Only update state if the content changed to avoid a re-render loop.
        const effectiveJson = JSON.stringify(effectiveSetup);
        if (effectiveJson !== sentJson) {
          lastSentSetupJson.current = effectiveJson;
          setSetup(effectiveSetup);
        }
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
  }, [setup, files]);

  async function handleOpen(paths: string[]) {
    try {
      await api.openGdx(paths);
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
  }

  function patchSetup(patch: Partial<DisplaySetup>) {
    setSetup((prev) => {
      if (!prev) return prev;
      const next = { ...prev, ...patch };
      // A dimension cannot be both the x-axis and the series.
      if (next.seriesDim === next.xDim) next.seriesDim = null;
      return next;
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
        <FileBar files={files} onOpen={handleOpen} onRemove={handleRemove} />
        <SymbolPicker symbols={symbols} selected={setup?.symbol ?? null} onSelect={selectSymbol} />
      </div>

      <div className="center">
        <div className="plot-wrap">
          {error && <div className="error">{error}</div>}
          {view ? (
            <ChartView view={view} />
          ) : (
            <div className="empty">
              {files.length === 0
                ? "Add one or more GDX files to begin."
                : "Pick a symbol to plot."}
            </div>
          )}
        </div>
        {view && <DataTable view={view} />}
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
