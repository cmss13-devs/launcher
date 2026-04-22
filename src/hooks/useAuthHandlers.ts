import { useCallback, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { commands } from "../bindings";
import type { AuthModalState } from "../components/AuthModal";
import { unwrap } from "../lib/unwrap";
import { useAuthStore, useByondStore } from "../stores";
import { useError } from "./useError";

export interface AuthModalView {
  visible: boolean;
  state: AuthModalState;
  error?: string;
}

const CLOSED: AuthModalView = { visible: false, state: "idle", error: undefined };

export function useAuthHandlers() {
  const { showError } = useError();
  const { login, hubLogin, hubOAuthLogin, hubSteamLogin, logout } = useAuthStore(
    useShallow((s) => ({
      login: s.login,
      hubLogin: s.hubLogin,
      hubOAuthLogin: s.hubOAuthLogin,
      hubSteamLogin: s.hubSteamLogin,
      logout: s.logout,
    })),
  );

  const [authModal, setAuthModal] = useState<AuthModalView>(CLOSED);

  const handleLogin = useCallback(async () => {
    setAuthModal({ visible: true, state: "loading", error: undefined });
    const result = await login();
    setAuthModal(
      result.success
        ? CLOSED
        : { visible: true, state: "error", error: result.error },
    );
  }, [login]);

  const handleLogout = useCallback(async () => {
    try {
      await logout();
    } catch (err) {
      showError(err instanceof Error ? err.message : String(err));
    }
  }, [logout, showError]);

  const handleByondLogin = useCallback(async () => {
    const result = await commands.startByondLogin();
    if (result.status === "error") {
      if (result.error.type === "cancelled") return;
      try { unwrap(result); } catch (err) {
        showError(err instanceof Error ? err.message : String(err));
      }
    }
  }, [showError]);

  const setLoggingOut = useByondStore((s) => s.setLoggingOut);
  const handleByondLogout = useCallback(async () => {
    setLoggingOut(true);
    try {
      unwrap(await commands.logoutByondWeb());
    } catch (err) {
      showError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoggingOut(false);
    }
  }, [showError, setLoggingOut]);

  const handleHubLogin = useCallback(
    async (username: string, password: string, totpCode?: string) => {
      setAuthModal({ visible: true, state: "loading", error: undefined });
      const result = await hubLogin(username, password, totpCode);
      if (result.success) {
        setAuthModal(CLOSED);
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
      setAuthModal(
        result.success
          ? CLOSED
          : { visible: true, state: "error", error: result.error },
      );
    },
    [hubOAuthLogin],
  );

  const handleSteamLogin = useCallback(async () => {
    setAuthModal({ visible: true, state: "loading", error: undefined });
    const result = await hubSteamLogin();
    setAuthModal(
      result.success
        ? CLOSED
        : { visible: true, state: "error", error: result.error },
    );
  }, [hubSteamLogin]);

  const handleAuthModalClose = useCallback(() => setAuthModal(CLOSED), []);

  const onLoginRequired = useCallback(() => {
    setAuthModal({ visible: true, state: "idle", error: undefined });
  }, []);

  return {
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
  };
}
