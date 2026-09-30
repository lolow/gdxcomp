import { itersOf, normalizeRange, runsOf, type IterRange } from "../iteration";
import type { FileMeta } from "../types";
import { RangeSlider } from "./RangeSlider";

interface Props {
  files: FileMeta[];
  range: IterRange;
  onChange: (range: IterRange) => void;
}

export function IterationPanel({ files, range, onChange }: Props) {
  const runs = runsOf(files);
  const iters = itersOf(files, range.rLo, range.rHi);

  return (
    <div className="section">
      <h2>Iterations</h2>
      <div className="field">
        <span style={{ color: "var(--muted)", fontSize: 12 }}>iteration (i)</span>
        <RangeSlider
          labels={iters}
          lo={iters.indexOf(range.iLo)}
          hi={iters.indexOf(range.iHi)}
          onChange={(lo, hi) => onChange({ ...range, iLo: iters[lo], iHi: iters[hi] })}
        />
      </div>
      {runs.length > 1 && (
        <div className="field">
          <span style={{ color: "var(--muted)", fontSize: 12 }}>run (r)</span>
          <RangeSlider
            labels={runs}
            lo={runs.indexOf(range.rLo)}
            hi={runs.indexOf(range.rHi)}
            onChange={(lo, hi) =>
              onChange(normalizeRange(files, { ...range, rLo: runs[lo], rHi: runs[hi] }))
            }
          />
        </div>
      )}
    </div>
  );
}
