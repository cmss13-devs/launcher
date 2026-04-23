import { useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "../bindings";
import { FontAwesomeIcon } from "@fortawesome/react-fontawesome";
import { faSteam, faDiscord } from "@fortawesome/free-brands-svg-icons";
import { faKey } from "@fortawesome/free-solid-svg-icons";
import type { IconDefinition } from "@fortawesome/fontawesome-svg-core";
import { Modal, ModalContent, ModalSpinner } from "./Modal";

export type AuthModalState = "idle" | "loading" | "error" | "2fa";

interface AuthModalProps {
  visible: boolean;
  state: AuthModalState;
  error?: string;
  loginPrompt: string;
  useHubAuth: boolean;
  oauthProviders: string[];
  steamAvailable: boolean;
  registerUrl?: string | null;
  onLogin: () => void;
  onHubLogin: (
    username: string,
    password: string,
    totpCode?: string,
  ) => void;
  onOAuthLogin: (provider: string) => void;
  onSteamLogin: () => void;
  onClose: () => void;
}

const OAUTH_DISPLAY_NAMES: Record<string, string> = {
  discord: "Discord",
  bab: "BYOND",
};

const OAUTH_ICONS: Record<string, IconDefinition> = {
  discord: faDiscord,
  bab: faKey,
};

export const AuthModal = ({
  visible,
  state,
  error,
  loginPrompt,
  useHubAuth,
  oauthProviders,
  steamAvailable,
  registerUrl,
  onLogin,
  onHubLogin,
  onOAuthLogin,
  onSteamLogin,
  onClose,
}: AuthModalProps) => {
  const { t } = useTranslation();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [totpCode, setTotpCode] = useState("");
  const [showHubLogin, setShowHubLogin] = useState(false);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (state === "2fa") {
      onHubLogin(username, password, totpCode);
    } else {
      onHubLogin(username, password);
    }
  };

  const titleMap: Record<AuthModalState, string> = {
    idle: useHubAuth ? t("auth.loginTitle") : t("auth.authRequired"),
    "2fa": t("auth.twoFactorTitle"),
    loading: t("auth.authenticating"),
    error: t("auth.authFailed"),
  };

  return (
    <Modal
      visible={visible}
      onClose={onClose}
      closeOnOverlayClick
      className={`auth-modal${steamAvailable && useHubAuth ? " auth-modal-steam" : ""}`}
      title={titleMap[state]}
    >
      {state === "idle" && !useHubAuth && (
        <ModalContent>
          <p>{loginPrompt}</p>
          <button type="button" className="button" onClick={onLogin}>
            {t("common.login")}
          </button>
        </ModalContent>
      )}
      {state === "idle" && useHubAuth && steamAvailable && (
        <div className="auth-modal-steam-body">
          {showHubLogin ? (
            <>
              <form onSubmit={handleSubmit} className="hub-login-form">
                <input
                  type="text"
                  placeholder={t("auth.usernamePlaceholder")}
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  autoFocus
                />
                <input
                  type="password"
                  placeholder={t("auth.passwordPlaceholder")}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                />
                <button type="submit" className="button" disabled={!username || !password}>
                  {t("common.login")}
                </button>
              </form>
              <div className="hub-login-links">
                {registerUrl && (
                  <button
                    type="button"
                    className="hub-login-toggle"
                    onClick={() => commands.openUrl(registerUrl)}
                  >
                    {t("common.createAccount")}
                  </button>
                )}
                {registerUrl && <span className="hub-login-separator">·</span>}
                <button
                  type="button"
                  className="hub-login-toggle"
                  onClick={() => setShowHubLogin(false)}
                >
                  {t("auth.backToSteam")}
                </button>
              </div>
            </>
          ) : (
            <>
              <div className="steam-login-section">
                <button
                  type="button"
                  className="button steam-login-button"
                  onClick={onSteamLogin}
                >
                  <FontAwesomeIcon icon={faSteam} />
                  {t("auth.signInWithSteam")}
                </button>
              </div>
              {oauthProviders.length > 0 && (
                <div className="oauth-providers">
                  <div className="oauth-divider"><span>{t("common.or")}</span></div>
                  {oauthProviders.map((provider) => (
                    <button
                      key={provider}
                      type="button"
                      className="button-secondary oauth-button"
                      onClick={() => onOAuthLogin(provider)}
                    >
                      <FontAwesomeIcon icon={OAUTH_ICONS[provider] ?? faKey} />
                      {" "}{OAUTH_DISPLAY_NAMES[provider] ?? provider}
                    </button>
                  ))}
                </div>
              )}
              <button
                type="button"
                className="hub-login-toggle"
                onClick={() => setShowHubLogin(true)}
              >
                {t("auth.hubLoginToggle")}
              </button>
            </>
          )}
        </div>
      )}
      {state === "idle" && useHubAuth && !steamAvailable && (
        <ModalContent>
          <form onSubmit={handleSubmit} className="hub-login-form">
            <input
              type="text"
              placeholder={t("auth.usernamePlaceholder")}
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              autoFocus
            />
            <input
              type="password"
              placeholder={t("auth.passwordPlaceholder")}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
            />
            <button type="submit" className="button" disabled={!username || !password}>
              {t("common.login")}
            </button>
            {registerUrl && (
              <button
                type="button"
                className="register-link"
                onClick={() => commands.openUrl(registerUrl)}
              >
                {t("auth.noAccount")} <span>{t("auth.createOne")}</span>
              </button>
            )}
          </form>
          {oauthProviders.length > 0 && (
            <div className="oauth-providers">
              <div className="oauth-divider"><span>{t("common.or")}</span></div>
              {oauthProviders.map((provider) => (
                <button
                  key={provider}
                  type="button"
                  className="button-secondary oauth-button"
                  onClick={() => onOAuthLogin(provider)}
                >
                  <FontAwesomeIcon icon={OAUTH_ICONS[provider] ?? faKey} />
                  {" "}{OAUTH_DISPLAY_NAMES[provider] ?? provider}
                </button>
              ))}
            </div>
          )}
        </ModalContent>
      )}
      {state === "2fa" && (
        <ModalContent>
          <form onSubmit={handleSubmit} className="hub-login-form">
            <p>{t("auth.twoFactorPrompt")}</p>
            <input
              type="text"
              placeholder={t("auth.codePlaceholder")}
              value={totpCode}
              onChange={(e) => setTotpCode(e.target.value)}
              autoFocus
              maxLength={6}
              inputMode="numeric"
              autoComplete="one-time-code"
            />
            <button type="submit" className="button" disabled={totpCode.length < 6}>
              {t("auth.verify")}
            </button>
          </form>
        </ModalContent>
      )}
      {state === "loading" && (
        <ModalContent>
          {useHubAuth ? <p>{t("auth.loggingIn")}</p> : <p>{t("auth.completeBrowserLogin")}</p>}
          <ModalSpinner />
        </ModalContent>
      )}
      {state === "error" && (
        <ModalContent>
          <p className="auth-error-message">{error}</p>
          {useHubAuth ? (
            <button
              type="button"
              className="button"
              onClick={() => {
                setPassword("");
                setTotpCode("");
              }}
            >
              {t("common.tryAgain")}
            </button>
          ) : (
            <button type="button" className="button" onClick={onLogin}>
              {t("common.tryAgain")}
            </button>
          )}
        </ModalContent>
      )}
    </Modal>
  );
};
