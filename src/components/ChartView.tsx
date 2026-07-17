import { useMemo } from "react";
import Plotly from "plotly.js-basic-dist-min";
import createPlotlyComponent from "react-plotly.js/factory";
import { save } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import type { ChartView as ChartViewData } from "../types";

const Plot = createPlotlyComponent(Plotly);

interface Props {
  view: ChartViewData;
  showZero: boolean;
  unit?: string | null;
  conversionFactor?: number;
  defaultSavePath: string;
  title: string;
}

// The title (variable, description, field, mapping) is baked into the
// exported image only — the on-screen chart already shows the same info in
// the app's own header, so a persistent Plotly title would duplicate it.
// It's added just for the toImage capture, then removed again.
async function saveChartAsPng(gd: unknown, defaultSavePath: string, title: string) {
  const path = await save({
    defaultPath: defaultSavePath,
    filters: [{ name: "PNG image", extensions: ["png"] }],
  });
  if (!path) return;
  await Plotly.relayout(gd as never, {
    "title.text": title,
    "title.font.size": 13,
    "title.x": 0,
    "title.xanchor": "left",
    "title.xref": "paper",
    "title.yref": "paper",
    "title.automargin": true,
  } as never);
  const dataUrl = await Plotly.toImage(gd as never, { format: "png" });
  await Plotly.relayout(gd as never, { "title.text": "" } as never);
  const bytes = Uint8Array.from(atob(dataUrl.split(",")[1]), (c) => c.charCodeAt(0));
  await api.saveChartImage(path, Array.from(bytes));
}

export function ChartView({ view, showZero, unit, conversionFactor = 1, defaultSavePath, title }: Props) {
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
    paper_bgcolor: "#ffffff",
    plot_bgcolor: "#ffffff",
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
      config={{
        displaylogo: false,
        responsive: true,
        // Full override (rather than modeBarButtonsToAdd, which always
        // appends at the end) so the save button keeps the default
        // toImage button's original leftmost slot.
        modeBarButtons: [
          [
            {
              name: "Save chart as PNG…",
              title: "Save chart as PNG…",
              icon: Plotly.Icons.camera,
              click: (gd: unknown) => {
                saveChartAsPng(gd, defaultSavePath, title);
              },
            },
          ],
          ["zoom2d", "pan2d", "select2d", "lasso2d"],
          ["zoomIn2d", "zoomOut2d", "autoScale2d", "resetScale2d"],
          ["toggleSpikelines", "hoverClosestCartesian", "hoverCompareCartesian"],
        ],
      } as never}
      useResizeHandler
      style={{ width: "100%", height: "100%" }}
    />
  );
}
