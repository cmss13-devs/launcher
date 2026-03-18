import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { useCallback, useEffect, useState } from "react";

interface UpdateInfo {
  version: string;
  body?: string;
}

export const UpdateNotification = () => {
  const [updateAvailable, setUpdateAvailable] = useState<UpdateInfo | null>(null);
  const [downloading, setDownloading] = useState(false);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const checkForUpdate = async () => {
      try {
        const update = await check();
        if (update) {
          setUpdateAvailable({
            version: update.version,
            body: update.body,
          });
        }
      } catch (err) {
        console.error("Failed to check for updates:", err);
      }
    };

    checkForUpdate();
  }, []);

  const handleUpdate = useCallback(async () => {
    setDownloading(true);
    setError(null);
    setProgress(0);

    try {
      const update = await check();
      if (!update) {
        setError("Update no longer available");
        setDownloading(false);
        return;
      }

      let downloaded = 0;
      let contentLength = 0;

      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            contentLength = event.data.contentLength ?? 0;
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            if (contentLength > 0) {
              setProgress(Math.round((downloaded / contentLength) * 100));
            }
            break;
          case "Finished":
            setProgress(100);
            break;
        }
      });

      await relaunch();
    } catch (err) {
      console.error("Failed to install update:", err);
      setError(err instanceof Error ? err.message : String(err));
      setDownloading(false);
    }
  }, []);

  const handleDismiss = useCallback(() => {
    setUpdateAvailable(null);
  }, []);

  if (!updateAvailable) {
    return null;
  }

  return (
    <div className="update-notification">
      <div className="update-content">
        <span className="update-message">
          Update available: v{updateAvailable.version}
        </span>
        {downloading ? (
          <div className="update-progress">
            <div className="update-progress-bar" style={{ width: `${progress}%` }} />
            <span className="update-progress-text">{progress}%</span>
          </div>
        ) : (
          <div className="update-actions">
            <button
              type="button"
              className="update-button"
              onClick={handleUpdate}
            >
              Install
            </button>
            <button
              type="button"
              className="update-dismiss"
              onClick={handleDismiss}
            >
              Later
            </button>
          </div>
        )}
      </div>
      {error && <div className="update-error">{error}</div>}
    </div>
  );
};
