import { useMemo, useState } from "react";
import type { SymbolKind, SymbolMeta } from "../types";

const KIND_ORDER: SymbolKind[] = ["parameter", "variable", "equation", "set", "alias"];
const KIND_LABEL: Record<SymbolKind, string> = {
  parameter: "Parameters",
  variable: "Variables",
  equation: "Equations",
  set: "Sets",
  alias: "Aliases",
};
const KIND_SHORT: Record<SymbolKind, string> = {
  parameter: "P",
  variable: "V",
  equation: "E",
  set: "S",
  alias: "A",
};

interface Props {
  symbols: SymbolMeta[];
  selected: string | null;
  onSelect: (name: string) => void;
}

interface SymbolButtonProps {
  s: SymbolMeta;
  selected: string | null;
  onSelect: (n: string) => void;
  showKind: boolean;
}

function SymbolButton({ s, selected, onSelect, showKind }: SymbolButtonProps) {
  return (
    <button
      className={"symbol" + (s.name === selected ? " active" : "")}
      onClick={() => onSelect(s.name)}
    >
      <span className="name">
        {showKind && (
          <span className={`kind-badge kind-${s.kind}`}>{KIND_SHORT[s.kind]}</span>
        )}
        {s.name}
      </span>
      <span className="meta">
        dim {s.dim} · {s.records} recs{s.text ? ` · ${s.text}` : ""}
      </span>
    </button>
  );
}

export function SymbolPicker({ symbols, selected, onSelect }: Props) {
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return null;
    return symbols.filter(
      (s) => s.name.toLowerCase().includes(q) || s.text.toLowerCase().includes(q),
    );
  }, [symbols, query]);

  const grouped = useMemo(() => {
    const map = new Map<SymbolKind, SymbolMeta[]>();
    for (const s of symbols) {
      if (!map.has(s.kind)) map.set(s.kind, []);
      map.get(s.kind)!.push(s);
    }
    return KIND_ORDER.filter((k) => map.has(k)).map((k) => ({ kind: k, items: map.get(k)! }));
  }, [symbols]);

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
          {filtered ? (
            filtered.length === 0 ? (
              <div className="empty">No matches.</div>
            ) : (
              filtered.map((s) => (
                <SymbolButton
                  key={s.name}
                  s={s}
                  selected={selected}
                  onSelect={onSelect}
                  showKind
                />
              ))
            )
          ) : (
            grouped.map(({ kind, items }) => (
              <details key={kind} className="symbol-group" open>
                <summary className="symbol-group-header">
                  {KIND_LABEL[kind]}
                  <span className="group-count">{items.length}</span>
                </summary>
                {items.map((s) => (
                  <SymbolButton
                    key={s.name}
                    s={s}
                    selected={selected}
                    onSelect={onSelect}
                    showKind={false}
                  />
                ))}
              </details>
            ))
          )}
        </div>
      )}
    </div>
  );
}
