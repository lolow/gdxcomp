// Minimal ambient declarations for the Plotly modules we use, which ship
// without (or with incomplete) TypeScript types.
declare module "plotly.js-basic-dist-min" {
  const Plotly: {
    toImage: (gd: unknown, opts: Record<string, unknown>) => Promise<string>;
    relayout: (gd: unknown, update: Record<string, unknown>) => Promise<unknown>;
    Icons: Record<string, unknown>;
  };
  export default Plotly;
}

declare module "react-plotly.js/factory" {
  import type { ComponentType } from "react";
  // The created component accepts Plotly's figure props; we keep it loose.
  const createPlotlyComponent: (plotly: unknown) => ComponentType<Record<string, unknown>>;
  export default createPlotlyComponent;
}
