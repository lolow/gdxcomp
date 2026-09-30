import { describe, expect, it } from "vitest";
import { extractUnit, extractUnits, unitSegments } from "./units";

const MCOST_INV =
  "Average cost of investment [T$/TW] [T$/million vehicles] [$/kWh] (batteries) [T$/GtonC] (DAC)";

describe("extractUnits", () => {
  it("lists every bracketed unit in order", () => {
    expect(extractUnits(MCOST_INV)).toEqual(["T$/TW", "T$/million vehicles", "$/kWh", "T$/GtonC"]);
    expect(extractUnits("no unit")).toEqual([]);
  });
});

describe("extractUnit", () => {
  it("takes the first bracketed unit when several are listed", () => {
    expect(extractUnit(MCOST_INV)).toBe("T$/TW");
    expect(extractUnit("Installed capacity by technology [TW][GtCe](DAC)")).toBe("TW");
  });

  it("returns the only unit, or null when there is none", () => {
    expect(extractUnit("Emissions [GtCe/yr]")).toBe("GtCe/yr");
    expect(extractUnit("no unit")).toBeNull();
  });
});

describe("unitSegments", () => {
  it("splits a description into text and unit parts", () => {
    expect(unitSegments("capacity [TW][GtCe](DAC)")).toEqual([
      { text: "capacity ", unit: false },
      { text: "TW", unit: true },
      { text: "GtCe", unit: true },
      { text: "(DAC)", unit: false },
    ]);
  });
});
