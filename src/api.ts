// Typed wrappers around the Tauri command layer. Keeping `invoke` calls in one
// place makes the rest of the UI free of stringly-typed command names.

import { invoke } from "@tauri-apps/api/core";
import type {
  DisplaySetup,
  FileMeta,
  GetViewResult,
  Session,
  SymbolMeta,
} from "./types";

export const api = {
  openGdx(paths: string[]): Promise<FileMeta[]> {
    return invoke("open_gdx", { paths });
  },
  openFolder(path: string): Promise<FileMeta[]> {
    return invoke("open_folder", { path });
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
  getView(setup: DisplaySetup): Promise<GetViewResult> {
    return invoke("get_view", { setup });
  },
  renameScenario(path: string, scenario: string): Promise<FileMeta[]> {
    return invoke("rename_scenario", { path, scenario });
  },
  resetScenarios(): Promise<FileMeta[]> {
    return invoke("reset_scenarios");
  },
  saveSession(session: Session): Promise<void> {
    return invoke("save_session", { session });
  },
  loadSession(): Promise<Session | null> {
    return invoke("load_session");
  },
};
