import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import { commands } from "../bindings";
import { unwrap } from "../lib/unwrap";

interface ByondStore {
  username: string | null;
  pagerRunning: boolean | null;
  loginVisible: boolean;
  loggingOut: boolean;
  setUsername: (username: string | null) => void;
  setPagerRunning: (running: boolean | null) => void;
  setLoggingOut: (loggingOut: boolean) => void;
  checkStatus: () => Promise<void>;
  checkSession: () => Promise<void>;
  initListener: () => Promise<() => void>;
}

export const useByondStore = create<ByondStore>()((set) => ({
  username: null,
  pagerRunning: null,
  loginVisible: false,
  loggingOut: false,

  setUsername: (username) => set({ username }),
  setPagerRunning: (pagerRunning) => set({ pagerRunning }),
  setLoggingOut: (loggingOut) => set({ loggingOut }),

  checkStatus: async () => {
    try {
      const pagerRunning = unwrap(await commands.isByondPagerRunning());
      set({ pagerRunning });
    } catch {
      // Ignore errors
    }
  },

  checkSession: async () => {
    try {
      const sessionCheck = unwrap(await commands.checkByondWebSession());
      if (sessionCheck.logged_in && sessionCheck.username) {
        set({ username: sessionCheck.username });
      }
    } catch {
      // Ignore errors - user may not be logged in
    }
  },

  initListener: async () => {
    try {
      const pagerRunning = unwrap(await commands.isByondPagerRunning());
      set({ pagerRunning });
    } catch {
      // Ignore errors
    }

    const unlistenSession = await listen<string | null>(
      "byond-session-changed",
      (event) => {
        set({ username: event.payload });
      },
    );

    const unlistenLogin = await listen<boolean>(
      "byond-login-visible",
      (event) => {
        set({ loginVisible: event.payload });
      },
    );

    return () => {
      unlistenSession();
      unlistenLogin();
    };
  },
}));
