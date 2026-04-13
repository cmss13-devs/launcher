import { useEffect } from "react";
import { useShallow } from "zustand/react/shallow";
import {
  useAuthStore,
  useByondStore,
  useConfigStore,
  useServerStore,
  useSettingsStore,
  useSteamStore,
} from "../stores";

/**
 * Fire-and-forget application bootstrap: loads config/settings/steam, registers
 * backend event listeners, fetches OAuth providers when the hub variant is
 * detected, and reacts to authMode changes.
 */
export function useAppBootstrap() {
  const loadConfig = useConfigStore((s) => s.load);
  const hubApi = useConfigStore((s) => s.config?.urls.hub_api);

  const { initAuthListener, loadOauthProviders } = useAuthStore(
    useShallow((s) => ({
      initAuthListener: s.initListener,
      loadOauthProviders: s.loadOauthProviders,
    })),
  );

  const { initServerListener, initRelays } = useServerStore(
    useShallow((s) => ({
      initServerListener: s.initListener,
      initRelays: s.initRelays,
    })),
  );

  const { initByondListener, checkByondSession } = useByondStore(
    useShallow((s) => ({
      initByondListener: s.initListener,
      checkByondSession: s.checkSession,
    })),
  );

  const { loadSettings, authMode } = useSettingsStore(
    useShallow((s) => ({
      loadSettings: s.load,
      authMode: s.authMode,
    })),
  );

  const initializeSteam = useSteamStore((s) => s.initialize);

  // Initial load
  useEffect(() => {
    loadConfig();
    loadSettings();
    initializeSteam();
  }, [loadConfig, loadSettings, initializeSteam]);

  // Backend event listeners
  useEffect(() => {
    const unlistenAuthPromise = initAuthListener();
    const unlistenServerPromise = initServerListener();
    const unlistenRelaysPromise = initRelays();
    const unlistenByondPromise = initByondListener();

    return () => {
      unlistenAuthPromise.then((u) => u());
      unlistenServerPromise.then((u) => u());
      unlistenRelaysPromise.then((u) => u());
      unlistenByondPromise.then((u) => u());
    };
  }, [initAuthListener, initServerListener, initRelays, initByondListener]);

  // Load OAuth providers once the hub variant is detected
  useEffect(() => {
    if (hubApi) loadOauthProviders();
  }, [hubApi, loadOauthProviders]);

  // Recheck BYOND session when user switches to BYOND auth mode
  useEffect(() => {
    if (authMode === "byond") checkByondSession();
  }, [authMode, checkByondSession]);
}
