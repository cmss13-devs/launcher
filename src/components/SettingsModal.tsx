import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "../bindings";
import { useAuthFlow } from "../hooks";
import { unwrap } from "../lib/unwrap";
import { getAvailableLocales } from "../i18n";
import { useByondStore, useConfigStore, useSettingsStore } from "../stores";
import type { AuthMode, RenderingPipeline, Theme, WineStatus } from "../bindings";
import type { Platform } from "../types";
import { faChevronDown, faChevronUp } from "@fortawesome/free-solid-svg-icons";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { Modal, ModalCloseButton } from "./Modal";

interface LocaleDropdownProps {
  value: string | null;
  options: { value: string; label: string }[];
  autoLabel: string;
  onChange: (value: string | null) => void;
}

const LocaleDropdown = ({ value, options, autoLabel, onChange }: LocaleDropdownProps) => {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  const selectedLabel = value
    ? options.find((o) => o.value === value)?.label ?? value
    : autoLabel;

  return (
    <div className="locale-dropdown" ref={ref}>
      <button
        type="button"
        className="locale-dropdown-button"
        onClick={() => setOpen((prev) => !prev)}
      >
        <span className="locale-dropdown-value">{selectedLabel}</span>
        <span className="locale-dropdown-arrow">
          <FontAwesomeIcon icon={open ? faChevronUp : faChevronDown} />
        </span>
      </button>
      {open && (
        <div className="locale-dropdown-menu">
          <button
            type="button"
            className={`locale-dropdown-item ${value === null ? "selected" : ""}`}
            onClick={() => { onChange(null); setOpen(false); }}
          >
            {autoLabel}
          </button>
          {options.map((opt) => (
            <button
              key={opt.value}
              type="button"
              className={`locale-dropdown-item ${value === opt.value ? "selected" : ""}`}
              onClick={() => { onChange(opt.value); setOpen(false); }}
            >
              {opt.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
};

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
  renderingPipeline: RenderingPipeline;
  isResetting: boolean;
  onResetPrefix: () => void;
  onRenderingPipelineChange: (pipeline: RenderingPipeline) => void;
}

const WineSettings = ({
  platform,
  wineStatus,
  renderingPipeline,
  isResetting,
  onResetPrefix,
  onRenderingPipelineChange,
}: WineSettingsProps) => {
  const { t } = useTranslation();

  if (platform !== "linux") {
    return null;
  }

  return (
    <div className="settings-section">
      <h3>{t("wine.configuration")}</h3>
      <div className="wine-status-info">
        <p>
          <strong>{t("wine.wineLabel")}</strong>{" "}
          {wineStatus.installed ? (
            <span className="status-ok">{wineStatus.version}</span>
          ) : (
            <span className="status-error">{t("wine.notInstalled")}</span>
          )}
        </p>
        <p>
          <strong>{t("wine.prefixLabel")}</strong>{" "}
          {wineStatus.prefix_initialized ? (
            <span className="status-ok">{t("wine.initialized")}</span>
          ) : (
            <span className="status-warning">{t("wine.notInitialized")}</span>
          )}
        </p>
        <p>
          <strong>{t("wine.webview2Label")}</strong>{" "}
          {wineStatus.webview2_installed ? (
            <span className="status-ok">{t("wine.installed")}</span>
          ) : (
            <span className="status-warning">{t("wine.notInstalled")}</span>
          )}
        </p>
      </div>
      <div className="wine-rendering-pipeline">
        <h4>{t("wine.renderingPipeline")}</h4>
        <p className="settings-description">{t("wine.renderingPipelineDesc")}</p>
        <div className="theme-options">
          <label className={`theme-option ${renderingPipeline === "dxvk" ? "selected" : ""}`}>
            <input
              type="radio"
              name="renderingPipeline"
              value="dxvk"
              checked={renderingPipeline === "dxvk"}
              onChange={() => onRenderingPipelineChange("dxvk")}
            />
            <div className="theme-info">
              <span className="theme-name">{t("wine.dxvkName")}</span>
              <span className="theme-desc">{t("wine.dxvkDesc")}</span>
            </div>
          </label>
          <label className={`theme-option ${renderingPipeline === "wined3d" ? "selected" : ""}`}>
            <input
              type="radio"
              name="renderingPipeline"
              value="wined3d"
              checked={renderingPipeline === "wined3d"}
              onChange={() => onRenderingPipelineChange("wined3d")}
            />
            <div className="theme-info">
              <span className="theme-name">{t("wine.wined3dName")}</span>
              <span className="theme-desc">{t("wine.wined3dDesc")}</span>
            </div>
          </label>
        </div>
        <p className="settings-hint">{t("wine.renderingPipelineHint")}</p>
      </div>
      <button
        type="button"
        className="button-secondary"
        onClick={onResetPrefix}
        disabled={isResetting}
      >
        {isResetting ? t("wine.resetting") : t("wine.resetPrefix")}
      </button>
      <p className="settings-hint">
        {t("wine.resetHint")}
      </p>
    </div>
  );
};

const DevConnectSection = () => {
  const { t } = useTranslation();
  const { onLoginRequired, onSteamAuthRequired } = useAuthFlow();
  const [url, setUrl] = useState("localhost:1337");
  const [version, setVersion] = useState("516.1667");
  const [connecting, setConnecting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleConnect = async () => {
    setConnecting(true);
    setError(null);

    try {
      const result = unwrap(await commands.connectToUrl(url, version, "DevConnectSection"));

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
        <label htmlFor="dev-url">{t("settings.serverUrl")}</label>
        <input
          id="dev-url"
          type="text"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="localhost:1337"
        />
      </div>
      <div className="dev-input-group">
        <label htmlFor="dev-version">{t("settings.byondVersion")}</label>
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
        {connecting ? t("settings.connecting") : t("common.connect")}
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
  renderingPipeline: RenderingPipeline;
  isResettingWine: boolean;
  onAuthModeChange: (mode: AuthMode) => void;
  onThemeChange: (theme: Theme) => void;
  onRenderingPipelineChange: (pipeline: RenderingPipeline) => void;
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
  renderingPipeline,
  isResettingWine,
  onAuthModeChange,
  onThemeChange,
  onRenderingPipelineChange,
  onResetWinePrefix,
  onClose,
}: SettingsModalProps) => {
  const { t } = useTranslation();
  const config = useConfigStore((s) => s.config);
  const byondWebUsername = useByondStore((s) => s.username);
  const byondPagerRunning = useByondStore((s) => s.pagerRunning);
  const checkByondStatus = useByondStore((s) => s.checkStatus);
  const locale = useSettingsStore((s) => s.locale);
  const saveLocale = useSettingsStore((s) => s.saveLocale);

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
      const result = unwrap(await commands.startByondLogin());
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
      <div className="modal-header">
        <h2>{t("settings.title")}</h2>
        <button
          type="button"
          className="help-link"
          onClick={() =>
            commands.openUrl(config?.urls.help_url || "https://github.com/cmss13-devs/cm-launcher/issues")
          }
          title={t("settings.reportIssue")}
        >
          {t("common.help")}
        </button>
        <ModalCloseButton onClick={onClose} />
      </div>
      <div className="settings-modal-content">
        <div className="settings-section">
          <h3>{t("settings.appearance")}</h3>
          <p className="settings-description">
            {t("settings.themeDescription")}
          </p>
          <div className="theme-options">
            <ThemeOption
              theme="tgui"
              currentTheme={theme}
              name={t("settings.tguiName")}
              description={t("settings.tguiDescription")}
              onChange={onThemeChange}
            />
            <ThemeOption
              theme="crt"
              currentTheme={theme}
              name={t("settings.crtName")}
              description={t("settings.crtDescription")}
              onChange={onThemeChange}
            />
          </div>
        </div>

        <div className="settings-section">
          <h3>{t("settings.language")}</h3>
          <p className="settings-description">
            {t("settings.languageDescription")}
          </p>
          <LocaleDropdown
            value={locale}
            autoLabel={t("settings.languageAuto")}
            options={getAvailableLocales().map((loc) => ({ value: loc, label: loc.toUpperCase() }))}
            onChange={saveLocale}
          />
        </div>

        <div className="settings-section">
          <h3>{t("settings.authMode")}</h3>
          <p className="settings-description">
            {t("settings.authModeDescription")}
          </p>
          {authMode === "byond" && byondPagerRunning === false && !byondWebUsername && (
            <div className="byond-login-section">
              <div className="auth-mode-warning">
                {t("settings.byondPagerWarning")}
              </div>
              <div className="byond-web-login">
                {byondLoginState === "idle" && (
                  <>
                    <button
                      type="button"
                      className="button"
                      onClick={handleByondWebLogin}
                    >
                      {t("settings.loginToByond")}
                    </button>
                    <button
                      type="button"
                      className="button-secondary"
                      onClick={() => commands.openUrl("https://secure.byond.com/Join")}
                    >
                      {t("common.createAccount")}
                    </button>
                  </>
                )}
                {byondLoginState === "loading" && (
                  <p className="byond-login-status">
                    {t("settings.byondWaiting")}
                  </p>
                )}
                {byondLoginState === "success" && (
                  <p className="byond-login-status success">
                    {t("settings.byondSuccess")}
                    {byondWebUsername && ` (${byondWebUsername})`}
                  </p>
                )}
                {byondLoginState === "error" && (
                  <div className="byond-login-error">
                    <p>{t("settings.byondLoginFailed", { error: byondLoginError })}</p>
                    <button
                      type="button"
                      className="button-secondary"
                      onClick={handleByondWebLogin}
                    >
                      {t("common.tryAgain")}
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
                name={t("settings.hubAuth")}
                description={t("settings.hubAuthDesc")}
                onChange={onAuthModeChange}
              />
            )}
            {config?.oidc && (
              <AuthModeOption
                mode="oidc"
                currentMode={authMode}
                name={t("settings.oidcAuth", { provider: config.strings.auth_provider_name })}
                description={t("settings.oidcAuthDesc", { provider: config.strings.auth_provider_name })}
                onChange={onAuthModeChange}
              />
            )}
            {steamAvailable && !config?.urls.hub_api && (
              <AuthModeOption
                mode="steam"
                currentMode={authMode}
                name={t("settings.steamAuth")}
                description={t("settings.steamAuthDesc")}
                onChange={onAuthModeChange}
              />
            )}
            <AuthModeOption
              mode="byond"
              currentMode={authMode}
              name={t("settings.byondAuth")}
              description={t("settings.byondAuthDesc")}
              onChange={onAuthModeChange}
            />
          </div>
        </div>

        <WineSettings
          platform={platform}
          wineStatus={wineStatus}
          renderingPipeline={renderingPipeline}
          isResetting={isResettingWine}
          onResetPrefix={onResetWinePrefix}
          onRenderingPipelineChange={onRenderingPipelineChange}
        />

        {devMode && (
          <div className="settings-section dev-section">
            <h3>{t("settings.devOptions")}</h3>
            <p className="settings-description">
              {t("settings.devDescription")}
            </p>
            <DevConnectSection />
          </div>
        )}
      </div>
      <div className="settings-modal-footer">
        <span className="version-info">v{appVersion}</span>
      </div>
    </Modal>
  );
};
