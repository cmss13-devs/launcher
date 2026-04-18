import { create } from "zustand";
import { type AppSettings, type AuthMode, commands, type Theme } from "../bindings";
import { setLocale } from "../i18n";
import { unwrap } from "../lib/unwrap";

interface SettingsStore {
  authMode: AuthMode;
  theme: Theme;
  devMode: boolean;
  notificationServers: Set<string>;
  ageVerified: boolean;
  locale: string | null;

  setAuthMode: (mode: AuthMode) => void;
  setTheme: (theme: Theme) => void;
  load: () => Promise<AppSettings | null>;
  saveAuthMode: (mode: AuthMode) => Promise<void>;
  saveTheme: (theme: Theme) => Promise<void>;
  saveAgeVerified: () => Promise<void>;
  saveLocale: (locale: string | null) => Promise<void>;
  toggleServerNotifications: (serverName: string, enabled: boolean) => Promise<void>;
  isServerNotificationsEnabled: (serverName: string) => boolean;
}

export const useSettingsStore = create<SettingsStore>()((set, get) => ({
  authMode: "oidc",
  theme: "tgui",
  devMode: false,
  notificationServers: new Set<string>(),
  ageVerified: false,
  locale: null,

  setAuthMode: (authMode) => set({ authMode }),
  setTheme: (theme) => set({ theme }),

  load: async () => {
    try {
      const [settings, devMode] = await Promise.all([
        commands.getSettings().then(unwrap),
        commands.isDevMode(),
      ]);
      set({
        authMode: settings.auth_mode,
        theme: settings.theme ?? "tgui",
        devMode,
        notificationServers: new Set(settings.notification_servers ?? []),
        ageVerified: settings.age_verified ?? false,
        locale: settings.locale ?? null,
      });
      if (settings.locale) {
        setLocale(settings.locale);
      }
      return settings;
    } catch (err) {
      console.error("Failed to load settings:", err);
      return null;
    }
  },

  saveAuthMode: async (mode: AuthMode) => {
    unwrap(await commands.setAuthMode(mode));
    set({ authMode: mode });
  },

  saveTheme: async (theme: Theme) => {
    unwrap(await commands.setTheme(theme));
    set({ theme });
  },

  saveAgeVerified: async () => {
    unwrap(await commands.setAgeVerified());
    set({ ageVerified: true });
  },

  saveLocale: async (locale: string | null) => {
    unwrap(await commands.setLocale(locale));
    setLocale(locale);
    set({ locale });
  },

  toggleServerNotifications: async (serverName: string, enabled: boolean) => {
    const settings = unwrap(await commands.toggleServerNotifications(serverName, enabled));
    set({ notificationServers: new Set(settings.notification_servers ?? []) });
  },

  isServerNotificationsEnabled: (serverName: string) => {
    return get().notificationServers.has(serverName);
  },
}));
