import { faMinus, faXmark } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useConfigStore } from "../stores";

export const Titlebar = () => {
  const config = useConfigStore((s) => s.config);
  const handleMinimize = async () => {
    const window = getCurrentWindow();
    await window.minimize();
  };

  const handleClose = async () => {
    const window = getCurrentWindow();
    await window.close();
  };

  return (
    <div className="titlebar" data-tauri-drag-region>
      <div className="titlebar-title">{config?.product_name || "SS13 Launcher"}</div>
      <div className="titlebar-buttons">
        <button
          type="button"
          className="titlebar-button"
          onClick={handleMinimize}
        >
          <FontAwesomeIcon icon={faMinus} className="titlebar-icon" />
        </button>
        <button
          type="button"
          className="titlebar-button titlebar-close"
          onClick={handleClose}
        >
          <FontAwesomeIcon icon={faXmark} className="titlebar-icon" />
        </button>
      </div>
    </div>
  );
};
