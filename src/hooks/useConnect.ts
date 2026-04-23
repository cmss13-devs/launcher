import { useCallback } from "react";
import { commands } from "../bindings";
import { formatCommandError } from "../lib/formatCommandError";
import { unwrap } from "../lib/unwrap";
import { useAuthFlow } from "./useAuthFlow";
import { useError } from "./useError";

export interface AuthError {
  code: string;
  message: string;
  linking_url: string | null;
}

export interface ConnectionResult {
  success: boolean;
  message: string;
  auth_error: AuthError | null;
}

function dispatchAuthError(
  error: AuthError,
  handlers: {
    onLoginRequired: () => void;
    onSteamAuthRequired: (serverName: string) => void;
    onByondAuthRequired: () => void;
    onError: (msg: string) => void;
  },
  serverName?: string,
) {
  switch (error.code) {
    case "auth_required":
      handlers.onLoginRequired();
      break;
    case "byond_auth_required":
      handlers.onByondAuthRequired();
      break;
    case "steam_linking_required":
      handlers.onSteamAuthRequired(serverName ?? "");
      break;
    default:
      handlers.onError(error.message);
  }
}

export function useRawConnect() {
  const connect = useCallback(
    async (serverName: string, source: string): Promise<ConnectionResult> => {
      return unwrap(await commands.connectToServer(serverName, source));
    },
    [],
  );
  return { connect };
}

export const useConnect = () => {
  const { onLoginRequired, onSteamAuthRequired, handleByondLogin } = useAuthFlow();
  const { showError } = useError();

  const handlers = {
    onLoginRequired,
    onSteamAuthRequired,
    onByondAuthRequired: handleByondLogin,
    onError: showError,
  };

  const connect = useCallback(
    async (serverName: string, source: string): Promise<boolean> => {
      const result = unwrap(await commands.connectToServer(serverName, source));
      if (result.success) return true;
      if (result.auth_error) {
        dispatchAuthError(result.auth_error, handlers, serverName);
      } else {
        showError(result.message);
      }
      return false;
    },
    [handlers, showError],
  );

  const connectToAddress = useCallback(
    async (address: string, source: string): Promise<boolean> => {
      const result = await commands.connectToAddress(address, source);
      if (result.status === "error") {
        showError(formatCommandError(result.error));
        return false;
      }
      const data = result.data;
      if (data.success) return true;
      if (data.auth_error) {
        dispatchAuthError(data.auth_error, handlers);
      } else {
        showError(data.message);
      }
      return false;
    },
    [handlers, showError],
  );

  return { connect, connectToAddress };
};
