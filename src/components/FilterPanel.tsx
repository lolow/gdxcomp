import { useEffect, useMemo, useState } from "react";
import type { AppMode, DimAgg, DisplaySetup, SymbolMeta } from "../types";
import { dimLabel } from "./MappingPanel";

interface Props {
  symbol: SymbolMeta;
  setup: DisplaySetup;
  mode: AppMode;
  onChange: (patch: Partial<DisplaySetup>) => void;
  fetchKeys: (dim: number) => Promise<string[]>;
}

function uelToYear(uel: string): number | null {
  const digits = uel.replace(/^\D*/, "");
  const n = parseFloat(digits);
  if (isNaN(n)) return null;
  return 2000 + 5 * n;
}

interface YearEntry { uel: string; year: number; }

function YearRangeFilter({ uels, filter, onFilter }: {
  uels: string[];
  filter: string[];
  onFilter: (uels: string[]) => void;
}) {
  const sorted: YearEntry[] = useMemo(() =>
    uels
      .map((uel) => ({ uel, year: uelToYear(uel) ?? 0 }))
      .filter((e) => e.year > 0)
      .sort((a, b) => a.year - b.year),
    [uels],
  );

  if (sorted.length === 0) return <div className="empty">no values</div>;

  const filterSet = new Set(filter.length > 0 ? filter : sorted.map((e) => e.uel));
  const inRange = sorted.filter((e) => filterSet.has(e.uel));
  const minIdx = inRange.length > 0 ? sorted.findIndex((e) => e.uel === inRange[0].uel) : 0;
  const maxIdx = inRange.length > 0 ? sorted.findIndex((e) => e.uel === inRange[inRange.length - 1].uel) : sorted.length - 1;

  function setRange(lo: number, hi: number) {
    const selected = sorted.slice(lo, hi + 1).map((e) => e.uel);
    onFilter(selected.length === sorted.length ? [] : selected);
  }

  const pct = (i: number) => `${(i / (sorted.length - 1)) * 100}%`;

  return (
    <div className="year-range">
      <div className="year-range-values">
        <span>{sorted[minIdx]?.year}</span>
        <span>–</span>
        <span>{sorted[maxIdx]?.year}</span>
      </div>
      <div className="year-range-track">
        <div
          className="year-range-fill"
          style={{ left: pct(minIdx), width: `calc(${pct(maxIdx)} - ${pct(minIdx)})` }}
        />
        <input
          type="range"
          min={0}
          max={sorted.length - 1}
          value={minIdx}
          onChange={(e) => setRange(Math.min(Number(e.target.value), maxIdx), maxIdx)}
        />
        <input
          type="range"
          min={0}
          max={sorted.length - 1}
          value={maxIdx}
          onChange={(e) => setRange(minIdx, Math.max(Number(e.target.value), minIdx))}
        />
      </div>
    </div>
  );
}

export function FilterPanel({ symbol, setup, mode, onChange, fetchKeys }: Props) {
  const [keysByDim, setKeysByDim] = useState<Record<number, string[]>>({});

  useEffect(() => {
    let cancelled = false;
    const dims = Array.from({ length: symbol.dim }, (_, i) => i);
    Promise.all(dims.map((d) => fetchKeys(d))).then((lists) => {
      if (cancelled) return;
      const map: Record<number, string[]> = {};
      dims.forEach((d, i) => (map[d] = [...lists[i]].sort((a, b) => a.localeCompare(b))));
      setKeysByDim(map);
    });
    return () => {
      cancelled = true;
    };
  }, [symbol.name, symbol.dim, fetchKeys]);

  if (symbol.dim === 0) {
    return null;
  }

  function selectedMany(dim: number): string[] {
    return setup.filters[String(dim)] ?? [];
  }

  function setMany(dim: number, values: string[]) {
    const next = { ...setup.filters };
    if (values.length === 0) {
      delete next[String(dim)];
    } else {
      next[String(dim)] = values;
    }
    onChange({ filters: next });
  }

  function toggle(dim: number, key: string, on: boolean) {
    const all = keysByDim[dim] ?? [];
    const current = selectedMany(dim);
    const base = current.length === 0 ? all : current;
    const next = on ? [...new Set([...base, key])] : base.filter((k) => k !== key);
    setMany(dim, next);
  }

  function isChecked(dim: number, key: string): boolean {
    const current = selectedMany(dim);
    return current.length === 0 || current.includes(key);
  }

  // For non-x dims: either an aggregation method or a specific UEL value.
  function nonXValue(dim: number): string {
    const uel = setup.filters[String(dim)]?.[0];
    if (uel) return uel;
    const agg = setup.dimAgg[String(dim)];
    return agg ? `__agg:${agg}` : "__agg:sum";
  }

  function setNonX(dim: number, value: string) {
    const nextFilters = { ...setup.filters };
    const nextDimAgg = { ...setup.dimAgg };
    if (value === "__agg:sum" || value === "__agg:mean") {
      delete nextFilters[String(dim)];
      nextDimAgg[String(dim)] = (value === "__agg:sum" ? "sum" : "mean") as DimAgg;
    } else {
      nextFilters[String(dim)] = [value];
      delete nextDimAgg[String(dim)];
    }
    onChange({ filters: nextFilters, dimAgg: nextDimAgg });
  }

  function isYearDim(dim: number): boolean {
    return mode === "witch" && symbol.domains[dim] === "t";
  }

  return (
    <div className="section">
      <h2>Filters</h2>
      {Array.from({ length: symbol.dim }, (_, dim) => {
        const keys = keysByDim[dim] ?? [];
        const isXDim = dim === setup.xDim;
        return (
          <div key={dim} className="field">
            <span style={{ color: "var(--muted)", fontSize: 12 }}>{dimLabel(symbol, dim, mode)}</span>
            {isXDim ? (
              isYearDim(dim) ? (
                <YearRangeFilter
                  uels={keys}
                  filter={selectedMany(dim)}
                  onFilter={(uels) => setMany(dim, uels)}
                />
              ) : (
                <>
                  <div className="row-gap" style={{ justifyContent: "flex-end" }}>
                    <button className="ghost" onClick={() => setMany(dim, [...keys])}>all</button>
                    <button className="ghost" onClick={() => setMany(dim, keys.length ? [keys[0]] : [])}>first</button>
                  </div>
                  <div className="checks">
                    {keys.length === 0 && <div className="empty">no values</div>}
                    {keys.map((k) => (
                      <label key={k}>
                        <input
                          type="checkbox"
                          checked={isChecked(dim, k)}
                          onChange={(e) => toggle(dim, k, e.target.checked)}
                        />
                        {k}
                      </label>
                    ))}
                  </div>
                </>
              )
            ) : (
              <select value={nonXValue(dim)} onChange={(e) => setNonX(dim, e.target.value)}>
                <option value="__agg:sum">— sum —</option>
                <option value="__agg:mean">— mean —</option>
                {keys.map((k) => (
                  <option key={k} value={k}>{k}</option>
                ))}
              </select>
            )}
          </div>
        );
      })}
    </div>
  );
}
