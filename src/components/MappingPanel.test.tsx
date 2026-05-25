import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MappingPanel } from "./MappingPanel";
import { defaultSetup, type SymbolMeta } from "../types";

const variable: SymbolMeta = {
  name: "x",
  dim: 2,
  kind: "variable",
  records: 6,
  text: "shipment",
  domains: ["i", "j"],
};

const parameter: SymbolMeta = { ...variable, name: "c", kind: "parameter" };

describe("MappingPanel", () => {
  it("shows the value-field selector for variables", () => {
    render(<MappingPanel symbol={variable} setup={defaultSetup("x")} onChange={() => {}} />);
    expect(screen.getByText("Value field")).toBeInTheDocument();
  });

  it("hides the value-field selector for parameters", () => {
    render(<MappingPanel symbol={parameter} setup={defaultSetup("c")} onChange={() => {}} />);
    expect(screen.queryByText("Value field")).not.toBeInTheDocument();
  });

  it("shows the x-axis selector", () => {
    render(<MappingPanel symbol={parameter} setup={defaultSetup("c")} onChange={() => {}} />);
    expect(screen.getByText("X axis")).toBeInTheDocument();
  });
});
