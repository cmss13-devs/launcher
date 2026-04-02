import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";

interface ByondSessionCheck {
  logged_in: boolean;
  username: string | null;
  web_id: string | null;
}

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
      const pagerRunning = await invoke<boolean>("is_byond_pager_running");
      set({ pagerRunning });
    } catch {
      // Ignore errors
    }
  },

  checkSession: async () => {
    try {
      const sessionCheck = await invoke<ByondSessionCheck>("check_byond_web_session");
      if (sessionCheck.logged_in && sessionCheck.username) {
        set({ username: sessionCheck.username });
      }
    } catch {
      // Ignore errors - user may not be logged in
    }
  },

  initListener: async () => {
    try {
      const pagerRunning = await invoke<boolean>("is_byond_pager_running");
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
