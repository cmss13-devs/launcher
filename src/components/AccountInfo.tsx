import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "../bindings";
import { useAuthFlow } from "../hooks";
import { unwrap } from "../lib/unwrap";
import { useAuthStore, useByondStore, useSettingsStore, useSteamStore } from "../stores";

interface AccountAction {
  label: string;
  onClick: () => void;
  primary?: boolean;
}

interface AccountDisplayProps {
  avatar: string;
  name: string;
  status: string;
  actions?: AccountAction[];
}

const AccountDisplay = ({ avatar, name, status, actions }: AccountDisplayProps) => {
  return (
    <>
      <div className="account-avatar">{avatar}</div>
      <div className="account-details">
        <div className="account-name">{name}</div>
        <div className="account-status">{status}</div>
      </div>
      {actions?.map((action) => (
        <button
          key={action.label}
          type="button"
          className={action.primary ? "button" : "button-secondary"}
          onClick={action.onClick}
        >
          {action.label}
        </button>
      ))}
    </>
  );
};

export const AccountInfo = () => {
  const { t } = useTranslation();
  const {
    onLoginRequired: onLogin,
    handleLogout: onLogout,
    handleSteamLogout: onSteamLogout,
    handleByondLogin: onByondLogin,
    handleByondLogout: onByondLogout,
  } = useAuthFlow();
  const authMode = useSettingsStore((s) => s.authMode);
  const authState = useAuthStore((s) => s.authState);
  const steamUser = useSteamStore((s) => s.user);
  const steamAccessToken = useSteamStore((s) => s.accessToken);
  const byondWebUsername = useByondStore((s) => s.username);
  const byondPagerRunning = useByondStore((s) => s.pagerRunning);
  const byondLoggingOut = useByondStore((s) => s.loggingOut);
  const checkByondStatus = useByondStore((s) => s.checkStatus);

  const [byondPagerUsername, setByondPagerUsername] = useState<string | null>(null);

  useEffect(() => {
    if (authMode === "byond") {
      const checkPagerUsername = async () => {
        checkByondStatus();
        if (byondPagerRunning) {
          try {
            const username = unwrap(await commands.getByondUsername());
            setByondPagerUsername(username);
          } catch {
            setByondPagerUsername(null);
          }
        } else {
          setByondPagerUsername(null);
        }
      };

      checkPagerUsername();
      const interval = setInterval(checkPagerUsername, 5000);
      return () => clearInterval(interval);
    }
  }, [authMode, byondPagerRunning, checkByondStatus]);

  if (authMode === "byond") {
    // Web-based BYOND login takes priority
    if (byondWebUsername) {
      return (
        <AccountDisplay
          avatar={byondWebUsername.charAt(0).toUpperCase()}
          name={byondWebUsername}
          status={t("account.loggedInViaByondWeb")}
          actions={[{ label: byondLoggingOut ? t("common.loggingOut") : t("common.logout"), onClick: onByondLogout }]}
        />
      );
    }
    // Logged in via pager - no need for web login
    if (byondPagerUsername) {
      return (
        <AccountDisplay
          avatar={byondPagerUsername.charAt(0).toUpperCase()}
          name={byondPagerUsername}
          status={t("account.loggedInViaByondPager")}
        />
      );
    }
    // Not logged in via web or pager - show login button
    const status = byondPagerRunning === true
      ? t("account.pagerOpenNotLoggedIn")
      : t("account.notLoggedIn");
    return (
      <AccountDisplay
        avatar="B"
        name="BYOND"
        status={status}
        actions={[
          { label: t("common.login"), onClick: onByondLogin, primary: true },
          { label: t("common.createAccount"), onClick: () => commands.openUrl("https://secure.byond.com/Join") },
        ]}
      />
    );
  }

  if (authMode === "steam") {
    if (steamAccessToken) {
      return (
        <AccountDisplay
          avatar="S"
          name={steamUser?.display_name || t("account.steamUser")}
          status={t("account.loggedInViaSteam")}
          actions={[{ label: t("common.logout"), onClick: onSteamLogout }]}
        />
      );
    }
    return (
      <AccountDisplay
        avatar="S"
        name={steamUser?.display_name || "Steam"}
        status={t("account.clickToAuth")}
      />
    );
  }

  if (authState.logged_in && authState.user) {
    const displayName =
      authState.user.name || authState.user.preferred_username || "User";
    return (
      <AccountDisplay
        avatar={displayName.charAt(0).toUpperCase()}
        name={displayName}
        status={authState.user.email || t("account.loggedIn")}
        actions={[{ label: t("common.logout"), onClick: onLogout }]}
      />
    );
  }

  return (
    <AccountDisplay
      avatar="?"
      name={t("account.notLoggedIn")}
      status={authState.loading ? t("account.checking") : t("account.clickToAuth")}
      actions={[{ label: t("common.login"), onClick: onLogin, primary: true }]}
    />
  );
};
