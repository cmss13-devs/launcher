import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import { commands } from "../bindings";
import { unwrap } from "../lib/unwrap";
import type { RenderingPipeline, WineStatus } from "../bindings";
import type { Platform, WineSetupProgress } from "../types";

const initialWineStatus: WineStatus = {
  installed: false,
  version: null,
  meets_minimum_version: false,
  winetricks_installed: false,
  prefix_initialized: false,
  webview2_installed: false,
  error: null,
};

export const useWine = () => {
  const [platform, setPlatform] = useState<Platform>("unknown");
  const [status, setStatus] = useState<WineStatus>(initialWineStatus);
  const [setupProgress, setSetupProgress] = useState<WineSetupProgress | null>(
    null,
  );
  const [isSettingUp, setIsSettingUp] = useState(false);
  const [setupError, setSetupError] = useState<string | null>(null);

  const refreshStatusRef = useRef<() => Promise<void>>();

  const checkStatus = useCallback(async (): Promise<WineStatus> => {
    try {
      const wineStatus = unwrap(await commands.checkWineStatus());
      setStatus(wineStatus);
      setSetupError(wineStatus.error);
      return wineStatus;
    } catch (err) {
      const errorStatus: WineStatus = {
        ...initialWineStatus,
        error: err instanceof Error ? err.message : String(err),
      };
      setStatus(errorStatus);
      setSetupError(errorStatus.error);
      return errorStatus;
    }
  }, []);

  refreshStatusRef.current = async () => {
    await checkStatus();
  };

  useEffect(() => {
    commands
      .getPlatform()
      .then((p) => setPlatform(p as Platform))
      .catch(() => setPlatform("unknown"));
  }, []);

  useEffect(() => {
    const unlisten = listen<WineSetupProgress>(
      "wine-setup-progress",
      (event) => {
        setSetupProgress(event.payload);

        if (event.payload.stage === "complete") {
          setIsSettingUp(false);
          refreshStatusRef.current?.();
        } else if (event.payload.stage === "error") {
          setIsSettingUp(false);
          setSetupError(event.payload.message);
        }
      },
    );

    return () => {
      unlisten.then((u) => u());
    };
  }, []);

  const initializePrefix = useCallback(async (pipeline: RenderingPipeline): Promise<boolean> => {
    setIsSettingUp(true);
    setSetupError(null);
    setSetupProgress({
      stage: "in_progress",
      progress: 0,
      message: "Starting Wine setup...",
    });

    try {
      unwrap(await commands.initializeWinePrefix(pipeline));
      await checkStatus();
      return true;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setSetupError(errorMessage);
      setSetupProgress({
        stage: "error",
        progress: 0,
        message: errorMessage,
      });
      setIsSettingUp(false);
      return false;
    }
  }, [checkStatus]);

  const resetPrefix = useCallback(async (): Promise<boolean> => {
    setIsSettingUp(true);
    setSetupError(null);
    setSetupProgress({
      stage: "in_progress",
      progress: 0,
      message: "Resetting Wine prefix...",
    });

    try {
      unwrap(await commands.resetWinePrefix());
      await checkStatus();
      return true;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setSetupError(errorMessage);
      setSetupProgress({
        stage: "error",
        progress: 0,
        message: errorMessage,
      });
      setIsSettingUp(false);
      return false;
    }
  }, [checkStatus]);

  const needsSetup =
    platform === "linux" &&
    (!status.prefix_initialized || !status.webview2_installed);

  const isReady =
    platform !== "linux" ||
    (status.prefix_initialized && status.webview2_installed);

  return {
    platform,
    status,
    setupProgress,
    isSettingUp,
    setupError,
    needsSetup,
    isReady,
    checkStatus,
    initializePrefix,
    resetPrefix,
  };
};
