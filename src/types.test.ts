import { describe, expect, it } from "vitest";
import { defaultSetup, fieldsForKind } from "./types";

describe("defaultSetup", () => {
  it("starts on dim 0", () => {
    const s = defaultSetup("c");
    expect(s.symbol).toBe("c");
    expect(s.xDim).toBe(0);
    expect(s.field).toBe("level");
  });
});

describe("fieldsForKind", () => {
  it("exposes all five fields for variables and equations", () => {
    expect(fieldsForKind("variable")).toEqual([
      "level",
      "marginal",
      "lower",
      "upper",
      "scale",
    ]);
    expect(fieldsForKind("equation")).toHaveLength(5);
  });

  it("exposes only the level for parameters and sets", () => {
    expect(fieldsForKind("parameter")).toEqual(["level"]);
    expect(fieldsForKind("set")).toEqual(["level"]);
  });
});
