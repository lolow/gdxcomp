// TypeScript mirrors of the Rust types exchanged with the backend.
// Field names match the serde representation (camelCase; lowercase enums).

export type SymbolKind =
  | "set"
  | "parameter"
  | "variable"
  | "equation"
  | "alias";

export type Field = "level" | "marginal" | "lower" | "upper" | "scale";
export type Aggregation = "sum" | "mean" | "min" | "max" | "count";
export type ChartKind = "line" | "bar";

export interface SymbolMeta {
  name: string;
  dim: number;
  kind: SymbolKind;
  records: number;
  text: string;
  domains: string[];
}

export interface FileMeta {
  label: string;
  path: string;
  symbols: SymbolMeta[];
}

export interface DisplaySetup {
  files: string[];
  symbol: string;
  field: Field;
  xDim: number;
  seriesDim: number | null;
  /** dimension index (as a string key in JSON) -> allowed UEL labels */
  filters: Record<string, string[]>;
  aggregate: Aggregation;
  chart: ChartKind;
}

export interface Trace {
  name: string;
  x: string[];
  y: (number | null)[];
}

export interface TableRow {
  file: string;
  keys: string[];
  value: number | null;
}

export interface PlotView {
  symbol: string;
  kind: SymbolKind;
  field: Field;
  chart: ChartKind;
  xLabel: string;
  seriesLabel: string | null;
  traces: Trace[];
  dimNames: string[];
  table: TableRow[];
}

/** A minimal setup for a symbol: x = dim 0, no series, sum aggregation. */
export function defaultSetup(symbol: string): DisplaySetup {
  return {
    files: [],
    symbol,
    field: "level",
    xDim: 0,
    seriesDim: null,
    filters: {},
    aggregate: "sum",
    chart: "line",
  };
}

export function fieldsForKind(kind: SymbolKind): Field[] {
  return kind === "variable" || kind === "equation"
    ? ["level", "marginal", "lower", "upper", "scale"]
    : ["level"];
}
