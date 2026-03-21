import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { create } from "zustand";

interface ByondStore {
  username: string | null;
  pagerRunning: boolean | null;
  setUsername: (username: string | null) => void;
  setPagerRunning: (running: boolean | null) => void;
  checkStatus: () => Promise<void>;
  initListener: () => Promise<() => void>;
}

export const useByondStore = create<ByondStore>()((set, get) => ({
  username: null,
  pagerRunning: null,

  setUsername: (username) => set({ username }),
  setPagerRunning: (pagerRunning) => set({ pagerRunning }),

  checkStatus: async () => {
    try {
      const [username, pagerRunning] = await Promise.all([
        invoke<string | null>("get_byond_session_status"),
        invoke<boolean>("is_byond_pager_running"),
      ]);
      set({ username, pagerRunning });
    } catch {
      // Ignore errors
    }
  },

  initListener: async () => {
    await get().checkStatus();

    const unlisten = await listen<string | null>(
      "byond-session-changed",
      (event) => {
        set({ username: event.payload });
      },
    );

    return unlisten;
  },
}));
