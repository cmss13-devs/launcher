import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { useAuthStore, useByondStore, useSettingsStore, useSteamStore } from "../stores";

interface AccountDisplayProps {
  avatar: string;
  name: string;
  status: string;
  action?: {
    label: string;
    onClick: () => void;
    primary?: boolean;
  };
}

const AccountDisplay = ({ avatar, name, status, action }: AccountDisplayProps) => {
  return (
    <>
      <div className="account-avatar">{avatar}</div>
      <div className="account-details">
        <div className="account-name">{name}</div>
        <div className="account-status">{status}</div>
      </div>
      {action && (
        <button
          type="button"
          className={action.primary ? "button" : "button-secondary"}
          onClick={action.onClick}
        >
          {action.label}
        </button>
      )}
    </>
  );
};

interface AccountInfoProps {
  onLogin: () => void;
  onLogout: () => void;
  onSteamLogout: () => void;
  onByondLogin: () => void;
  onByondLogout: () => void;
}

export const AccountInfo = ({
  onLogin,
  onLogout,
  onSteamLogout,
  onByondLogin,
  onByondLogout,
}: AccountInfoProps) => {
  const authMode = useSettingsStore((s) => s.authMode);
  const authState = useAuthStore((s) => s.authState);
  const steamUser = useSteamStore((s) => s.user);
  const steamAccessToken = useSteamStore((s) => s.accessToken);
  const byondWebUsername = useByondStore((s) => s.username);
  const byondPagerRunning = useByondStore((s) => s.pagerRunning);
  const checkByondStatus = useByondStore((s) => s.checkStatus);

  const [byondPagerUsername, setByondPagerUsername] = useState<string | null>(null);

  useEffect(() => {
    if (authMode === "byond") {
      const checkPagerUsername = async () => {
        checkByondStatus();
        if (byondPagerRunning) {
          try {
            const username = await invoke<string | null>("get_byond_username");
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
          status="Logged in via BYOND Web"
          action={{ label: "Logout", onClick: onByondLogout }}
        />
      );
    }
    // Logged in via pager - no need for web login
    if (byondPagerUsername) {
      return (
        <AccountDisplay
          avatar={byondPagerUsername.charAt(0).toUpperCase()}
          name={byondPagerUsername}
          status="Logged in via BYOND Pager"
        />
      );
    }
    // Not logged in via web or pager - show login button
    const status = byondPagerRunning === true
      ? "Pager open (not logged in)"
      : "Not logged in";
    return (
      <AccountDisplay
        avatar="B"
        name="BYOND"
        status={status}
        action={{ label: "Login", onClick: onByondLogin, primary: true }}
      />
    );
  }

  if (authMode === "steam") {
    if (steamAccessToken) {
      return (
        <AccountDisplay
          avatar="S"
          name={steamUser?.display_name || "Steam User"}
          status="Logged in via Steam"
          action={{ label: "Logout", onClick: onSteamLogout }}
        />
      );
    }
    return (
      <AccountDisplay
        avatar="S"
        name={steamUser?.display_name || "Steam"}
        status="Click connect to authenticate"
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
        status={authState.user.email || "Logged in"}
        action={{ label: "Logout", onClick: onLogout }}
      />
    );
  }

  return (
    <AccountDisplay
      avatar="?"
      name="Not logged in"
      status={authState.loading ? "Checking..." : "Click to authenticate"}
      action={{ label: "Login", onClick: onLogin, primary: true }}
    />
  );
};
