import { useMemo } from "react";
import Plotly from "plotly.js-dist-min";
import createPlotlyComponent from "react-plotly.js/factory";
import type { PlotView } from "../types";

const Plot = createPlotlyComponent(Plotly);

interface Props {
  view: PlotView;
  showZero: boolean;
}

export function ChartView({ view, showZero }: Props) {
  const data = useMemo(
    () =>
      view.traces.map((t) => ({
        type: "scatter",
        mode: "lines+markers",
        name: t.name,
        x: t.x,
        y: t.y,
        connectgaps: false,
      })),
    [view],
  );

  const rangemode = showZero ? "tozero" : "normal";

  const yTitle =
    view.kind === "variable" || view.kind === "equation"
      ? `${view.symbol} (${view.field})`
      : view.symbol;

  const layout = {
    autosize: true,
    margin: { l: 64, r: 16, t: 24, b: 64 },
    xaxis: { title: { text: view.xLabel }, type: "category", automargin: true },
    yaxis: { title: { text: yTitle }, automargin: true, rangemode },
    legend: { orientation: "h", y: -0.2 },
    font: { family: "system-ui, sans-serif", size: 12 },
    paper_bgcolor: "transparent",
    plot_bgcolor: "transparent",
  };

  if (view.traces.length === 0) {
    return <div className="empty">No data for the current filters.</div>;
  }

  return (
    <Plot
      data={data as never}
      layout={layout as never}
      config={{ displaylogo: false, responsive: true } as never}
      useResizeHandler
      style={{ width: "100%", height: "100%" }}
    />
  );
}
