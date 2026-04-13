import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import { type AuthState, commands } from "../bindings";
import { formatCommandError } from "../lib/formatCommandError";
import { unwrap } from "../lib/unwrap";

interface AuthStore {
  authState: AuthState;
  setAuthState: (state: AuthState) => void;
  login: () => Promise<{ success: boolean; error?: string }>;
  hubLogin: (
    username: string,
    password: string,
    totpCode?: string,
  ) => Promise<{ success: boolean; error?: string; requires2fa?: boolean }>;
  hubOAuthLogin: (
    provider: string,
  ) => Promise<{ success: boolean; error?: string }>;
  hubSteamLogin: () => Promise<{ success: boolean; error?: string }>;
  logout: () => Promise<void>;
  initListener: () => Promise<() => void>;
}

const initialAuthState: AuthState = {
  logged_in: false,
  user: null,
  loading: true,
  error: null,
};

export const useAuthStore = create<AuthStore>()((set, get) => ({
  authState: initialAuthState,

  setAuthState: (authState) => set({ authState }),

  login: async () => {
    try {
      const state = unwrap(await commands.startLogin());
      set({ authState: state });
      return { success: state.logged_in };
    } catch (err) {
      const error = err instanceof Error ? err.message : String(err);
      return { success: false, error };
    }
  },

  hubLogin: async (username, password, totpCode?) => {
    const r = await commands.hubLogin(username, password, totpCode || null);
    if (r.status === "ok") {
      set({ authState: r.data });
      return { success: r.data.logged_in };
    }
    if (r.error.type === "requires_2fa") {
      return { success: false, requires2fa: true };
    }
    return { success: false, error: formatCommandError(r.error) };
  },

  hubOAuthLogin: async (provider) => {
    try {
      const state = unwrap(await commands.hubOauthLogin(provider));
      set({ authState: state });
      return { success: state.logged_in };
    } catch (err) {
      const error = err instanceof Error ? err.message : String(err);
      return { success: false, error };
    }
  },

  hubSteamLogin: async () => {
    try {
      const state = unwrap(await commands.hubSteamLogin());
      set({ authState: state });
      return { success: state.logged_in };
    } catch (err) {
      const error = err instanceof Error ? err.message : String(err);
      return { success: false, error };
    }
  },

  logout: async () => {
    try {
      const state = unwrap(await commands.logout());
      set({ authState: state });
    } catch (err) {
      console.error("Logout failed:", err);
    }
  },

  initListener: async () => {
    try {
      const state = unwrap(await commands.getAuthState());
      get().setAuthState(state);
    } catch (err) {
      get().setAuthState({
        logged_in: false,
        user: null,
        loading: false,
        error: String(err),
      });
    }

    const unlisten = await listen<AuthState>("auth-state-changed", (event) => {
      get().setAuthState(event.payload);
    });

    return unlisten;
  },
}));
