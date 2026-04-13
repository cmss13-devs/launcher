import { useCallback, useEffect, useState } from "react";
import { useShallow } from "zustand/react/shallow";

import {
  AccountInfo,
  AuthModal,
  ErrorNotifications,
  GameConnectionModal,
  RelayDropdown,
  ServerFilterPanel,
  ServerItem,
  SettingsModal,
  SinglePlayerPanel,
  SocialLinks,
  SteamAuthModal,
  Titlebar,
  UpdateNotification,
  WineSetupModal,
} from "./components";
import {
  ErrorProvider,
  useAppBootstrap,
  useAuthHandlers,
  useAutoConnect,
  useError,
  useGameConnection,
  useServerFilters,
  useSteamLinking,
  useWine,
} from "./hooks";
import {
  useAuthStore,
  useConfigStore,
  useServerStore,
  useSettingsStore,
  useSteamStore,
} from "./stores";

const AppContent = () => {
  const { errors, dismissError, showError } = useError();

  useAppBootstrap();

  const config = useConfigStore((s) => s.config);
  const oauthProviders = useAuthStore((s) => s.oauthProviders);
  const steamAvailable = useSteamStore((s) => s.available);

  const {
    authModal,
    handleLogin,
    handleLogout,
    handleByondLogin,
    handleByondLogout,
    handleHubLogin,
    handleOAuthLogin,
    handleSteamLogin,
    handleAuthModalClose,
    onLoginRequired,
  } = useAuthHandlers();

  const {
    steamModal,
    handleSteamAuthenticate,
    handleSteamModalClose,
    handleSteamLogout,
    onSteamAuthRequired,
    onAutoConnectLinkingRequired,
  } = useSteamLinking();

  const {
    servers,
    loading: serversLoading,
    error: serversError,
    relays,
    selectedRelay,
    setSelectedRelay,
    lastUpdated,
  } = useServerStore(
    useShallow((s) => ({
      servers: s.servers,
      loading: s.loading,
      error: s.error,
      relays: s.relays,
      selectedRelay: s.selectedRelay,
      setSelectedRelay: s.setSelectedRelay,
      lastUpdated: s.lastUpdated,
    })),
  );

  const { authMode, theme, devMode, saveAuthMode, saveTheme } = useSettingsStore(
    useShallow((s) => ({
      authMode: s.authMode,
      theme: s.theme,
      devMode: s.devMode,
      saveAuthMode: s.saveAuthMode,
      saveTheme: s.saveTheme,
    })),
  );

  const {
    gameConnectionState,
    connectedServerName,
    restartReason,
    closeGameConnectionModal,
    showGameConnectionModal,
  } = useGameConnection();

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

  const [settingsVisible, setSettingsVisible] = useState(false);
  const [relayDropdownOpen, setRelayDropdownOpen] = useState(false);
  const [wineModalVisible, setWineModalVisible] = useState(false);

  const filters = useServerFilters(servers, config);
  const { showHubStatus, showSingleplayer, filteredServers } = filters;

  const autoConnecting = useAutoConnect({
    onLoginRequired,
    onAutoConnectLinkingRequired,
    showError,
  });

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
        steamAvailable={steamAvailable}
        registerUrl={config?.urls.register_url}
        onLogin={handleLogin}
        onHubLogin={handleHubLogin}
        onOAuthLogin={handleOAuthLogin}
        onSteamLogin={handleSteamLogin}
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
        onAuthModeChange={handleAuthModeChange}
        onThemeChange={handleThemeChange}
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
            {config && (
              <ServerFilterPanel
                features={config.features}
                filters={filters}
                serverCount={filteredServers.length}
                playerCount={filteredServers
                  .filter((s) => s.status === "available")
                  .reduce((sum, s) => sum + (s.players ?? 0), 0)}
              />
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
