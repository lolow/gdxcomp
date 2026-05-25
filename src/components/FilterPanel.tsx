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

  function selectedOne(dim: number): string {
    return setup.filters[String(dim)]?.[0] ?? "";
  }

  function selectedMany(dim: number): string[] {
    return setup.filters[String(dim)] ?? [];
  }

  function setSingle(dim: number, value: string) {
    const next = { ...setup.filters };
    if (value === "") {
      delete next[String(dim)];
    } else {
      next[String(dim)] = [value];
    }
    onChange({ filters: next });
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

  return (
    <div className="section">
      <h2>Filters</h2>
      {Array.from({ length: symbol.dim }, (_, dim) => {
        const keys = keysByDim[dim] ?? [];
        const isXDim = dim === setup.xDim;
        return (
          <div key={dim} className="field">
            <span style={{ color: "var(--muted)", fontSize: 12 }}>{dimLabel(symbol, dim)}</span>
            {isXDim ? (
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
            ) : (
              <select
                value={selectedOne(dim)}
                onChange={(e) => setSingle(dim, e.target.value)}
              >
                <option value="">— all —</option>
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
