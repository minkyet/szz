import {
  ServerCatalogItem,
  ProgressPayload,
  LaunchResult,
  InstanceLifecycleStatus,
  ServerManifest,
} from "../types/launcher";
import { AccountProfile, AuthStatePayload } from "../types/auth";
import {
  UserSettings,
  ServerStatusPayload,
  DEFAULT_USER_SETTINGS,
} from "../types/settings";
import { PlatformAdapter, PlatformCapabilities, UnsubscribeFn } from "./types";
import { UnsupportedPlatformError } from "./errors";

// Static JSON fixture imports as fallback for test runners / environments without Vite glob
import sampleManifest from "../../fixtures/sample-manifest.json";
import cobblemonManifest from "../../fixtures/servers/cobblemon-1.21.1.json";
import skyfactoryManifest from "../../fixtures/servers/skyfactory4-1.12.2.json";
import hypixelModernManifest from "../../fixtures/servers/hypixel-26.2.json";
import hypixelLegacyManifest from "../../fixtures/servers/hypixel-1.8.9.json";

const STATIC_FIXTURES: ServerManifest[] = [
  cobblemonManifest as unknown as ServerManifest,
  skyfactoryManifest as unknown as ServerManifest,
  hypixelModernManifest as unknown as ServerManifest,
  hypixelLegacyManifest as unknown as ServerManifest,
  sampleManifest as unknown as ServerManifest,
];

const SZZ_USER_SETTINGS_STORAGE_KEY = "szz_user_settings";

let cachedManifests: ServerManifest[] | null = null;

function loadBundledManifests(): ServerManifest[] {
  if (cachedManifests) {
    return cachedManifests;
  }

  try {
    if (typeof import.meta !== "undefined" && typeof (import.meta as any).glob === "function") {
      const globbed = (import.meta as any).glob("../../fixtures/servers/*.json", { eager: true });
      const manifests = Object.values(globbed).map((mod: any) => (mod.default ? mod.default : mod));
      if (manifests.length > 0) {
        cachedManifests = manifests;
        return cachedManifests;
      }
    }
  } catch {
    // Non-Vite fallback
  }
  cachedManifests = STATIC_FIXTURES;
  return cachedManifests;
}

function manifestToCatalogItem(manifest: ServerManifest): ServerCatalogItem {
  const loaderDesc = manifest.loader
    ? `${manifest.loader.type.toUpperCase()} ${manifest.loader.version}`
    : "Vanilla";
  return {
    id: manifest.serverId,
    name: manifest.name,
    manifestPath: `/fixtures/servers/${manifest.serverId}.json`,
    manifest,
    tagline: `Minecraft ${manifest.minecraftVersion} (${loaderDesc}) - ${manifest.mods?.length ?? 0} mods included`,
    status: "NotInstalled",
  };
}

/**
 * Web browser implementation of PlatformAdapter.
 * Operates in client-side sandbox using bundled fixtures, localStorage, and szz:// custom protocol deep links.
 */
export class WebAdapter implements PlatformAdapter {
  readonly capabilities: PlatformCapabilities = {
    canManageLocalInstances: false,
    canDirectPing: false,
    canManageAuth: false,
    canConfigureJvm: false,
    canPruneStorage: false,
  };

  async getCatalog(): Promise<ServerCatalogItem[]> {
    const manifests = loadBundledManifests();
    return manifests.map(manifestToCatalogItem);
  }

  async loadManifest(manifestPath: string): Promise<ServerManifest> {
    const manifests = loadBundledManifests();
    const found = manifests.find(
      (m) =>
        m.serverId === manifestPath ||
        manifestPath.includes(m.serverId)
    );
    if (found) {
      return found;
    }
    return sampleManifest as unknown as ServerManifest;
  }

  async play(serverId: string): Promise<void> {
    const uri = `szz://launch/${encodeURIComponent(serverId)}`;
    if (typeof window !== "undefined" && window.location) {
      window.location.href = uri;
    }
  }

  async installInstance(_params: { manifestPath?: string; instanceId: string; dryRun?: boolean }): Promise<void> {
    throw new UnsupportedPlatformError(
      "canManageLocalInstances",
      "installInstance",
      "로컬 인스턴스 설치는 데스크톱 szz 런처에서만 가능합니다. [szz로 플레이]를 클릭하여 런처를 실행하세요."
    );
  }

  async launchInstance(_params: { instanceId: string; dryRun?: boolean }): Promise<LaunchResult> {
    throw new UnsupportedPlatformError(
      "canManageLocalInstances",
      "launchInstance",
      "마인크래프트 클라이언트 실행은 데스크톱 szz 런처에서만 가능합니다."
    );
  }

  async uninstallInstance(_instanceId: string): Promise<void> {
    throw new UnsupportedPlatformError(
      "canManageLocalInstances",
      "uninstallInstance",
      "인스턴스 제거는 데스크톱 szz 런처에서만 가능합니다."
    );
  }

  async updateInstance(_params: { manifest: ServerManifest; instanceId: string }): Promise<void> {
    throw new UnsupportedPlatformError(
      "canManageLocalInstances",
      "updateInstance",
      "인스턴스 업데이트는 데스크톱 szz 런처에서만 가능합니다."
    );
  }

  async getInstanceStatus(_instanceId: string): Promise<InstanceLifecycleStatus> {
    return "NotInstalled";
  }

  async pingServer(address: string): Promise<ServerStatusPayload> {
    return {
      address,
      online: true,
      latencyMs: 45,
      playersOnline: 24,
      playersMax: 100,
      motd: "§a§lszz §7Multiplayer Server (Web Preview)",
      versionName: "1.21.1",
    };
  }

  async getSettings(): Promise<UserSettings> {
    if (typeof window !== "undefined" && window.localStorage) {
      const raw = window.localStorage.getItem(SZZ_USER_SETTINGS_STORAGE_KEY);
      if (raw) {
        try {
          return JSON.parse(raw);
        } catch {
          // ignore corrupted JSON
        }
      }
    }
    return DEFAULT_USER_SETTINGS;
  }

  async updateSettings(settings: UserSettings): Promise<UserSettings> {
    if (typeof window !== "undefined" && window.localStorage) {
      window.localStorage.setItem(SZZ_USER_SETTINGS_STORAGE_KEY, JSON.stringify(settings));
    }
    return settings;
  }

  async resetSettings(): Promise<UserSettings> {
    if (typeof window !== "undefined" && window.localStorage) {
      window.localStorage.removeItem(SZZ_USER_SETTINGS_STORAGE_KEY);
    }
    return DEFAULT_USER_SETTINGS;
  }

  async getAuthState(): Promise<AuthStatePayload> {
    const guestProfile: AccountProfile = {
      id: "guest",
      account_type: "dev",
      uuid: "00000000-0000-0000-0000-000000000000",
      username: "Guest",
      skin_url: null,
      is_active: true,
    };

    return {
      accounts: [guestProfile],
      active_account: guestProfile,
    };
  }

  async switchAccount(_accountId: string): Promise<void> {
    throw new UnsupportedPlatformError(
      "canManageAuth",
      "switchAccount",
      "계정 전환은 데스크톱 szz 런처에서만 가능합니다."
    );
  }

  async removeAccount(_accountId: string): Promise<void> {
    throw new UnsupportedPlatformError(
      "canManageAuth",
      "removeAccount",
      "계정 삭제는 데스크톱 szz 런처에서만 가능합니다."
    );
  }

  async createDevAccount(_username: string): Promise<AccountProfile> {
    throw new UnsupportedPlatformError(
      "canManageAuth",
      "createDevAccount",
      "계정 생성은 데스크톱 szz 런처에서만 가능합니다."
    );
  }

  async onLaunchProgress(_callback: (payload: ProgressPayload) => void): Promise<UnsubscribeFn> {
    return () => {};
  }

  async onInstanceStatusChanged(
    _callback: (payload: { instanceId: string; status: InstanceLifecycleStatus }) => void
  ): Promise<UnsubscribeFn> {
    return () => {};
  }

  async onDeepLink(_callback: (serverId: string) => void): Promise<UnsubscribeFn> {
    return () => {};
  }
}
