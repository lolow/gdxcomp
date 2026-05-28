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

  // Revision increments only when something Plotly actually has to redraw —
  // never on incidental parent re-renders. Without this, a unit toggle that
  // only changes y (new array, same x ref) can leave Plotly's internal axis
  // state desynced and render the x axis at -1..6 instead of the year range.
  const revision = useRevision(view, conversionFactor, showZero, yTitle);

  return (
    <Plot
      data={data as never}
      layout={layout as never}
      revision={revision}
      config={{ displaylogo: false, responsive: true } as never}
      useResizeHandler
      style={{ width: "100%", height: "100%" }}
    />
  );
}

function useRevision(...keys: unknown[]): number {
  return useMemo(() => Date.now(), keys); // eslint-disable-line react-hooks/exhaustive-deps
}
