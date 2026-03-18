import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";
import type { AppSettings, AuthMode, Theme } from "../types";

interface SettingsStore {
  authMode: AuthMode;
  theme: Theme;
  devMode: boolean;
  notificationServers: Set<string>;
  fullscreenOverlay: boolean;

  setAuthMode: (mode: AuthMode) => void;
  setTheme: (theme: Theme) => void;
  load: () => Promise<AppSettings | null>;
  saveAuthMode: (mode: AuthMode) => Promise<void>;
  saveTheme: (theme: Theme) => Promise<void>;
  toggleServerNotifications: (serverName: string, enabled: boolean) => Promise<void>;
  isServerNotificationsEnabled: (serverName: string) => boolean;
  saveFullscreenOverlay: (enabled: boolean) => Promise<void>;
}

export const useSettingsStore = create<SettingsStore>()((set, get) => ({
  authMode: "cm_ss13",
  theme: "tgui",
  devMode: false,
  notificationServers: new Set<string>(),
  fullscreenOverlay: true,

  setAuthMode: (authMode) => set({ authMode }),
  setTheme: (theme) => set({ theme }),

  load: async () => {
    try {
      const [settings, devMode] = await Promise.all([
        invoke<AppSettings>("get_settings"),
        invoke<boolean>("is_dev_mode"),
      ]);
      set({
        authMode: settings.auth_mode,
        theme: settings.theme,
        devMode,
        notificationServers: new Set(settings.notification_servers),
        fullscreenOverlay: settings.fullscreen_overlay,
      });
      return settings;
    } catch (err) {
      console.error("Failed to load settings:", err);
      return null;
    }
  },

  saveAuthMode: async (mode: AuthMode) => {
    await invoke<AppSettings>("set_auth_mode", { mode });
    set({ authMode: mode });
  },

  saveTheme: async (theme: Theme) => {
    await invoke<AppSettings>("set_theme", { theme });
    set({ theme });
  },

  toggleServerNotifications: async (serverName: string, enabled: boolean) => {
    const settings = await invoke<AppSettings>("toggle_server_notifications", {
      serverName,
      enabled,
    });
    set({ notificationServers: new Set(settings.notification_servers) });
  },

  isServerNotificationsEnabled: (serverName: string) => {
    return get().notificationServers.has(serverName);
  },

  saveFullscreenOverlay: async (enabled: boolean) => {
    const settings = await invoke<AppSettings>("set_fullscreen_overlay", { enabled });
    set({ fullscreenOverlay: settings.fullscreen_overlay });
  },
}));
