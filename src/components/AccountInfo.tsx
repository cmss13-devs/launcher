import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { useAuthStore, useSettingsStore, useSteamStore } from "../stores";
import type { ByondSessionCheck } from "../types";

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

  const [byondPagerRunning, setByondPagerRunning] = useState<boolean | null>(null);
  const [byondUsername, setByondUsername] = useState<string | null>(null);
  const [byondWebUsername, setByondWebUsername] = useState<string | null>(null);
  const [sessionCheckDone, setSessionCheckDone] = useState(false);

  // Listen for session changes from backend
  useEffect(() => {
    const unlisten = listen<string | null>("byond-session-changed", (event) => {
      setByondWebUsername(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (authMode === "byond") {
      // On initial load or mode change, check if already logged in via cookies
      const checkExistingSession = async () => {
        if (sessionCheckDone) return;

        try {
          // First check in-memory session
          const memoryUsername = await invoke<string | null>("get_byond_session_status");
          if (memoryUsername) {
            setByondWebUsername(memoryUsername);
            setSessionCheckDone(true);
            return;
          }

          // Check for existing cookie-based session
          const sessionCheck = await invoke<ByondSessionCheck>("check_byond_web_session");
          if (sessionCheck.logged_in && sessionCheck.username) {
            setByondWebUsername(sessionCheck.username);
          }
          setSessionCheckDone(true);
        } catch (err) {
          console.error("Failed to check BYOND session:", err);
          setSessionCheckDone(true);
        }
      };

      checkExistingSession();

      const checkByondStatus = async () => {
        try {
          // Check for web-based BYOND session first
          const webUsername = await invoke<string | null>("get_byond_session_status");
          setByondWebUsername(webUsername);

          // Also check pager status
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
    } else {
      // Reset session check when switching away from BYOND mode
      setSessionCheckDone(false);
    }
  }, [authMode, sessionCheckDone]);

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
    // Fall back to pager-based login
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
        status="Not logged in"
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
