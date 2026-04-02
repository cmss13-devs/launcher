import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import type { ConnectionResult } from "../hooks/useConnect";
import { useByondStore, useConfigStore } from "../stores";
import type { AuthMode, ByondLoginResult, Platform, Theme, WineStatus } from "../types";
import { Modal, ModalCloseButton } from "./Modal";

interface AuthModeOptionProps {
  mode: AuthMode;
  currentMode: AuthMode;
  name: string;
  description: string;
  onChange: (mode: AuthMode) => void;
}

const AuthModeOption = ({
  mode,
  currentMode,
  name,
  description,
  onChange,
}: AuthModeOptionProps) => {
  return (
    <label
      className={`auth-mode-option ${currentMode === mode ? "selected" : ""}`}
    >
      <input
        type="radio"
        name="authMode"
        value={mode}
        checked={currentMode === mode}
        onChange={() => onChange(mode)}
      />
      <div className="auth-mode-info">
        <span className="auth-mode-name">{name}</span>
        <span className="auth-mode-desc">{description}</span>
      </div>
    </label>
  );
};

interface ThemeOptionProps {
  theme: Theme;
  currentTheme: Theme;
  name: string;
  description: string;
  onChange: (theme: Theme) => void;
}

const ThemeOption = ({
  theme,
  currentTheme,
  name,
  description,
  onChange,
}: ThemeOptionProps) => {
  return (
    <label
      className={`theme-option ${currentTheme === theme ? "selected" : ""}`}
    >
      <input
        type="radio"
        name="theme"
        value={theme}
        checked={currentTheme === theme}
        onChange={() => onChange(theme)}
      />
      <div className="theme-info">
        <span className="theme-name">{name}</span>
        <span className="theme-desc">{description}</span>
      </div>
    </label>
  );
};

interface WineSettingsProps {
  platform: Platform;
  wineStatus: WineStatus;
  isResetting: boolean;
  onResetPrefix: () => void;
}

const WineSettings = ({
  platform,
  wineStatus,
  isResetting,
  onResetPrefix,
}: WineSettingsProps) => {
  if (platform !== "linux") {
    return null;
  }

  return (
    <div className="settings-section">
      <h3>Wine Configuration</h3>
      <div className="wine-status-info">
        <p>
          <strong>Wine:</strong>{" "}
          {wineStatus.installed ? (
            <span className="status-ok">{wineStatus.version}</span>
          ) : (
            <span className="status-error">Not installed</span>
          )}
        </p>
        <p>
          <strong>Prefix:</strong>{" "}
          {wineStatus.prefix_initialized ? (
            <span className="status-ok">Initialized</span>
          ) : (
            <span className="status-warning">Not initialized</span>
          )}
        </p>
        <p>
          <strong>WebView2:</strong>{" "}
          {wineStatus.webview2_installed ? (
            <span className="status-ok">Installed</span>
          ) : (
            <span className="status-warning">Not installed</span>
          )}
        </p>
      </div>
      <button
        type="button"
        className="button-secondary"
        onClick={onResetPrefix}
        disabled={isResetting}
      >
        {isResetting ? "Resetting..." : "Reset Wine Prefix"}
      </button>
      <p className="settings-hint">
        Use this if you're experiencing issues. This will reinstall all
        dependencies.
      </p>
    </div>
  );
};

interface DevConnectSectionProps {
  onLoginRequired: () => void;
  onSteamAuthRequired: () => void;
}

const DevConnectSection = ({
  onLoginRequired,
  onSteamAuthRequired,
}: DevConnectSectionProps) => {
  const [url, setUrl] = useState("localhost:1337");
  const [version, setVersion] = useState("516.1667");
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleConnect = async () => {
    setConnecting(true);
    setError(null);

    try {
      const result = await invoke<ConnectionResult>("connect_to_url", {
        url,
        version,
        source: "DevConnectSection",
      });

      if (!result.success && result.auth_error) {
        if (result.auth_error.code === "auth_required") {
          onLoginRequired();
        } else if (result.auth_error.code === "steam_linking_required") {
          onSteamAuthRequired();
        } else {
          setError(result.auth_error.message);
        }
      } else if (!result.success) {
        setError(result.message);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setConnecting(false);
    }
  };

  return (
    <div className="dev-connect-section">
      <div className="dev-input-group">
        <label htmlFor="dev-url">Server URL</label>
        <input
          id="dev-url"
          type="text"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="localhost:1337"
        />
      </div>
      <div className="dev-input-group">
        <label htmlFor="dev-version">BYOND Version</label>
        <input
          id="dev-version"
          type="text"
          value={version}
          onChange={(e) => setVersion(e.target.value)}
          placeholder="516.1667"
        />
      </div>
      {error && <div className="dev-error">{error}</div>}
      <button
        type="button"
        className="button dev-connect-button"
        onClick={handleConnect}
        disabled={connecting || !url || !version}
      >
        {connecting ? "Connecting..." : "Connect"}
      </button>
    </div>
  );
};

interface SettingsModalProps {
  visible: boolean;
  authMode: AuthMode;
  theme: Theme;
  steamAvailable: boolean;
  devMode: boolean;
  platform: Platform;
  wineStatus: WineStatus;
  isResettingWine: boolean;
  fullscreenOverlay: boolean;
  onAuthModeChange: (mode: AuthMode) => void;
  onThemeChange: (theme: Theme) => void;
  onFullscreenOverlayChange: (enabled: boolean) => void;
  onLoginRequired: () => void;
  onSteamAuthRequired: () => void;
  onResetWinePrefix: () => void;
  onClose: () => void;
}

export const SettingsModal = ({
  visible,
  authMode,
  theme,
  steamAvailable,
  devMode,
  platform,
  wineStatus,
  isResettingWine,
  fullscreenOverlay,
  onAuthModeChange,
  onThemeChange,
  onFullscreenOverlayChange,
  onLoginRequired,
  onSteamAuthRequired,
  onResetWinePrefix,
  onClose,
}: SettingsModalProps) => {
  const config = useConfigStore((s) => s.config);
  const byondWebUsername = useByondStore((s) => s.username);
  const byondPagerRunning = useByondStore((s) => s.pagerRunning);
  const checkByondStatus = useByondStore((s) => s.checkStatus);

  const [appVersion, setAppVersion] = useState<string>("");
  const [byondLoginState, setByondLoginState] = useState<
    "idle" | "loading" | "success" | "error"
  >("idle");
  const [byondLoginError, setByondLoginError] = useState<string | null>(null);

  useEffect(() => {
    getVersion().then(setAppVersion);
  }, []);

  useEffect(() => {
    if (visible && authMode === "byond") {
      checkByondStatus();
    }
  }, [visible, authMode, checkByondStatus]);

  const handleByondWebLogin = async () => {
    setByondLoginState("loading");
    setByondLoginError(null);
    try {
      const result = await invoke<ByondLoginResult>("start_byond_login");
      console.log("BYOND login successful, username:", result.username);
      setByondLoginState("success");
    } catch (err) {
      const error = err instanceof Error ? err.message : String(err);
      console.error("BYOND login failed:", error);
      setByondLoginError(error);
      setByondLoginState("error");
    }
  };

  return (
    <Modal
      visible={visible}
      onClose={onClose}
      className="settings-modal"
      overlayClassName="settings-modal-overlay"
      closeOnOverlayClick
    >
      <div className="settings-modal-header">
        <h2>Settings</h2>
        <button
          type="button"
          className="help-link"
          onClick={() =>
            invoke("open_url", { url: config?.urls.help_url || "https://github.com/cmss13-devs/cm-launcher/issues" })
          }
          title="Report an issue"
        >
          Help
        </button>
        <ModalCloseButton onClick={onClose} />
      </div>
      <div className="settings-modal-content">
        <div className="settings-section">
          <h3>Appearance</h3>
          <p className="settings-description">
            Choose a visual theme for the launcher.
          </p>
          <div className="theme-options">
            <ThemeOption
              theme="tgui"
              currentTheme={theme}
              name="TGUI"
              description="Modern flat interface"
              onChange={onThemeChange}
            />
            <ThemeOption
              theme="crt"
              currentTheme={theme}
              name="CRT Terminal"
              description="Classic green CRT terminal"
              onChange={onThemeChange}
            />
          </div>
        </div>

        <div className="settings-section">
          <h3>Authentication Mode</h3>
          <p className="settings-description">
            Choose how you want to authenticate when connecting to servers.
          </p>
          {authMode === "byond" && byondPagerRunning === false && !byondWebUsername && (
            <div className="byond-login-section">
              <div className="auth-mode-warning">
                BYOND pager is not running. You can either open BYOND and log
                in, or use web login below.
              </div>
              <div className="byond-web-login">
                {byondLoginState === "idle" && (
                  <>
                    <button
                      type="button"
                      className="button"
                      onClick={handleByondWebLogin}
                    >
                      Login to BYOND
                    </button>
                    <button
                      type="button"
                      className="button-secondary"
                      onClick={() => invoke("open_url", { url: "https://secure.byond.com/Join" })}
                    >
                      Create Account
                    </button>
                  </>
                )}
                {byondLoginState === "loading" && (
                  <p className="byond-login-status">
                    Waiting for BYOND login... (check the login window)
                  </p>
                )}
                {byondLoginState === "success" && (
                  <p className="byond-login-status success">
                    Logged in to BYOND successfully!
                    {byondWebUsername && ` (${byondWebUsername})`}
                  </p>
                )}
                {byondLoginState === "error" && (
                  <div className="byond-login-error">
                    <p>Login failed: {byondLoginError}</p>
                    <button
                      type="button"
                      className="button-secondary"
                      onClick={handleByondWebLogin}
                    >
                      Try Again
                    </button>
                  </div>
                )}
              </div>
            </div>
          )}
          <div className="auth-mode-options">
            {config?.urls.hub_api && (
              <AuthModeOption
                mode="hub"
                currentMode={authMode}
                name="SS13Hub Authentication"
                description="Login with your SS13Hub account"
                onChange={onAuthModeChange}
              />
            )}
            {config?.oidc && (
              <AuthModeOption
                mode="oidc"
                currentMode={authMode}
                name={`${config.strings.auth_provider_name} Authentication`}
                description={`Login with your ${config.strings.auth_provider_name} account for server access`}
                onChange={onAuthModeChange}
              />
            )}
            {steamAvailable && (
              <AuthModeOption
                mode="steam"
                currentMode={authMode}
                name="Steam Authentication"
                description="Login with your Steam account"
                onChange={onAuthModeChange}
              />
            )}
            <AuthModeOption
              mode="byond"
              currentMode={authMode}
              name="BYOND Authentication"
              description="Login with your BYOND account, or via the pager"
              onChange={onAuthModeChange}
            />
          </div>
        </div>

        {steamAvailable && (
          <div className="settings-section">
            <h3>Steam Integration</h3>
            <label className="toggle-setting">
              <input
                type="checkbox"
                checked={fullscreenOverlay}
                onChange={(e) => onFullscreenOverlayChange(e.target.checked)}
              />
              <div className="toggle-info">
                <span className="toggle-name">Fullscreen Overlay</span>
                <span className="toggle-desc">
                  If the Steam Overlay should appear fullscreen
                </span>
              </div>
            </label>
          </div>
        )}

        <WineSettings
          platform={platform}
          wineStatus={wineStatus}
          isResetting={isResettingWine}
          onResetPrefix={onResetWinePrefix}
        />

        {devMode && (
          <div className="settings-section dev-section">
            <h3>Developer Options</h3>
            <p className="settings-description">
              Connect to a local development server.
            </p>
            <DevConnectSection
              onLoginRequired={onLoginRequired}
              onSteamAuthRequired={onSteamAuthRequired}
            />
          </div>
        )}
      </div>
      <div className="settings-modal-footer">
        <span className="version-info">v{appVersion}</span>
      </div>
    </Modal>
  );
};
