export type LoaderType = "fabric" | "forge" | "neoforge" | "liteloader" | "vanilla";

export interface LoaderConfig {
  type: LoaderType;
  version: string;
}

export interface ModEntry {
  name: string;
  source: string;
  fileUrl: string;
  sha1?: string;
  sha512?: string;
}

export interface ConfigSpec {
  url: string;
  hash?: string;
  version?: string;
}

export interface ServerManifest {
  serverId: string;
  name: string;
  serverAddress: string;
  minecraftVersion: string;
  loader: LoaderConfig;
  configUrl?: string | null;
  config?: ConfigSpec | string | null;
  manifestVersion: number;
  mods: ModEntry[];
}

export const isModEntryVerified = (mod: ModEntry): boolean => {
  const src = (mod.source || "").trim().toLowerCase();
  return (src === "modrinth" || src === "curseforge") && Boolean(mod.sha1 || mod.sha512);
};

export const isManifestVerified = (manifest?: ServerManifest | null): boolean => {
  if (!manifest) return true;
  return manifest.mods.every(isModEntryVerified);
};

export type LaunchStep =
  | "preparingInstance"
  | "checkingJavaRuntime"
  | "syncingMods"
  | "assemblingClasspath"
  | "spawningClient"
  | "complete";

export interface ProgressPayload {
  step: LaunchStep;
  stepNumber: number;
  totalSteps: number;
  message: string;
  percentage: number;
}

export interface LaunchResult {
  commandLine: string;
  pid?: number;
}

export type LaunchResultPayload = LaunchResult;

export type InstanceLifecycleStatus =
  | "NotInstalled"
  | "Installing"
  | "Installed"
  | "Running"
  | "UpdateAvailable"
  | "Updating";

export const normalizeStatus = (
  status?: InstanceLifecycleStatus | string | null
): InstanceLifecycleStatus => {
  if (!status) return "NotInstalled";
  const s = status.toLowerCase();
  if (s === "installed") return "Installed";
  if (s === "running") return "Running";
  if (s === "installing") return "Installing";
  if (s === "updateavailable") return "UpdateAvailable";
  if (s === "updating") return "Updating";
  return "NotInstalled";
};

export const isInstanceInstalled = (
  status?: InstanceLifecycleStatus | string | null
): boolean => {
  const norm = normalizeStatus(status);
  return (
    norm === "Installed" ||
    norm === "Running" ||
    norm === "UpdateAvailable" ||
    norm === "Updating"
  );
};

export const isInstanceRunning = (
  status?: InstanceLifecycleStatus | string | null
): boolean => {
  return normalizeStatus(status) === "Running";
};

export const isInstanceUpdateAvailable = (
  status?: InstanceLifecycleStatus | string | null
): boolean => {
  return normalizeStatus(status) === "UpdateAvailable";
};

export const isInstanceUpdating = (
  status?: InstanceLifecycleStatus | string | null
): boolean => {
  return normalizeStatus(status) === "Updating";
};

export interface InstalledInstanceInfo {
  instance_id: string;
  server_name: string;
  server_address: string;
  minecraft_version: string;
  loader: string;
  mod_count: number;
  status: InstanceLifecycleStatus;
}

export interface ServerCatalogItem {
  id: string;
  name: string;
  manifestPath: string;
  manifest?: ServerManifest;
  tagline: string;
  status?: InstanceLifecycleStatus;
  latestManifestVersion?: number;
  isOfflineCache?: boolean;
}
