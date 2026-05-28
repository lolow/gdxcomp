import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import witchHat from "../assets/witch-hat.png";

interface Props {
  onClose: () => void;
}

export function AboutDialog({ onClose }: Props) {
  const [version, setVersion] = useState<string>("");

  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
  }, []);

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal about-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <span>About gdxcomp</span>
          <button className="ghost" onClick={onClose}>✕</button>
        </div>
        <div className="modal-body about-body">
          <img src={witchHat} alt="witch hat" className="about-image" />
          <div className="about-info">
            <strong>gdxcomp</strong>{version && <> &nbsp;v{version}</>}
            <br />
            <span className="about-tagline">Plot &amp; compare GAMS GDX files</span>
            <br /><br />
            Author: <strong>Laurent Drouet</strong>
          </div>
        </div>
      </div>
    </div>
  );
}
