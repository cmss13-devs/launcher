import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import type { ByondLoginResult } from "./types";

import {
  AccountInfo,
  AuthModal,
  ErrorNotifications,
  GameConnectionModal,
  RelayDropdown,
  ServerItem,
  SettingsModal,
  SinglePlayerPanel,
  SocialLinks,
  SteamAuthModal,
  Titlebar,
  UpdateNotification,
  WineSetupModal,
} from "./components";
import type { AuthModalState } from "./components/AuthModal";
import type { SteamAuthModalState } from "./components/SteamAuthModal";
import {
  ErrorProvider,
  useConnect,
  useError,
  useGameConnection,
  useWine,
} from "./hooks";
import {
  useAuthStore,
  useByondStore,
  useConfigStore,
  useServerStore,
  useSettingsStore,
  useSteamStore,
} from "./stores";

interface AutoConnectEvent {
  status:
    | "starting"
    | "waiting_for_servers"
    | "server_not_found"
    | "server_unavailable"
    | "auth_required"
    | "steam_linking_required"
    | "connecting"
    | "connected"
    | "error";
  server_name: string;
  message: string | null;
  linking_url: string | null;
}

const AppContent = () => {
  const { errors, dismissError, showError } = useError();

  const { config, load: loadConfig } = useConfigStore(
    useShallow((s) => ({
      config: s.config,
      load: s.load,
    })),
  );

  const {
    login,
    hubLogin,
    hubOAuthLogin,
    logout,
    initListener: initAuthListener,
  } = useAuthStore(
    useShallow((s) => ({
      login: s.login,
      hubLogin: s.hubLogin,
      hubOAuthLogin: s.hubOAuthLogin,
      logout: s.logout,
      initListener: s.initListener,
    })),
  );

  const {
    available: steamAvailable,
    initialize: initializeSteam,
    authenticate: authenticateSteam,
    logout: steamLogout,
    cancelAuthTicket: cancelSteamAuthTicket,
  } = useSteamStore(
    useShallow((s) => ({
      available: s.available,
      initialize: s.initialize,
      authenticate: s.authenticate,
      logout: s.logout,
      cancelAuthTicket: s.cancelAuthTicket,
    })),
  );

  const { initListener: initByondListener, checkSession: checkByondSession } = useByondStore(
    useShallow((s) => ({
      initListener: s.initListener,
      checkSession: s.checkSession,
    })),
  );

  const {
    servers,
    loading: serversLoading,
    error: serversError,
    relays,
    selectedRelay,
    setSelectedRelay,
    initListener: initServerListener,
    initRelays,
    lastUpdated,
  } = useServerStore(
    useShallow((s) => ({
      servers: s.servers,
      loading: s.loading,
      error: s.error,
      relays: s.relays,
      selectedRelay: s.selectedRelay,
      setSelectedRelay: s.setSelectedRelay,
      initListener: s.initListener,
      initRelays: s.initRelays,
      lastUpdated: s.lastUpdated,
    })),
  );

  const {
    authMode,
    setAuthMode,
    theme,
    devMode,
    fullscreenOverlay,
    load: loadSettings,
    saveAuthMode,
    saveTheme,
    saveFullscreenOverlay,
  } = useSettingsStore(
    useShallow((s) => ({
      authMode: s.authMode,
      setAuthMode: s.setAuthMode,
      theme: s.theme,
      devMode: s.devMode,
      fullscreenOverlay: s.fullscreenOverlay,
      load: s.load,
      saveAuthMode: s.saveAuthMode,
      saveTheme: s.saveTheme,
      saveFullscreenOverlay: s.saveFullscreenOverlay,
    })),
  );

  const [authModal, setAuthModal] = useState<{
    visible: boolean;
    state: AuthModalState;
    error?: string;
  }>({ visible: false, state: "idle", error: undefined });

  const [steamModal, setSteamModal] = useState<{
    visible: boolean;
    state: SteamAuthModalState;
    error?: string;
    linkingUrl?: string;
  }>({
    visible: false,
    state: "idle",
    error: undefined,
    linkingUrl: undefined,
  });

  const [settingsVisible, setSettingsVisible] = useState(false);
  const [relayDropdownOpen, setRelayDropdownOpen] = useState(false);

  const {
    gameConnectionState,
    connectedServerName,
    restartReason,
    closeGameConnectionModal,
    showGameConnectionModal,
  } = useGameConnection();

  const { connect } = useConnect();

  const {
    platform,
    status: wineStatus,
    setupProgress: wineSetupProgress,
    isSettingUp: wineIsSettingUp,
    needsSetup: wineNeedsSetup,
    checkStatus: checkWineStatus,
    initializePrefix: initializeWinePrefix,
    resetPrefix: resetWinePrefix,
  } = useWine();

  const [wineModalVisible, setWineModalVisible] = useState(false);

  const [autoConnecting, setAutoConnecting] = useState(false);
  const [pendingServerName, setPendingServerName] = useState<string | null>(
    null,
  );
  const [selectedTags, setSelectedTags] = useState<Set<string>>(new Set());
  const [showSingleplayer, setShowSingleplayer] = useState(false);
  const [show18Plus, setShow18Plus] = useState(false);
  const [showOffline, setShowOffline] = useState(false);
  const [showHubStatus, setShowHubStatus] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [filtersOpen, setFiltersOpen] = useState(false);
  const filtersRef = useRef<HTMLDivElement>(null);
  const [oauthProviders, setOauthProviders] = useState<string[]>([]);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (filtersRef.current && !filtersRef.current.contains(event.target as Node)) {
        setFiltersOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  useEffect(() => {
    if (!config?.urls.hub_api) return;
    invoke<string[]>("get_hub_oauth_providers")
      .then(setOauthProviders)
      .catch(() => {});
  }, [config?.urls.hub_api]);

  const categories = useMemo(() => {
    const tagSet = new Set<string>();
    for (const server of servers) {
      if (server.tags) {
        for (const tag of server.tags) {
          tagSet.add(tag);
        }
      }
    }
    const sorted = Array.from(tagSet).sort();

    const pvpIndex = sorted.findIndex((t) => t.toLowerCase() === "pvp");
    if (pvpIndex > 0) {
      const [pvp] = sorted.splice(pvpIndex, 1);
      sorted.unshift(pvp);
    }

    if (config?.features.singleplayer) {
      sorted.push("sandbox");
    }

    return sorted;
  }, [servers, config?.features.singleplayer]);

  const hasOffline = useMemo(() => servers.some((s) => s.status !== "available"), [servers]);
  const hasHubStatus = useMemo(() => servers.some((s) => s.hub_status.length > 0), [servers]);

  const filteredServers = useMemo(() => {
    const seen = new Set<string>();
    const uniqueServers = servers.filter((server) => {
      if (seen.has(server.url)) return false;
      seen.add(server.url);
      return true;
    });

    let filtered = selectedTags.size > 0
      ? uniqueServers.filter((server) =>
          server.tags?.some((t) => selectedTags.has(t)),
        )
      : uniqueServers;

    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter((server) =>
        server.name.toLowerCase().includes(query),
      );
    }

    if (!show18Plus) {
      filtered = filtered.filter((server) => !server.is_18_plus);
    }

    if (!showOffline && !config?.features.show_offline_servers) {
      filtered = filtered.filter((server) => server.status === "available");
    }

    return filtered.sort((a, b) => {
      const aOnline = a.status === "available";
      const bOnline = b.status === "available";
      if (aOnline !== bOnline) return aOnline ? -1 : 1;
      return b.players - a.players;
    });
  }, [servers, selectedTags, searchQuery, show18Plus, showOffline, config?.features.show_offline_servers]);

  useEffect(() => {
    document.documentElement.className = `theme-${theme}`;
  }, [theme]);

  useEffect(() => {
    if (platform === "linux") {
      checkWineStatus().then((status) => {
        if (!status.prefix_initialized || !status.webview2_installed) {
          setWineModalVisible(true);
        }
      });
    }
  }, [platform, checkWineStatus]);

  useEffect(() => {
    const unlistenAuthPromise = initAuthListener();
    const unlistenServerPromise = initServerListener();
    const unlistenRelaysPromise = initRelays();
    const unlistenByondPromise = initByondListener();

    return () => {
      unlistenAuthPromise.then((unlisten) => unlisten());
      unlistenServerPromise.then((unlisten) => unlisten());
      unlistenRelaysPromise.then((unlisten) => unlisten());
      unlistenByondPromise.then((unlisten) => unlisten());
    };
  }, [initAuthListener, initServerListener, initRelays, initByondListener]);

  useEffect(() => {
    const loadInitialState = async () => {
      const launcherConfig = await loadConfig();
      const settings = await loadSettings();
      const steamAvail = await initializeSteam();

      if (settings?.auth_mode) {
        setAuthMode(settings.auth_mode);
      } else if (steamAvail) {
        setAuthMode("steam");
      } else if (launcherConfig.oidc) {
        setAuthMode("oidc");
      } else {
        setAuthMode("byond");
      }
    };
    loadInitialState();
  }, [loadConfig, loadSettings, initializeSteam, setAuthMode]);

  useEffect(() => {
    if (authMode === "byond") {
      checkByondSession();
    }
  }, [authMode, checkByondSession]);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    const setupListener = async () => {
      unlisten = await listen<AutoConnectEvent>(
        "autoconnect-status",
        (event) => {
          const { status, server_name, message, linking_url } = event.payload;
          console.log(`[autoconnect] status=${status} server=${server_name}`);

          switch (status) {
            case "starting":
            case "waiting_for_servers":
            case "connecting":
              setAutoConnecting(true);
              break;

            case "auth_required":
              setAutoConnecting(false);
              setAuthModal({ visible: true, state: "idle", error: undefined });
              break;

            case "steam_linking_required":
              setAutoConnecting(false);
              setSteamModal({
                visible: true,
                state: "linking",
                error: undefined,
                linkingUrl: linking_url || undefined,
              });
              break;

            case "server_not_found":
            case "server_unavailable":
            case "error":
              setAutoConnecting(false);
              if (message) {
                showError(message);
              }
              break;

            case "connected":
              setAutoConnecting(false);
              break;
          }
        },
      );
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [showError]);

  const handleLogin = useCallback(async () => {
    setAuthModal({ visible: true, state: "loading", error: undefined });
    const result = await login();
    if (result.success) {
      setAuthModal({ visible: false, state: "idle", error: undefined });
    } else {
      setAuthModal({ visible: true, state: "error", error: result.error });
    }
  }, [login]);

  const handleLogout = useCallback(async () => {
    try {
      await logout();
    } catch (err) {
      showError(err instanceof Error ? err.message : String(err));
    }
  }, [logout, showError]);

  const handleByondLogin = useCallback(async () => {
    try {
      await invoke<ByondLoginResult>("start_byond_login");
    } catch (err) {
      showError(err instanceof Error ? err.message : String(err));
    }
  }, [showError]);

  const handleByondLogout = useCallback(async () => {
    try {
      await invoke("logout_byond_web");
    } catch (err) {
      showError(err instanceof Error ? err.message : String(err));
    }
  }, [showError]);

  const handleHubLogin = useCallback(
    async (username: string, password: string, totpCode?: string) => {
      setAuthModal({ visible: true, state: "loading", error: undefined });
      const result = await hubLogin(username, password, totpCode);
      if (result.success) {
        setAuthModal({ visible: false, state: "idle", error: undefined });
      } else if (result.requires2fa) {
        setAuthModal({ visible: true, state: "2fa", error: undefined });
      } else {
        setAuthModal({ visible: true, state: "error", error: result.error });
      }
    },
    [hubLogin],
  );

  const handleOAuthLogin = useCallback(
    async (provider: string) => {
      setAuthModal({ visible: true, state: "loading", error: undefined });
      const result = await hubOAuthLogin(provider);
      if (result.success) {
        setAuthModal({ visible: false, state: "idle", error: undefined });
      } else {
        setAuthModal({ visible: true, state: "error", error: result.error });
      }
    },
    [hubOAuthLogin],
  );

  const handleAuthModalClose = useCallback(() => {
    setAuthModal({ visible: false, state: "idle", error: undefined });
  }, []);

  const onLoginRequired = useCallback(() => {
    setAuthModal({ visible: true, state: "idle", error: undefined });
  }, []);

  const handleSteamAuthenticate = useCallback(
    async (createAccountIfMissing: boolean) => {
      setSteamModal((prev) => ({
        ...prev,
        state: "loading",
        error: undefined,
        linkingUrl: undefined,
      }));

      const result = await authenticateSteam(createAccountIfMissing);

      if (result?.success && result.access_token) {
        setSteamModal({
          visible: false,
          state: "idle",
          error: undefined,
          linkingUrl: undefined,
        });

        if (pendingServerName) {
          const serverToConnect = pendingServerName;
          setPendingServerName(null);
          connect(serverToConnect, "SteamAuthModal.afterAuth").catch((err) => {
            showError(err instanceof Error ? err.message : String(err));
          });
        }

        return result;
      }
      if (result?.requires_linking) {
        setSteamModal({
          visible: true,
          state: "linking",
          error: undefined,
          linkingUrl: result.linking_url || undefined,
        });
        return result;
      }
      setSteamModal({
        visible: true,
        state: "error",
        error: result?.error || "Authentication failed",
        linkingUrl: undefined,
      });
      return result;
    },
    [authenticateSteam, connect, pendingServerName, showError],
  );

  const handleSteamModalClose = useCallback(async () => {
    setSteamModal({
      visible: false,
      state: "idle",
      error: undefined,
      linkingUrl: undefined,
    });
    await cancelSteamAuthTicket();
  }, [cancelSteamAuthTicket]);

  const handleSteamLogout = useCallback(() => {
    steamLogout();
  }, [steamLogout]);

  const onSteamAuthRequired = useCallback(
    (serverName?: string) => {
      if (serverName) {
        setPendingServerName(serverName);
      }
      setSteamModal({
        visible: true,
        state: "idle",
        error: undefined,
        linkingUrl: undefined,
      });
      handleSteamAuthenticate(false);
    },
    [handleSteamAuthenticate],
  );

  const handleAuthModeChange = useCallback(
    async (mode: typeof authMode) => {
      try {
        await saveAuthMode(mode);
      } catch (err) {
        showError(err instanceof Error ? err.message : String(err));
      }
    },
    [saveAuthMode, showError],
  );

  const handleThemeChange = useCallback(
    async (newTheme: typeof theme) => {
      try {
        await saveTheme(newTheme);
      } catch (err) {
        showError(err instanceof Error ? err.message : String(err));
      }
    },
    [saveTheme, showError],
  );

  const handleWineSetup = useCallback(async () => {
    await initializeWinePrefix();
  }, [initializeWinePrefix]);

  const handleWineRetry = useCallback(async () => {
    await checkWineStatus();
  }, [checkWineStatus]);

  const handleWineModalClose = useCallback(() => {
    if (!wineIsSettingUp && !wineNeedsSetup) {
      setWineModalVisible(false);
    }
  }, [wineIsSettingUp, wineNeedsSetup]);

  const handleRelaySelect = useCallback(
    (relayId: string) => {
      setSelectedRelay(relayId);
      setRelayDropdownOpen(false);
    },
    [setSelectedRelay],
  );

  const toggleRelayDropdown = useCallback(() => {
    setRelayDropdownOpen((prev) => !prev);
  }, []);

  return (
    <div className="launcher-frame">
      {theme === "crt" && (
        <>
          <div className="crt-bezel" />
          <div className="crt" />
        </>
      )}
      <UpdateNotification />
      <ErrorNotifications errors={errors} onDismiss={dismissError} />
      <AuthModal
        {...authModal}
        loginPrompt={config?.strings.login_prompt ?? "Please log in to continue."}
        useHubAuth={config?.urls.hub_api != null}
        oauthProviders={oauthProviders}
        onLogin={handleLogin}
        onHubLogin={handleHubLogin}
        onOAuthLogin={handleOAuthLogin}
        onClose={handleAuthModalClose}
      />
      <SteamAuthModal
        {...steamModal}
        authProviderName={config?.strings.auth_provider_name ?? ""}
        onAuthenticate={handleSteamAuthenticate}
        onClose={handleSteamModalClose}
      />
      <SettingsModal
        visible={settingsVisible}
        authMode={authMode}
        theme={theme}
        steamAvailable={steamAvailable}
        devMode={devMode}
        platform={platform}
        wineStatus={wineStatus}
        isResettingWine={wineIsSettingUp}
        fullscreenOverlay={fullscreenOverlay}
        onAuthModeChange={handleAuthModeChange}
        onThemeChange={handleThemeChange}
        onFullscreenOverlayChange={saveFullscreenOverlay}
        onLoginRequired={onLoginRequired}
        onSteamAuthRequired={onSteamAuthRequired}
        onResetWinePrefix={resetWinePrefix}
        onClose={() => setSettingsVisible(false)}
      />
      <GameConnectionModal
        visible={showGameConnectionModal}
        state={gameConnectionState}
        serverName={connectedServerName}
        restartReason={restartReason}
        onClose={closeGameConnectionModal}
      />
      <WineSetupModal
        visible={wineModalVisible}
        status={wineStatus}
        progress={wineSetupProgress}
        isSettingUp={wineIsSettingUp}
        onSetup={handleWineSetup}
        onClose={handleWineModalClose}
        onRetry={handleWineRetry}
      />

      <div className="launcher">
        <Titlebar />

        <main className="main-content">
          <section className="section servers-section">
            {(config?.features.server_stats || config?.features.server_search || config?.features.server_filters || config?.features.singleplayer) && (
              <div className="server-header">
                {config?.features.server_stats && (
                  <div className="server-stats">
                    <span className="stat-label">Servers</span>
                    <span className="stat-value">{filteredServers.length}</span>
                    <span className="stat-label">Players</span>
                    <span className="stat-value">
                      {filteredServers
                        .filter((s) => s.status === "available")
                        .reduce((sum, s) => sum + s.players, 0)}
                    </span>
                  </div>
                )}
                {(config?.features.server_search || config?.features.server_filters || config?.features.singleplayer) && (
                  <div className="server-controls">
                    {config?.features.server_search && (
                      <input
                        type="text"
                        className="search-input"
                        placeholder="Search servers..."
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                      />
                    )}
                    {(config?.features.server_filters || categories.filter((c) => c !== "sandbox").length > 0) && (
                      <div className="filters-dropdown" ref={filtersRef}>
                        <button
                          type="button"
                          className={`filters-button${selectedTags.size > 0 ? " active" : ""}`}
                          onClick={() => setFiltersOpen(!filtersOpen)}
                        >
                          Filters{selectedTags.size > 0 ? ` (${selectedTags.size})` : ""}
                        </button>
                        {filtersOpen && (
                          <div className="filters-menu">
                            {config?.features.server_filters && (
                              <>
                                {hasHubStatus && (
                                  <label className="filter-checkbox">
                                    <input
                                      type="checkbox"
                                      checked={showHubStatus}
                                      onChange={(e) => setShowHubStatus(e.target.checked)}
                                    />
                                    <span>hub status</span>
                                  </label>
                                )}
                                <label className="filter-checkbox">
                                  <input
                                    type="checkbox"
                                    checked={show18Plus}
                                    onChange={(e) => setShow18Plus(e.target.checked)}
                                  />
                                  <span>18+ servers</span>
                                </label>
                                {hasOffline && (
                                  <label className="filter-checkbox">
                                    <input
                                      type="checkbox"
                                      checked={showOffline}
                                      onChange={(e) => setShowOffline(e.target.checked)}
                                    />
                                    <span>offline servers</span>
                                  </label>
                                )}
                              </>
                            )}
                            {config?.features.server_filters && categories.filter((c) => c !== "sandbox").length > 0 && (
                              <div className="filter-divider" />
                            )}
                            {categories
                              .filter((c) => c !== "sandbox")
                              .map((tag) => (
                                <label className="filter-checkbox" key={tag}>
                                  <input
                                    type="checkbox"
                                    checked={selectedTags.has(tag)}
                                    onChange={(e) => {
                                      const next = new Set(selectedTags);
                                      if (e.target.checked) {
                                        next.add(tag);
                                      } else {
                                        next.delete(tag);
                                      }
                                      setSelectedTags(next);
                                    }}
                                  />
                                  <span>{tag}</span>
                                </label>
                              ))}
                          </div>
                        )}
                      </div>
                    )}
                    {config?.features.singleplayer && (
                      <button
                        type="button"
                        className={`filters-button${showSingleplayer ? " active" : ""}`}
                        onClick={() => setShowSingleplayer(!showSingleplayer)}
                      >
                        Singleplayer
                      </button>
                    )}
                  </div>
                )}
              </div>
            )}
            {showSingleplayer && config?.features.singleplayer ? (
              <SinglePlayerPanel />
            ) : (
              <div className="server-list">
                {serversLoading && servers.length === 0 && (
                  <div className="server-loading">Loading servers...</div>
                )}
                {serversError && (
                  <div className="server-error">Error: {serversError}</div>
                )}
                {filteredServers.map((server) => (
                  <ServerItem
                    key={server.url}
                    server={server}
                    showHubStatus={showHubStatus}
                    onLoginRequired={onLoginRequired}
                    onSteamAuthRequired={onSteamAuthRequired}
                    autoConnecting={autoConnecting}
                  />
                ))}
              </div>
            )}
            {lastUpdated !== null && (
              <div className="refresh-bar">
                <div key={lastUpdated} className="refresh-bar-fill" />
              </div>
            )}
          </section>
        </main>

        <footer className="section footer">
          <div className="account-info">
            <AccountInfo
              onLogin={onLoginRequired}
              onLogout={handleLogout}
              onSteamLogout={handleSteamLogout}
              onByondLogin={handleByondLogin}
              onByondLogout={handleByondLogout}
            />
          </div>
          <div className="footer-actions">
            {config && config.social_links.length > 0 && (
              <SocialLinks links={config.social_links} />
            )}
            {config?.features.relay_selector && (
              <RelayDropdown
                relays={relays}
                selectedRelay={selectedRelay}
                isOpen={relayDropdownOpen}
                onToggle={toggleRelayDropdown}
                onSelect={handleRelaySelect}
              />
            )}
            <button
              type="button"
              className="button-secondary settings-button"
              onClick={() => setSettingsVisible(true)}
              title="Settings"
            >
              Settings
            </button>
          </div>
        </footer>
      </div>
    </div>
  );
};

const App = () => {
  return (
    <ErrorProvider>
      <AppContent />
    </ErrorProvider>
  );
};

export default App;
