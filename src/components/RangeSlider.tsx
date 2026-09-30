interface Props {
  labels: (string | number)[];
  lo: number;
  hi: number;
  onChange: (lo: number, hi: number) => void;
}

/** Dual-thumb slider over the indices of `labels` (inclusive `lo`..`hi`). */
export function RangeSlider({ labels, lo, hi, onChange }: Props) {
  const last = labels.length - 1;
  const pct = (i: number) => `${last > 0 ? (i / last) * 100 : 0}%`;

  return (
    <div className="year-range">
      <div className="year-range-values">
        <span>{labels[lo]}</span>
        <span>–</span>
        <span>{labels[hi]}</span>
      </div>
      <div className="year-range-track">
        <div
          className="year-range-fill"
          style={{ left: pct(lo), width: `calc(${pct(hi)} - ${pct(lo)})` }}
        />
        <input
          type="range"
          min={0}
          max={last}
          value={lo}
          onChange={(e) => onChange(Math.min(Number(e.target.value), hi), hi)}
        />
        <input
          type="range"
          min={0}
          max={last}
          value={hi}
          onChange={(e) => onChange(lo, Math.max(Number(e.target.value), lo))}
        />
      </div>
    </div>
  );
}
