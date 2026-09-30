import { useMemo } from "react";
import Plotly from "plotly.js-basic-dist-min";
import createPlotlyComponent from "react-plotly.js/factory";
import { save } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import type { IterStyle } from "../iteration";
import type { ChartView as ChartViewData, Trace } from "../types";

const Plot = createPlotlyComponent(Plotly);

interface Props {
  view: ChartViewData;
  showZero: boolean;
  unit?: string | null;
  conversionFactor?: number;
  defaultSavePath: string;
  title: string;
  onError?: (message: string) => void;
  /** Convergence-set styling by trace name; replaces the per-file legend. */
  styles?: Map<string, IterStyle> | null;
}

// The title (variable, description, field, mapping) is baked into the
// exported image only — the on-screen chart already shows the same info in
// the app's own header, so a persistent Plotly title would duplicate it.
// It's added only to the off-screen copy that toImage renders, so the live
// chart is never touched and keeps the user's zoom and legend toggles.
//
// title.automargin does NOT grow the top margin to fit the title text here
// (verified against plotly.js-basic-dist-min 3.5.1 with title.xref/yref set
// to "paper", which is needed for left-alignment): it draws the title
// directly inside whatever margin.t already is, so it overlaps the plot's
// top edge — right on top of the zero line for charts whose data starts
// near zero. chartTitle (App.tsx) can be up to two lines (name + filter
// chips), so we reserve a fixed margin.t sized for two lines instead of
// relying on automargin.
const EXPORT_TITLE_MARGIN_T = 56;

interface GraphDiv extends HTMLElement {
  data: unknown[];
  layout: { margin?: Record<string, number> } & Record<string, unknown>;
}

async function saveChartAsPng(gd: GraphDiv, defaultSavePath: string, title: string) {
  let path = await save({
    defaultPath: defaultSavePath,
    filters: [{ name: "PNG image", extensions: ["png"] }],
  });
  if (!path) return;
  // Linux file dialogs do not append the filter's extension.
  if (!/\.png$/i.test(path)) path += ".png";
  const dataUrl = await Plotly.toImage(
    {
      data: gd.data,
      layout: {
        ...gd.layout,
        title: { text: title, font: { size: 13 }, x: 0, xanchor: "left", xref: "paper", yref: "paper" },
        margin: { ...gd.layout.margin, t: EXPORT_TITLE_MARGIN_T },
      },
    },
    { format: "png", width: gd.clientWidth, height: gd.clientHeight, scale: 2 },
  );
  const bytes = Uint8Array.from(atob(dataUrl.split(",")[1]), (c) => c.charCodeAt(0));
  await api.saveChartImage(path, Array.from(bytes));
}

function iterationTraces(traces: Trace[], styles: Map<string, IterStyle>, scale: (y: Trace["y"]) => Trace["y"]) {
  const styled = traces
    .filter((t) => styles.has(t.name))
    .map((t) => ({ t, s: styles.get(t.name)! }))
    .sort((a, b) => a.s.run - b.s.run || a.s.iter - b.s.iter);
  const lines = styled.map(({ t, s }) => ({
    type: "scatter",
    mode: "lines",
    name: `r${s.run} i${s.iter}`,
    x: t.x,
    y: scale(t.y),
    line: { color: s.color, dash: s.dash, width: 1.5 },
    legendgroup: `r${s.run}`,
    showlegend: false,
    connectgaps: false,
  }));
  // Empty traces that only draw legend entries: one per run (clicking it
  // toggles the whole legendgroup), plus one carrying the colour bar.
  const runs = [...new Map(styled.map(({ s }) => [s.run, s.dash])).entries()];
  const runLegend = runs.map(([run, dash]) => ({
    type: "scatter",
    mode: "lines",
    name: `run ${run}`,
    x: [null],
    y: [null],
    line: { color: "#888", dash },
    legendgroup: `r${run}`,
    hoverinfo: "skip",
  }));
  const allIters = [...styles.values()].map((s) => s.iter);
  const colorBar = {
    type: "scatter",
    mode: "markers",
    x: [null],
    y: [null],
    showlegend: false,
    hoverinfo: "skip",
    marker: {
      color: [Math.min(...allIters)],
      colorscale: "Viridis",
      cmin: Math.min(...allIters),
      cmax: Math.max(...allIters),
      showscale: true,
      colorbar: { title: { text: "iteration" }, thickness: 12 },
    },
  };
  return [...lines, ...runLegend, colorBar];
}

export function ChartView({ view, showZero, unit, conversionFactor = 1, defaultSavePath, title, onError, styles }: Props) {
  const data = useMemo(() => {
    const scale = (y: Trace["y"]) =>
      conversionFactor !== 1 ? y.map((v) => (v === null ? null : v * conversionFactor)) : y;
    if (styles && styles.size > 0) return iterationTraces(view.traces, styles, scale);
    return view.traces.map((t) => ({
      type: "scatter",
      mode: "lines+markers",
      name: t.name,
      x: t.x,
      y: scale(t.y),
      connectgaps: false,
    }));
  }, [view, conversionFactor, styles]);

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
              click: (gd: GraphDiv) => {
                saveChartAsPng(gd, defaultSavePath, title).catch((e) => onError?.(String(e)));
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
