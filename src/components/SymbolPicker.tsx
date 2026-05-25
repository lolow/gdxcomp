import { useMemo, useState } from "react";
import type { SymbolMeta } from "../types";

interface Props {
  symbols: SymbolMeta[];
  selected: string | null;
  onSelect: (name: string) => void;
}

export function SymbolPicker({ symbols, selected, onSelect }: Props) {
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return symbols;
    return symbols.filter(
      (s) => s.name.toLowerCase().includes(q) || s.text.toLowerCase().includes(q),
    );
  }, [symbols, query]);

  return (
    <div className="section">
      <h2>Symbols ({symbols.length} common)</h2>
      <input
        type="search"
        placeholder="Filter symbols…"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        style={{ marginBottom: 8 }}
      />
      {symbols.length === 0 ? (
        <div className="empty">Load files to list comparable symbols.</div>
      ) : (
        <div className="symbol-list">
          {filtered.map((s) => (
            <button
              key={s.name}
              className={"symbol" + (s.name === selected ? " active" : "")}
              onClick={() => onSelect(s.name)}
            >
              <span className="name">{s.name}</span>
              <span className="meta">
                {s.kind} · dim {s.dim} · {s.records} recs
                {s.text ? ` · ${s.text}` : ""}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
