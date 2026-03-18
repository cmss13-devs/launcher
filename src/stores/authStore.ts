import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import type { AuthState } from "../types";

interface AuthStore {
  authState: AuthState;
  setAuthState: (state: AuthState) => void;
  login: () => Promise<{ success: boolean; error?: string }>;
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
      const state = await invoke<AuthState>("start_login");
      set({ authState: state });
      return { success: state.logged_in };
    } catch (err) {
      const error = err instanceof Error ? err.message : String(err);
      return { success: false, error };
    }
  },

  logout: async () => {
    try {
      const state = await invoke<AuthState>("logout");
      set({ authState: state });
    } catch (err) {
      console.error("Logout failed:", err);
    }
  },

  initListener: async () => {
    try {
      const state = await invoke<AuthState>("get_auth_state");
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
