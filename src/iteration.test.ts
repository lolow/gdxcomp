import { describe, expect, it } from "vitest";
import {
  fullRange,
  isIterationSet,
  iterationStyles,
  normalizeRange,
  selectedPaths,
  viridis,
} from "./iteration";
import type { FileMeta } from "./types";

function file(run: number, iter: number, scenario = `r${run}_i${iter}`): FileMeta {
  return {
    label: `debug_r${run}_i${iter}`,
    scenario,
    path: `/d/debug_r${run}_i${iter}.gdx`,
    symbols: [],
    iter: { run, iter },
  };
}

const plain: FileMeta = { label: "base", scenario: "base", path: "/d/base.gdx", symbols: [], iter: null };

describe("viridis", () => {
  it("matches Plotly's Viridis end points", () => {
    expect(viridis(0)).toBe("#440154");
    expect(viridis(1)).toBe("#fde725");
  });

  it("clamps out-of-range input", () => {
    expect(viridis(-1)).toBe("#440154");
    expect(viridis(2)).toBe("#fde725");
  });
});

describe("isIterationSet", () => {
  it("requires every file to be tagged", () => {
    expect(isIterationSet([file(1, 1), file(1, 2)])).toBe(true);
    expect(isIterationSet([file(1, 1), plain])).toBe(false);
    expect(isIterationSet([])).toBe(false);
  });
});

describe("iterationStyles", () => {
  it("colours by iteration over the full range and dashes by run", () => {
    const styles = iterationStyles([file(1, 1), file(1, 5), file(2, 3)]);
    expect(styles.get("r1_i1")).toMatchObject({ color: "#440154", dash: "solid", run: 1, iter: 1 });
    expect(styles.get("r1_i5")).toMatchObject({ color: "#fde725", dash: "solid" });
    expect(styles.get("r2_i3")?.dash).toBe("dash");
  });
});

describe("selectedPaths", () => {
  const files = [file(1, 1), file(1, 2), file(1, 3), file(2, 1)];

  it("is empty when the full range is selected", () => {
    expect(selectedPaths(files, fullRange(files))).toEqual([]);
  });

  it("lists paths inside the range", () => {
    const range = { iLo: 2, iHi: 3, rLo: 1, rHi: 1 };
    expect(selectedPaths(files, range)).toEqual(["/d/debug_r1_i2.gdx", "/d/debug_r1_i3.gdx"]);
  });
});

describe("normalizeRange", () => {
  it("snaps iteration bounds to values present in the selected runs", () => {
    const files = [file(1, 1), file(1, 10), file(2, 1), file(2, 3)];
    expect(normalizeRange(files, { iLo: 5, iHi: 10, rLo: 2, rHi: 2 })).toEqual({
      iLo: 3,
      iHi: 3,
      rLo: 2,
      rHi: 2,
    });
  });
});
