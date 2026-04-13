import { create } from "zustand";
import { commands, type LauncherConfig } from "../bindings";

interface ConfigStore {
  config: LauncherConfig | null;
  loading: boolean;
  load: () => Promise<LauncherConfig>;
}

export const useConfigStore = create<ConfigStore>()((set, get) => ({
  config: null,
  loading: false,

  load: async () => {
    const existing = get().config;
    if (existing) return existing;

    set({ loading: true });
    try {
      const config = await commands.getLauncherConfig();
      set({ config, loading: false });
      return config;
    } catch (err) {
      console.error("Failed to load launcher config:", err);
      set({ loading: false });
      throw err;
    }
  },
}));
