import { open } from "@tauri-apps/plugin-dialog";
import type { FileMeta } from "../types";

interface Props {
  files: FileMeta[];
  onOpen: (paths: string[]) => void;
  onOpenFolder: (path: string) => void;
  onRemove: (path: string) => void;
}

export function FileBar({ files, onOpen, onOpenFolder, onRemove }: Props) {
  async function pick() {
    const selected = await open({
      multiple: true,
      filters: [{ name: "GDX files", extensions: ["gdx"] }],
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    onOpen(paths);
  }

  async function pickFolder() {
    const selected = await open({ directory: true });
    if (!selected) return;
    const path = Array.isArray(selected) ? selected[0] : selected;
    onOpenFolder(path);
  }

  return (
    <div className="section">
      <h2>Files</h2>
      <div style={{ display: "flex", gap: 4, marginBottom: 8 }}>
        <button className="primary" onClick={pick} style={{ flex: 1 }}>
          + Add GDX…
        </button>
        <button className="primary" onClick={pickFolder} title="Open all GDX files in a folder">
          📁
        </button>
      </div>
      {files.length === 0 && <div className="empty">No files loaded</div>}
      {files.map((f) => (
        <div className="file" key={f.path} title={f.path}>
          <span className="label">{f.label}</span>
          <button className="ghost" onClick={() => onRemove(f.path)} title="Remove">
            ✕
          </button>
        </div>
      ))}
    </div>
  );
}
