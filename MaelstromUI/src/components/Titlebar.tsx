import { Minus, Square, X, Hexagon } from "lucide-react";
import { WindowManager } from "@tauri-apps/api/window";

function Titlebar() {
  const appWindow = WindowManager.getCurrentWindow();
  
  return (
    <div className="titlebar">
      <div className="titlebar-title">
        <Hexagon size={14} color="var(--accent-solid)" /> Aaroneous Command Center
      </div>
      <div className="titlebar-actions">
        <button className="titlebar-btn" onClick={() => appWindow.hide()}>
          <Minus size={16} />
        </button>
        <button className="titlebar-btn" onClick={() => appWindow.toggleMaximize()}>
          <Square size={14} />
        </button>
        <button className="titlebar-btn close" onClick={() => appWindow.hide()}>
          <X size={16} />
        </button>
      </div>
    </div>
  );
}

export default Titlebar;