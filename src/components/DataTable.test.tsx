import { act, render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DataTable } from "./DataTable";
import type { TableRow, TableView } from "../types";

function makeView(n: number): TableView {
  const table: TableRow[] = Array.from({ length: n }, (_, i) => ({
    file: "f",
    keys: [`k${i}`],
    value: i,
  }));
  return { dimNames: ["Dim1"], table };
}

const ROW_CAP = 5000;

describe("DataTable", () => {
  it("renders all rows when count is under the cap", () => {
    render(<DataTable view={makeView(3)} />);
    const rows = screen.getAllByRole("row");
    // 2 header rows + 3 data rows
    expect(rows).toHaveLength(5);
    expect(screen.queryByText(/showing/i)).toBeNull();
  });

  it("caps rendering and shows notice when over the cap", () => {
    render(<DataTable view={makeView(ROW_CAP + 10)} />);
    const bodyRows = document.querySelectorAll("tbody tr");
    expect(bodyRows).toHaveLength(ROW_CAP);
    expect(screen.getByText(/showing first 5,000 of 5,010/i)).toBeInTheDocument();
  });

  it("column filter narrows displayed rows", async () => {
    render(<DataTable view={makeView(5)} />);
    const inputs = screen.getAllByPlaceholderText("filter…");
    await act(async () => {
      fireEvent.change(inputs[1], { target: { value: "k2" } });
    });
    const bodyRows = document.querySelectorAll("tbody tr");
    expect(bodyRows).toHaveLength(1);
  });
});
