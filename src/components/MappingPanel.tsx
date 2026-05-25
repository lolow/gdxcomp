import type { DisplaySetup, Field, SymbolMeta } from "../types";
import { fieldsForKind } from "../types";

interface Props {
  symbol: SymbolMeta;
  setup: DisplaySetup;
  onChange: (patch: Partial<DisplaySetup>) => void;
}

export function dimLabel(symbol: SymbolMeta, i: number): string {
  const d = symbol.domains[i];
  return d && d !== "*" ? `${d} (dim ${i + 1})` : `Dim ${i + 1}`;
}


export function MappingPanel({ symbol, setup, onChange }: Props) {
  const dims = Array.from({ length: symbol.dim }, (_, i) => i);
  const fields = fieldsForKind(symbol.kind);
  const scalar = symbol.dim === 0;

  return (
    <div className="section">
      <h2>Mapping</h2>

      {fields.length > 1 && (
        <label className="field">
          <span>Value field</span>
          <select
            value={setup.field}
            onChange={(e) => onChange({ field: e.target.value as Field })}
          >
            {fields.map((f) => (
              <option key={f} value={f}>
                {f}
              </option>
            ))}
          </select>
        </label>
      )}

      <label className="field">
        <span>X axis</span>
        <select
          value={setup.xDim}
          disabled={scalar}
          onChange={(e) => onChange({ xDim: Number(e.target.value) })}
        >
          {scalar ? (
            <option value={0}>value (scalar)</option>
          ) : (
            dims.map((i) => (
              <option key={i} value={i}>
                {dimLabel(symbol, i)}
              </option>
            ))
          )}
        </select>
      </label>


    </div>
  );
}
