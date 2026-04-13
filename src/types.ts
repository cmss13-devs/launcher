import type { SteamUserInfo } from "./bindings";

export interface SteamAuthState {
  available: boolean;
  user: SteamUserInfo | null;
  access_token: string | null;
  loading: boolean;
  error: string | null;
}

export interface ErrorNotification {
  id: number;
  message: string;
}

export type WineSetupStage = "in_progress" | "complete" | "error";

export interface WineSetupProgress {
  stage: WineSetupStage;
  progress: number;
  message: string;
}

export type Platform = "windows" | "linux" | "macos" | "unknown";
