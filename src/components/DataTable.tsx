import { useMemo, useState } from "react";
import type { PlotView, TableRow } from "../types";

interface Props {
  view: PlotView;
}

type SortKey = { col: number; dir: 1 | -1 };

function fmt(v: number | null): string {
  if (v === null || Number.isNaN(v)) return "—";
  if (!Number.isFinite(v)) return v > 0 ? "+∞" : "−∞";
  return Number(v.toPrecision(6)).toString();
}

export function DataTable({ view }: Props) {
  const [sort, setSort] = useState<SortKey | null>(null);
  // Columns: File, <dim names>, Value. Indices: 0=file, 1..n=keys, n+1=value.
  const valueCol = view.dimNames.length + 1;

  const cell = (row: TableRow, col: number): string | number => {
    if (col === 0) return row.file;
    if (col === valueCol) return row.value ?? Number.NEGATIVE_INFINITY;
    return row.keys[col - 1] ?? "";
  };

  const rows = useMemo(() => {
    if (!sort) return view.table;
    const sorted = [...view.table];
    sorted.sort((a, b) => {
      const va = cell(a, sort.col);
      const vb = cell(b, sort.col);
      if (va < vb) return -sort.dir;
      if (va > vb) return sort.dir;
      return 0;
    });
    return sorted;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [view.table, sort]);

  function clickHeader(col: number) {
    setSort((s) => (s && s.col === col ? { col, dir: (s.dir * -1) as 1 | -1 } : { col, dir: 1 }));
  }

  const headers = ["File", ...view.dimNames, "Value"];

  return (
    <div className="table-wrap">
      <table>
        <thead>
          <tr>
            {headers.map((h, i) => (
              <th key={i} onClick={() => clickHeader(i)}>
                {h}
                {sort?.col === i ? (sort.dir === 1 ? " ▲" : " ▼") : ""}
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
