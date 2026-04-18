import { useTranslation } from "react-i18next";
import { useSinglePlayer } from "../hooks";

const formatBytes = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
};

export const SinglePlayerPanel = () => {
  const { t } = useTranslation();
  const {
    status,
    latestRelease,
    loading,
    checking,
    error,
    updateAvailable,
    refresh,
    install,
    remove,
    launch,
  } = useSinglePlayer();

  if (checking) {
    return (
      <div className="singleplayer-panel">
        <div className="singleplayer-loading">
          <div className="singleplayer-spinner" />
          <p>{t("singleplayer.checkingStatus")}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="singleplayer-panel">
      <div className="singleplayer-header">
        <h3>{t("singleplayer.title")}</h3>
        <p className="singleplayer-description">
          {t("singleplayer.description")}
        </p>
      </div>

      {error && (
        <div className="singleplayer-error">
          <span>{error}</span>
          <button type="button" className="button-secondary" onClick={refresh}>
            {t("common.retry")}
          </button>
        </div>
      )}

      <div className="singleplayer-content">
        {loading ? (
          <div className="singleplayer-progress">
            <div className="singleplayer-spinner" />
            <p>{t("singleplayer.downloadingHint")}</p>
          </div>
        ) : (
          <div className="singleplayer-connect-area">
            <button
              type="button"
              className="button singleplayer-connect-button"
              disabled={!status.installed || loading}
              onClick={launch}
            >
              {t("common.connect")}
            </button>
            {!status.installed && latestRelease?.size && (
              <p className="singleplayer-size-hint">
                {t("singleplayer.downloadSize", { size: formatBytes(latestRelease.size) })}
              </p>
            )}
          </div>
        )}
      </div>

      <div className="singleplayer-footer">
        <div className="singleplayer-status-indicator">
          {status.installed ? (
            <span className={updateAvailable ? "status-warning" : "status-ok"}>
              {updateAvailable ? t("singleplayer.updateRequired") : t("singleplayer.upToDate", { version: status.version })}
            </span>
          ) : (
            <span>{t("singleplayer.notInstalled")}</span>
          )}
        </div>
        <div className="singleplayer-actions">
          {status.installed ? (
            <>
              {updateAvailable && (
                <button
                  type="button"
                  className="button"
                  onClick={install}
                  disabled={loading}
                >
                  {loading ? t("singleplayer.updating") : t("singleplayer.update")}
                </button>
              )}
              <button
                type="button"
                className="button-secondary"
                onClick={remove}
                disabled={loading}
              >
                {loading ? t("singleplayer.removing") : t("singleplayer.remove")}
              </button>
            </>
          ) : (
            <button
              type="button"
              className="button"
              onClick={install}
              disabled={loading || !latestRelease?.download_url}
            >
              {loading ? t("singleplayer.downloading") : t("singleplayer.download")}
            </button>
          )}
          <button
            type="button"
            className="button-secondary"
            onClick={refresh}
            disabled={loading}
          >
            {t("common.refresh")}
          </button>
        </div>
      </div>
    </div>
  );
};
