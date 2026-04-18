import { createContext, useContext, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { AuthModal } from "../components/AuthModal";
import { SteamAuthModal } from "../components/SteamAuthModal";
import { useAuthStore, useConfigStore, useSteamStore } from "../stores";
import { useAuthHandlers } from "./useAuthHandlers";
import { useSteamLinking } from "./useSteamLinking";

interface AuthFlowContextValue {
  onLoginRequired: () => void;
  onSteamAuthRequired: (serverName?: string) => void;
  onAutoConnectLinkingRequired: (linkingUrl: string | null) => void;
  handleLogout: () => void | Promise<void>;
  handleSteamLogout: () => void;
  handleByondLogin: () => void | Promise<void>;
  handleByondLogout: () => void | Promise<void>;
}

const AuthFlowContext = createContext<AuthFlowContextValue | null>(null);

export function useAuthFlow(): AuthFlowContextValue {
  const ctx = useContext(AuthFlowContext);
  if (!ctx) {
    throw new Error("useAuthFlow must be used within an AuthFlowProvider");
  }
  return ctx;
}

export const AuthFlowProvider = ({ children }: { children: ReactNode }) => {
  const { t } = useTranslation();
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

  return (
    <AuthFlowContext.Provider
      value={{
        onLoginRequired,
        onSteamAuthRequired,
        onAutoConnectLinkingRequired,
        handleLogout,
        handleSteamLogout,
        handleByondLogin,
        handleByondLogout,
      }}
    >
      <AuthModal
        {...authModal}
        loginPrompt={config?.strings.login_prompt ?? t("auth.loginPromptDefault")}
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
      {children}
    </AuthFlowContext.Provider>
  );
};
