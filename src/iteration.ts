import type { FileMeta } from "./types";

// Plotly's own "Viridis" stops, so line colours agree with the colour bar.
const VIRIDIS: [number, string][] = [
  [0, "#440154"], [0.06274509803921569, "#48186a"],
  [0.12549019607843137, "#472d7b"], [0.18823529411764706, "#424086"],
  [0.25098039215686274, "#3b528b"], [0.3137254901960784, "#33638d"],
  [0.3764705882352941, "#2c728e"], [0.4392156862745098, "#26828e"],
  [0.5019607843137255, "#21918c"], [0.5647058823529412, "#1fa088"],
  [0.6274509803921569, "#28ae80"], [0.6901960784313725, "#3fbc73"],
  [0.7529411764705882, "#5ec962"], [0.8156862745098039, "#84d44b"],
  [0.8784313725490196, "#addc30"], [0.9411764705882353, "#d8e219"],
  [1, "#fde725"],
];

const RUN_DASHES = ["solid", "dash", "dot", "dashdot", "longdash", "longdashdot"];

export interface IterStyle {
  color: string;
  dash: string;
  run: number;
  iter: number;
}

/** Inclusive iteration (i) and run (r) bounds. */
export interface IterRange {
  iLo: number;
  iHi: number;
  rLo: number;
  rHi: number;
}

export function viridis(t: number): string {
  const x = Math.min(1, Math.max(0, t));
  const k = VIRIDIS.findIndex(([pos]) => pos >= x);
  if (k <= 0) return VIRIDIS[0][1];
  const [p0, c0] = VIRIDIS[k - 1];
  const [p1, c1] = VIRIDIS[k];
  const f = (x - p0) / (p1 - p0);
  const channel = (i: number) => {
    const a = parseInt(c0.slice(i, i + 2), 16);
    const b = parseInt(c1.slice(i, i + 2), 16);
    return Math.round(a + (b - a) * f).toString(16).padStart(2, "0");
  };
  return `#${channel(1)}${channel(3)}${channel(5)}`;
}

export function isIterationSet(files: FileMeta[]): boolean {
  return files.length > 0 && files.every((f) => f.iter);
}

function sortedUnique(values: number[]): number[] {
  return [...new Set(values)].sort((a, b) => a - b);
}

export function runsOf(files: FileMeta[]): number[] {
  return sortedUnique(files.map((f) => f.iter?.run ?? 0));
}

/** Iterations present in files whose run lies in [rLo, rHi]. */
export function itersOf(files: FileMeta[], rLo: number, rHi: number): number[] {
  return sortedUnique(
    files.filter((f) => f.iter && f.iter.run >= rLo && f.iter.run <= rHi).map((f) => f.iter!.iter),
  );
}

export function fullRange(files: FileMeta[]): IterRange {
  const runs = runsOf(files);
  const iters = itersOf(files, runs[0], runs[runs.length - 1]);
  return { iLo: iters[0], iHi: iters[iters.length - 1], rLo: runs[0], rHi: runs[runs.length - 1] };
}

/** Snaps the iteration bounds onto values that exist in the selected runs,
 *  so the selection can never be empty. */
export function normalizeRange(files: FileMeta[], range: IterRange): IterRange {
  const iters = itersOf(files, range.rLo, range.rHi);
  const iLo = iters.find((i) => i >= range.iLo) ?? iters[iters.length - 1];
  const iHi = [...iters].reverse().find((i) => i <= range.iHi) ?? iters[0];
  return { ...range, iLo, iHi: Math.max(iLo, iHi) };
}

/** Paths for `DisplaySetup.files`: empty when every file is selected. */
export function selectedPaths(files: FileMeta[], range: IterRange | null): string[] {
  if (!range) return [];
  const picked = files.filter(
    (f) =>
      f.iter &&
      f.iter.run >= range.rLo &&
      f.iter.run <= range.rHi &&
      f.iter.iter >= range.iLo &&
      f.iter.iter <= range.iHi,
  );
  return picked.length === files.length ? [] : picked.map((f) => f.path);
}

/** Style per scenario name (traces are named by scenario). Colours span the
 *  full iteration range so they stay put while the range sliders move. */
export function iterationStyles(files: FileMeta[]): Map<string, IterStyle> {
  const runs = runsOf(files);
  const iters = sortedUnique(files.map((f) => f.iter?.iter ?? 0));
  const lo = iters[0];
  const span = iters[iters.length - 1] - lo || 1;
  const styles = new Map<string, IterStyle>();
  for (const f of files) {
    if (!f.iter) continue;
    styles.set(f.scenario, {
      color: viridis((f.iter.iter - lo) / span),
      dash: RUN_DASHES[runs.indexOf(f.iter.run) % RUN_DASHES.length],
      run: f.iter.run,
      iter: f.iter.iter,
    });
  }
  return styles;
}
