import { faMinus, faXmark } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTranslation } from "react-i18next";
import { useConfigStore } from "../stores";

export const Titlebar = ({ title }: { title?: string }) => {
  const { t } = useTranslation();
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
      <div className="titlebar-title">
        {config?.logo && <img src={config.logo} alt="" className="titlebar-logo" />}
        {title || config?.product_name || t("titlebar.defaultTitle")}
      </div>
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
