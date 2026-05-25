import { open, save } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import type { DisplaySetup } from "../types";

interface Props {
  setup: DisplaySetup | null;
  filePaths: string[];
  onImport: (setup: DisplaySetup) => void;
  onError: (message: string) => void;
}

export function SetupToolbar({ setup, filePaths, onImport, onError }: Props) {
  async function exportSetup() {
    if (!setup) return;
    try {
      const path = await save({
        defaultPath: `${setup.symbol || "view"}.gdxcomp.json`,
        filters: [{ name: "gdxcomp setup", extensions: ["json"] }],
      });
      if (!path) return;
      // Record the current file selection so the view can be reproduced.
      await api.saveSetup(path, { ...setup, files: filePaths });
    } catch (e) {
      onError(String(e));
    }
  }

  async function importSetup() {
    try {
      const path = await open({
        multiple: false,
        filters: [{ name: "gdxcomp setup", extensions: ["json"] }],
      });
      if (!path || Array.isArray(path)) return;
      const loaded = await api.loadSetup(path);
      onImport(loaded);
    } catch (e) {
      onError(String(e));
    }
  }

  return (
    <div className="row-gap">
      <button onClick={importSetup}>Import setup…</button>
      <button onClick={exportSetup} disabled={!setup}>
        Export setup…
      </button>
    </div>
  );
}
