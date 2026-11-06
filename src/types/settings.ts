import { ThemeMode } from "./theme";

export type LauncherBehaviorOnLaunch = "keep_open" | "minimize" | "close";

export interface GameSettings {
  resolutionWidth: number;
  resolutionHeight: number;
  fullscreen: boolean;
  autoConnect: boolean;
}

export interface JavaSettings {
  allocatedMemoryMb: number;
  customJvmArgs: string[];
}

export interface LauncherSettings {
  behaviorOnLaunch: LauncherBehaviorOnLaunch;
  theme: ThemeMode;
}

export interface UserSettings {
  game: GameSettings;
  java: JavaSettings;
  launcher: LauncherSettings;
}

export interface SystemMemoryInfo {
  totalMemoryMb: number;
  availableMemoryMb: number;
}

export interface ServerStatusPayload {
  address: string;
  online: boolean;
  latencyMs?: number | null;
  playersOnline?: number | null;
  playersMax?: number | null;
  motd?: string | null;
  versionName?: string | null;
}

export const DEFAULT_USER_SETTINGS: UserSettings = {
  game: {
    resolutionWidth: 1280,
    resolutionHeight: 720,
    fullscreen: false,
    autoConnect: true,
  },
  java: {
    allocatedMemoryMb: 4096,
    customJvmArgs: [],
  },
  launcher: {
    behaviorOnLaunch: "keep_open",
    theme: "latte",
  },
};
