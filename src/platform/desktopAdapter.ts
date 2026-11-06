import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import {
  ServerCatalogItem,
  ProgressPayload,
  LaunchResult,
  InstanceLifecycleStatus,
  ServerManifest,
} from "../types/launcher";
import { AccountProfile, AuthStatePayload } from "../types/auth";
import { UserSettings, ServerStatusPayload } from "../types/settings";
import { PlatformAdapter, PlatformCapabilities, UnsubscribeFn } from "./types";

/**
 * Desktop implementation of PlatformAdapter communicating with Rust core engine via Tauri v2 IPC.
 */
export class DesktopAdapter implements PlatformAdapter {
  readonly capabilities: PlatformCapabilities = {
    canManageLocalInstances: true,
    canDirectPing: true,
    canManageAuth: true,
    canConfigureJvm: true,
    canPruneStorage: true,
  };

  async getCatalog(): Promise<ServerCatalogItem[]> {
    try {
      return await invoke<ServerCatalogItem[]>("get_server_catalog");
    } catch (catErr) {
      console.warn("Could not load full catalog from core engine, falling back to sample:", catErr);
      const fixturePath = await invoke<string>("get_sample_manifest_path");
      const manifest = await invoke<ServerManifest>("load_manifest", { manifestPath: fixturePath });
      const fallbackItem: ServerCatalogItem = {
        id: manifest.serverId,
        name: manifest.name,
        manifestPath: fixturePath,
        manifest,
        tagline: "Official szz test server featuring Fabric 1.20.1.",
        status: "NotInstalled",
      };
      return [fallbackItem];
    }
  }

  async loadManifest(manifestPath: string): Promise<ServerManifest> {
    return await invoke<ServerManifest>("load_manifest", { manifestPath });
  }

  async play(serverId: string): Promise<void> {
    await this.launchInstance({ instanceId: serverId, dryRun: false });
  }

  async installInstance(params: { manifestPath?: string; instanceId: string; dryRun?: boolean }): Promise<void> {
    await invoke("install_instance", params);
  }

  async launchInstance(params: { instanceId: string; dryRun?: boolean }): Promise<LaunchResult> {
    return await invoke<LaunchResult>("launch_instance", params);
  }

  async uninstallInstance(instanceId: string): Promise<void> {
    await invoke("uninstall_instance", { instanceId });
  }

  async updateInstance(params: { manifest: ServerManifest; instanceId: string }): Promise<void> {
    await invoke("update_instance", params);
  }

  async getInstanceStatus(instanceId: string): Promise<InstanceLifecycleStatus> {
    return await invoke<InstanceLifecycleStatus>("get_instance_status", { instanceId });
  }

  async pingServer(address: string): Promise<ServerStatusPayload> {
    return await invoke<ServerStatusPayload>("ping_server", { address });
  }

  async getSettings(): Promise<UserSettings> {
    return await invoke<UserSettings>("get_settings");
  }

  async updateSettings(settings: UserSettings): Promise<UserSettings> {
    return await invoke<UserSettings>("update_settings", { settings });
  }

  async resetSettings(): Promise<UserSettings> {
    return await invoke<UserSettings>("reset_settings");
  }

  async getAuthState(): Promise<AuthStatePayload> {
    return await invoke<AuthStatePayload>("get_auth_state");
  }

  async switchAccount(accountId: string): Promise<void> {
    await invoke("switch_account", { accountId });
  }

  async removeAccount(accountId: string): Promise<void> {
    await invoke("remove_account", { accountId });
  }

  async createDevAccount(username: string): Promise<AccountProfile> {
    return await invoke<AccountProfile>("create_dev_account", { username });
  }

  async onLaunchProgress(callback: (payload: ProgressPayload) => void): Promise<UnsubscribeFn> {
    const unlisten: UnlistenFn = await listen<ProgressPayload>("launch-progress", (event) => {
      callback(event.payload);
    });
    return () => unlisten();
  }

  async onInstanceStatusChanged(
    callback: (payload: { instanceId: string; status: InstanceLifecycleStatus }) => void
  ): Promise<UnsubscribeFn> {
    const unlisten: UnlistenFn = await listen<{ instanceId: string; status: InstanceLifecycleStatus }>(
      "instance-status-changed",
      (event) => {
        callback(event.payload);
      }
    );
    return () => unlisten();
  }

  async onDeepLink(callback: (serverId: string) => void): Promise<UnsubscribeFn> {
    const unlisten: UnlistenFn = await listen<string>("szz://deep-link", (event) => {
      callback(event.payload);
    });
    return () => unlisten();
  }
}
