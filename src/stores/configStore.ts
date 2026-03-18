import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

export interface LauncherFeatures {
  social_links: boolean;
  relay_selector: boolean;
  hub_server_list: boolean;
  cm_auth: boolean;
  singleplayer: boolean;
  server_search: boolean;
  server_filters: boolean;
  show_offline_servers: boolean;
  server_stats: boolean;
  auto_launch_byond: boolean;
  connection_timeout_fallback: boolean;
}

export interface SingleplayerConfig {
  github_repo: string | null;
  build_asset_name: string | null;
  dmb_name: string | null;
}

export interface LauncherUrls {
  server_api: string;
  auth_base: string | null;
  steam_auth: string | null;
  byond_hash_api: string | null;
  help_url: string;
}

export interface LauncherStrings {
  auth_provider_name: string;
  login_prompt: string;
  discord_game_name: string;
}

export interface LauncherConfig {
  variant: string;
  product_name: string;
  default_theme: string;
  app_identifier: string;
  discord_app_id: number;
  default_byond_version: string | null;
  features: LauncherFeatures;
  urls: LauncherUrls;
  strings: LauncherStrings;
  singleplayer: SingleplayerConfig;
}

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
      const config = await invoke<LauncherConfig>("get_launcher_config");
      set({ config, loading: false });
      return config;
    } catch (err) {
      console.error("Failed to load launcher config:", err);
      set({ loading: false });
      throw err;
    }
  },
}));
