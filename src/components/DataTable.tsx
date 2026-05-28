import { useMemo, useState } from "react";
import type { TableRow, TableView } from "../types";

interface Props {
  view: TableView;
}

type SortKey = { col: number; dir: 1 | -1 };

function fmt(v: number | null): string {
  if (v === null || Number.isNaN(v)) return "—";
  if (!Number.isFinite(v)) return v > 0 ? "+∞" : "−∞";
  return Number(v.toPrecision(6)).toString();
}

export function DataTable({ view }: Props) {
  const [sort, setSort] = useState<SortKey | null>(null);
  const [colFilters, setColFilters] = useState<Record<number, string>>({});

  // Columns: File, <dim names>, Value.
  const valueCol = view.dimNames.length + 1;
  const headers = ["File", ...view.dimNames, "Value"];

  const cellStr = (row: TableRow, col: number): string => {
    if (col === 0) return row.file;
    if (col === valueCol) return fmt(row.value);
    return row.keys[col - 1] ?? "";
  };

  const cellSort = (row: TableRow, col: number): string | number => {
    if (col === 0) return row.file;
    if (col === valueCol) return row.value ?? Number.NEGATIVE_INFINITY;
    return row.keys[col - 1] ?? "";
  };

  const rows = useMemo(() => {
    const activeFilters = Object.entries(colFilters).filter(([, v]) => v.trim());
    let result = view.table;

    if (activeFilters.length > 0) {
      result = result.filter((row) =>
        activeFilters.every(([col, f]) =>
          cellStr(row, Number(col)).toLowerCase().includes(f.toLowerCase()),
        ),
      );
    }

    if (sort) {
      result = [...result].sort((a, b) => {
        const va = cellSort(a, sort.col);
        const vb = cellSort(b, sort.col);
        if (va < vb) return -sort.dir;
        if (va > vb) return sort.dir;
        return 0;
      });
    }

    return result;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [view.table, sort, colFilters]);

  function clickHeader(col: number) {
    setSort((s) => (s?.col === col ? { col, dir: (s.dir * -1) as 1 | -1 } : { col, dir: 1 }));
  }

  function setFilter(col: number, value: string) {
    setColFilters((prev) => {
      const next = { ...prev, [col]: value };
      if (!value) delete next[col];
      return next;
    });
  }

  return (
    <div className="table-wrap">
      <table>
        <thead>
          <tr>
            {headers.map((h, i) => (
              <th key={i} className="sortable" onClick={() => clickHeader(i)}>
                {h}
                {sort?.col === i ? (sort.dir === 1 ? " ▲" : " ▼") : ""}
              </th>
            ))}
          </tr>
          <tr className="filter-row">
            {headers.map((_, i) => (
              <th key={i}>
                <input
                  type="search"
                  value={colFilters[i] ?? ""}
                  onChange={(e) => setFilter(i, e.target.value)}
                  placeholder="filter…"
                />
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => (
            <tr key={i}>
              <td>{r.file}</td>
              {r.keys.map((k, j) => (
                <td key={j}>{k}</td>
              ))}
              <td className="num">{fmt(r.value)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
