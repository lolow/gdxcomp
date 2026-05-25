// Typed wrappers around the Tauri command layer. Keeping `invoke` calls in one
// place makes the rest of the UI free of stringly-typed command names.

import { invoke } from "@tauri-apps/api/core";
import type { DisplaySetup, FileMeta, PlotView, SymbolMeta } from "./types";

export const api = {
  openGdx(paths: string[]): Promise<FileMeta[]> {
    return invoke("open_gdx", { paths });
  },
  removeGdx(path: string): Promise<FileMeta[]> {
    return invoke("remove_gdx", { path });
  },
  listFiles(): Promise<FileMeta[]> {
    return invoke("list_files");
  },
  commonSymbols(): Promise<SymbolMeta[]> {
    return invoke("common_symbols_cmd");
  },
  distinctKeys(symbol: string, dim: number): Promise<string[]> {
    return invoke("distinct_keys", { symbol, dim });
  },
  getView(setup: DisplaySetup): Promise<PlotView> {
    return invoke("get_view", { setup });
  },
  saveSetup(path: string, setup: DisplaySetup): Promise<void> {
    return invoke("save_setup", { path, setup });
  },
  loadSetup(path: string): Promise<DisplaySetup> {
    return invoke("load_setup", { path });
  },
};
