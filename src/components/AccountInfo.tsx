import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { useAuthStore, useSettingsStore, useSteamStore } from "../stores";

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
}

export const AccountInfo = ({
  onLogin,
  onLogout,
  onSteamLogout,
}: AccountInfoProps) => {
  const authMode = useSettingsStore((s) => s.authMode);
  const authState = useAuthStore((s) => s.authState);
  const steamUser = useSteamStore((s) => s.user);
  const steamAccessToken = useSteamStore((s) => s.accessToken);

  const [byondPagerRunning, setByondPagerRunning] = useState<boolean | null>(null);
  const [byondUsername, setByondUsername] = useState<string | null>(null);

  useEffect(() => {
    if (authMode === "byond") {
      const checkByondStatus = async () => {
        try {
          const running = await invoke<boolean>("is_byond_pager_running");
          setByondPagerRunning(running);

          if (running) {
            const username = await invoke<string | null>("get_byond_username");
            setByondUsername(username);
          } else {
            setByondUsername(null);
          }
        } catch {
          setByondPagerRunning(null);
          setByondUsername(null);
        }
      };

      checkByondStatus();
      // Poll every 5 seconds
      const interval = setInterval(checkByondStatus, 5000);
      return () => clearInterval(interval);
    }
  }, [authMode]);

  if (authMode === "byond") {
    if (byondPagerRunning === true && byondUsername) {
      return (
        <AccountDisplay
          avatar={byondUsername.charAt(0).toUpperCase()}
          name={byondUsername}
          status="Logged in via BYOND"
        />
      );
    }
    if (byondPagerRunning === true) {
      return (
        <AccountDisplay
          avatar="B"
          name="BYOND"
          status="Open (not logged in)"
        />
      );
    }
    return (
      <AccountDisplay
        avatar="B"
        name="BYOND"
        status="Not running"
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
