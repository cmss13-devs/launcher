import { useCallback } from "react";
import { commands } from "../bindings";
import { unwrap } from "../lib/unwrap";

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

export const useConnect = () => {
  const connect = useCallback(
    async (serverName: string, source: string): Promise<ConnectionResult> => {
      console.log(`[useConnect] connect called, source=${source}`);

      return unwrap(await commands.connectToServer(serverName, source));
    },
    [],
  );

  return { connect };
};
