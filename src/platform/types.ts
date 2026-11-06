import {
  ServerCatalogItem,
  ProgressPayload,
  LaunchResult,
  InstanceLifecycleStatus,
  ServerManifest,
} from "../types/launcher";
import { AccountProfile, AuthStatePayload } from "../types/auth";
import { UserSettings, ServerStatusPayload } from "../types/settings";

/**
 * Declares the capabilities of the current execution platform.
 */
export interface PlatformCapabilities {
  /** Whether the platform can manage local filesystem instances (install, uninstall, update) */
  readonly canManageLocalInstances: boolean;
  /** Whether the platform can perform direct raw TCP Server List Ping to Minecraft servers */
  readonly canDirectPing: boolean;
  /** Whether the platform can manage authentication profiles and tokens locally */
  readonly canManageAuth: boolean;
  /** Whether the platform can configure JVM memory and execution arguments */
  readonly canConfigureJvm: boolean;
  /** Whether the platform can manage and prune local CAS storage */
  readonly canPruneStorage: boolean;
}

export type UnsubscribeFn = () => void;

/**
 * Universal platform abstraction interface for Desktop (Tauri IPC) and Web (Browser/Mock).
 */
export interface PlatformAdapter {
  readonly capabilities: PlatformCapabilities;

  // Catalog & Manifests
  getCatalog(): Promise<ServerCatalogItem[]>;
  loadManifest(manifestPath: string): Promise<ServerManifest>;

  // Execution & Lifecycle
  play(serverId: string): Promise<void>;
  installInstance(params: { manifestPath?: string; instanceId: string; dryRun?: boolean }): Promise<void>;
  launchInstance(params: { instanceId: string; dryRun?: boolean }): Promise<LaunchResult>;
  uninstallInstance(instanceId: string): Promise<void>;
  updateInstance(params: { manifest: ServerManifest; instanceId: string }): Promise<void>;
  getInstanceStatus(instanceId: string): Promise<InstanceLifecycleStatus>;

  // Server Status (SLP Ping)
  pingServer(address: string): Promise<ServerStatusPayload>;

  // Settings
  getSettings(): Promise<UserSettings>;
  updateSettings(settings: UserSettings): Promise<UserSettings>;
  resetSettings(): Promise<UserSettings>;

  // Authentication
  getAuthState(): Promise<AuthStatePayload>;
  switchAccount(accountId: string): Promise<void>;
  removeAccount(accountId: string): Promise<void>;
  createDevAccount(username: string): Promise<AccountProfile>;

  // Events & Subscriptions
  onLaunchProgress(callback: (payload: ProgressPayload) => void): Promise<UnsubscribeFn>;
  onInstanceStatusChanged(
    callback: (payload: { instanceId: string; status: InstanceLifecycleStatus }) => void
  ): Promise<UnsubscribeFn>;
  onDeepLink(callback: (serverId: string) => void): Promise<UnsubscribeFn>;
}
