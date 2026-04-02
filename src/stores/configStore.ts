import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

export type ServerApiType = "hub" | "cm_ss13";

export interface LauncherFeatures {
  relay_selector: boolean;
  singleplayer: boolean;
  server_search: boolean;
  server_filters: boolean;
  show_offline_servers: boolean;
  server_stats: boolean;
  auto_launch_byond: boolean;
  connection_timeout_fallback: boolean;
}

export interface SocialLink {
  name: string;
  url: string;
  icon: string;
}

export interface SingleplayerConfig {
  github_repo: string | null;
  build_asset_name: string | null;
  dmb_name: string | null;
}

export interface LauncherUrls {
  server_api: string;
  hub_api: string | null;
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

export interface OidcConfig {
  client_id: string;
  auth_url: string;
  token_url: string;
  userinfo_url: string;
}

export interface LauncherConfig {
  variant: string;
  product_name: string;
  logo: string;
  default_theme: string;
  app_identifier: string;
  discord_app_id: number;
  default_byond_version: string | null;
  server_api: ServerApiType;
  features: LauncherFeatures;
  urls: LauncherUrls;
  strings: LauncherStrings;
  singleplayer: SingleplayerConfig;
  oidc: OidcConfig | null;
  social_links: SocialLink[];
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
