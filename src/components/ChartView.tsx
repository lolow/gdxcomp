import { useMemo } from "react";
import Plotly from "plotly.js-basic-dist-min";
import createPlotlyComponent from "react-plotly.js/factory";
import type { ChartView as ChartViewData } from "../types";

const Plot = createPlotlyComponent(Plotly);

interface Props {
  view: ChartViewData;
  showZero: boolean;
  unit?: string | null;
  conversionFactor?: number;
}

export function ChartView({ view, showZero, unit, conversionFactor = 1 }: Props) {
  const data = useMemo(
    () =>
      view.traces.map((t) => ({
        type: "scatter",
        mode: "lines+markers",
        name: t.name,
        x: t.x,
        y: conversionFactor !== 1
          ? t.y.map((v) => (v === null ? null : (v as number) * conversionFactor))
          : t.y,
        connectgaps: false,
      })),
    [view, conversionFactor],
  );

  const rangemode = showZero ? "tozero" : "normal";

  const yTitle = unit ?? "";

  const xAxisType =
    view.traces.length > 0 && typeof view.traces[0].x[0] === "number" ? "linear" : "category";

  const layout = {
    autosize: true,
    margin: { l: 64, r: 16, t: 24, b: 64 },
    xaxis: { title: { text: view.xLabel }, type: xAxisType, automargin: true, autorange: true },
    yaxis: { title: { text: yTitle }, automargin: true, rangemode, autorange: true },
    legend: { orientation: "h", y: -0.2 },
    font: { family: "system-ui, sans-serif", size: 12 },
    paper_bgcolor: "transparent",
    plot_bgcolor: "transparent",
  };

  if (view.traces.length === 0) {
    return <div className="empty">No data for the current filters.</div>;
  }

  // Force a full unmount/remount of Plot whenever the unit (and therefore
  // conversionFactor) changes. Plotly.react alone doesn't fully reset the
  // internal axis-range cache between updates, so a unit toggle could
  // re-render with the x-axis stuck at -1..6 instead of 2005..2100. A
  // changing key sidesteps that path entirely. Unit toggle is a low-frequency
  // user action, so the full redraw cost is fine.
  return (
    <Plot
      key={`${view.symbol}|${unit ?? ""}|${conversionFactor}`}
      data={data as never}
      layout={layout as never}
      config={{ displaylogo: false, responsive: true } as never}
      useResizeHandler
      style={{ width: "100%", height: "100%" }}
    />
  );
}
