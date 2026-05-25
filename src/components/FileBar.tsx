import { open } from "@tauri-apps/plugin-dialog";
import type { FileMeta } from "../types";

interface Props {
  files: FileMeta[];
  onOpen: (paths: string[]) => void;
  onRemove: (path: string) => void;
}

export function FileBar({ files, onOpen, onRemove }: Props) {
  async function pick() {
    const selected = await open({
      multiple: true,
      filters: [{ name: "GDX files", extensions: ["gdx"] }],
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    onOpen(paths);
  }

  return (
    <div className="section">
      <h2>Files</h2>
      <button className="primary" onClick={pick} style={{ width: "100%", marginBottom: 8 }}>
        + Add GDX…
      </button>
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
