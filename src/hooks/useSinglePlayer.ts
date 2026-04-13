import { useCallback, useEffect, useState } from "react";
import { commands } from "../bindings";
import { unwrap } from "../lib/unwrap";
import type { ReleaseInfo, SinglePlayerStatus } from "../bindings";

const initialStatus: SinglePlayerStatus = {
  installed: false,
  version: null,
  release_tag: null,
  path: null,
};

export const useSinglePlayer = () => {
  const [status, setStatus] = useState<SinglePlayerStatus>(initialStatus);
  const [latestRelease, setLatestRelease] = useState<ReleaseInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [checking, setChecking] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const checkStatus = useCallback(async (): Promise<SinglePlayerStatus> => {
    try {
      const result = unwrap(await commands.getSingleplayerStatus());
      setStatus(result);
      return result;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      return initialStatus;
    }
  }, []);

  const checkLatestRelease = useCallback(async (): Promise<ReleaseInfo | null> => {
    try {
      const result = unwrap(await commands.getLatestSingleplayerRelease());
      setLatestRelease(result);
      return result;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      return null;
    }
  }, []);

  const refresh = useCallback(async () => {
    setChecking(true);
    setError(null);
    await Promise.all([checkStatus(), checkLatestRelease()]);
    setChecking(false);
  }, [checkStatus, checkLatestRelease]);

  const install = useCallback(async (): Promise<boolean> => {
    setLoading(true);
    setError(null);

    try {
      const result = unwrap(await commands.installSingleplayer());
      setStatus(result);
      return true;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      return false;
    } finally {
      setLoading(false);
    }
  }, []);

  const remove = useCallback(async (): Promise<boolean> => {
    setLoading(true);
    setError(null);

    try {
      unwrap(await commands.deleteSingleplayer());
      setStatus(initialStatus);
      return true;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      return false;
    } finally {
      setLoading(false);
    }
  }, []);

  const launch = useCallback(async (): Promise<boolean> => {
    setLoading(true);
    setError(null);

    try {
      unwrap(await commands.launchSingleplayer());
      return true;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      return false;
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const updateAvailable =
    status.installed &&
    latestRelease !== null &&
    status.version !== null &&
    status.version !== latestRelease.tag_name;

  return {
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
  };
};
