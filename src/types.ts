export interface UserInfo {
  sub: string;
  name?: string;
  preferred_username?: string;
  email?: string;
  email_verified?: boolean;
}

export interface AuthState {
  logged_in: boolean;
  user: UserInfo | null;
  loading: boolean;
  error: string | null;
}

export type AuthMode = "oidc" | "hub" | "byond" | "steam";

export type Theme = "tgui" | "crt";

export interface SteamUserInfo {
  steam_id: string;
  display_name: string;
}

export interface SteamAuthResult {
  success: boolean;
  user_exists: boolean;
  access_token: string | null;
  requires_linking: boolean;
  linking_url: string | null;
  error: string | null;
}

export interface SteamLaunchOptions {
  raw: string;
  server_name: string | null;
}

export interface SteamAuthState {
  available: boolean;
  user: SteamUserInfo | null;
  access_token: string | null;
  loading: boolean;
  error: string | null;
}

export interface AppSettings {
  auth_mode: AuthMode;
  theme: Theme;
  notification_servers: string[];
  fullscreen_overlay: boolean;
}

export interface ErrorNotification {
  id: number;
  message: string;
}

export interface Relay {
  id: string;
  name: string;
  host: string;
}

export interface RelayWithPing extends Relay {
  ping: number | null;
  checking: boolean;
}

export interface ServerData {
  round_id: number;
  mode: string;
  map_name: string;
  round_duration: number;
  gamestate: number;
  players: number;
  admins?: number;
  popcap?: number;
  security_level?: string;
}

export interface Server {
  name: string;
  url: string;
  status: string;
  hub_status: string;
  players: number;
  data?: ServerData;
  is_18_plus: boolean;
  version?: string;
  recommended_byond_version?: string;
  tags?: string[];
}

export interface WineStatus {
  installed: boolean;
  version: string | null;
  meets_minimum_version: boolean;
  winetricks_installed: boolean;
  prefix_initialized: boolean;
  webview2_installed: boolean;
  error: string | null;
}

export type WineSetupStage = "in_progress" | "complete" | "error";

export interface WineSetupProgress {
  stage: WineSetupStage;
  progress: number;
  message: string;
}

export type Platform = "windows" | "linux" | "macos" | "unknown";

export interface SinglePlayerStatus {
  installed: boolean;
  version: string | null;
  release_tag: string | null;
  path: string | null;
}

export interface ReleaseInfo {
  tag_name: string;
  name: string;
  published_at: string;
  download_url: string | null;
  size: number | null;
}

export interface ByondLoginResult {
  username: string | null;
}

export interface ByondSessionCheck {
  logged_in: boolean;
  username: string | null;
  web_id: string | null;
}
