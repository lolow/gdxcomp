import { useEffect, useState } from "react";
import type { DisplaySetup, SymbolMeta } from "../types";
import { dimLabel } from "./MappingPanel";

interface Props {
  symbol: SymbolMeta;
  setup: DisplaySetup;
  onChange: (patch: Partial<DisplaySetup>) => void;
  fetchKeys: (dim: number) => Promise<string[]>;
}

export function FilterPanel({ symbol, setup, onChange, fetchKeys }: Props) {
  const [keysByDim, setKeysByDim] = useState<Record<number, string[]>>({});

  // Load the available UELs for each dimension whenever the symbol changes.
  useEffect(() => {
    let cancelled = false;
    const dims = Array.from({ length: symbol.dim }, (_, i) => i);
    Promise.all(dims.map((d) => fetchKeys(d))).then((lists) => {
      if (cancelled) return;
      const map: Record<number, string[]> = {};
      dims.forEach((d, i) => (map[d] = lists[i]));
      setKeysByDim(map);
    });
    return () => {
      cancelled = true;
    };
  }, [symbol.name, symbol.dim, fetchKeys]);

  if (symbol.dim === 0) {
    return null;
  }

  function selected(dim: number): string[] {
    return setup.filters[String(dim)] ?? [];
  }

  function setSelected(dim: number, values: string[]) {
    const next = { ...setup.filters };
    const all = keysByDim[dim] ?? [];
    // Empty selection (or "all selected") means "no filter".
    if (values.length === 0 || values.length === all.length) {
      delete next[String(dim)];
    } else {
      next[String(dim)] = values;
    }
    onChange({ filters: next });
  }

  function toggle(dim: number, key: string, on: boolean) {
    const all = keysByDim[dim] ?? [];
    const current = selected(dim);
    // An empty stored filter means "all" — expand it before removing one.
    const base = current.length === 0 ? all : current;
    const next = on ? [...new Set([...base, key])] : base.filter((k) => k !== key);
    setSelected(dim, next);
  }

  function isChecked(dim: number, key: string): boolean {
    const current = selected(dim);
    return current.length === 0 || current.includes(key);
  }

  return (
    <div className="section">
      <h2>Filters</h2>
      {Array.from({ length: symbol.dim }, (_, dim) => {
        const keys = keysByDim[dim] ?? [];
        return (
          <div key={dim} className="field">
            <div className="row-gap" style={{ justifyContent: "space-between" }}>
              <span style={{ color: "var(--muted)", fontSize: 12 }}>{dimLabel(symbol, dim)}</span>
              <span className="row-gap">
                <button className="ghost" onClick={() => setSelected(dim, [])}>
                  all
                </button>
                <button
                  className="ghost"
                  onClick={() => setSelected(dim, keys.length ? [keys[0]] : [])}
                >
                  none
                </button>
              </span>
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
          </div>
        );
      })}
    </div>
  );
}
