import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

interface AutoConnectEvent {
  status:
    | "starting"
    | "waiting_for_servers"
    | "server_not_found"
    | "server_unavailable"
    | "auth_required"
    | "steam_linking_required"
    | "connecting"
    | "connected"
    | "error";
  server_name: string;
  message: string | null;
  linking_url: string | null;
}

interface Deps {
  onLoginRequired: () => void;
  onAutoConnectLinkingRequired: (linkingUrl: string | null) => void;
  showError: (message: string) => void;
}

export function useAutoConnect({
  onLoginRequired,
  onAutoConnectLinkingRequired,
  showError,
}: Deps) {
  const [autoConnecting, setAutoConnecting] = useState(false);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    const setupListener = async () => {
      unlisten = await listen<AutoConnectEvent>("autoconnect-status", (event) => {
        const { status, server_name, message, linking_url } = event.payload;
        console.log(`[autoconnect] status=${status} server=${server_name}`);

        switch (status) {
          case "starting":
          case "waiting_for_servers":
          case "connecting":
            setAutoConnecting(true);
            break;

          case "auth_required":
            setAutoConnecting(false);
            onLoginRequired();
            break;

          case "steam_linking_required":
            setAutoConnecting(false);
            onAutoConnectLinkingRequired(linking_url);
            break;

          case "server_not_found":
          case "server_unavailable":
          case "error":
            setAutoConnecting(false);
            if (message) showError(message);
            break;

          case "connected":
            setAutoConnecting(false);
            break;
        }
      });
    };

    setupListener();

    return () => {
      if (unlisten) unlisten();
    };
  }, [showError, onLoginRequired, onAutoConnectLinkingRequired]);

  return autoConnecting;
}
