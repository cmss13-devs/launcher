import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";
import { commands } from "../bindings";
import { unwrap } from "../lib/unwrap";

interface ByondStore {
  username: string | null;
  pagerRunning: boolean | null;
  setUsername: (username: string | null) => void;
  setPagerRunning: (running: boolean | null) => void;
  checkStatus: () => Promise<void>;
  checkSession: () => Promise<void>;
  initListener: () => Promise<() => void>;
}

export const useByondStore = create<ByondStore>()((set) => ({
  username: null,
  pagerRunning: null,

  setUsername: (username) => set({ username }),
  setPagerRunning: (pagerRunning) => set({ pagerRunning }),

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

    const unlisten = await listen<string | null>(
      "byond-session-changed",
      (event) => {
        set({ username: event.payload });
      },
    );

    return unlisten;
  },
}));
