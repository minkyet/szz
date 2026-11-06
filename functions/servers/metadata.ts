import { ServerManifest } from "../../src/types/launcher";
import sampleManifest from "../../fixtures/sample-manifest.json";
import cobblemonManifest from "../../fixtures/servers/cobblemon-1.21.1.json";
import skyfactoryManifest from "../../fixtures/servers/skyfactory4-1.12.2.json";
import hypixelModernManifest from "../../fixtures/servers/hypixel-26.2.json";
import hypixelLegacyManifest from "../../fixtures/servers/hypixel-1.8.9.json";

export interface ServerOgMetadata {
  serverId: string;
  title: string;
  description: string;
  bannerUrl: string;
  url: string;
  themeColor: string;
  minecraftVersion?: string;
  loader?: string;
  modsCount?: number;
}

export const DEFAULT_SZZ_METADATA: ServerOgMetadata = {
  serverId: "default",
  title: "szz — 마인크래프트 원클릭 런처 & 플랫폼",
  description: "웹에서 둘러보고 원클릭으로 로비 직행, 자바 수동 설치와 디스크 용량 낭비 없는 스팀 스타일 마인크래프트 플랫폼",
  bannerUrl: "https://szz.nz/icons/icon.png",
  url: "https://szz.nz/",
  themeColor: "#A7C957",
};

/** Whitelist pattern: alphanumeric, hyphen, and underscore, max 64 chars */
const SERVER_ID_SAFE_REGEX = /^[a-zA-Z0-9_-]{1,64}$/;

const STATIC_MANIFESTS: ServerManifest[] = [
  cobblemonManifest as unknown as ServerManifest,
  skyfactoryManifest as unknown as ServerManifest,
  hypixelModernManifest as unknown as ServerManifest,
  hypixelLegacyManifest as unknown as ServerManifest,
  sampleManifest as unknown as ServerManifest,
];

/** Canonical server ID aliases for clean short URLs */
const SERVER_ID_ALIASES: Record<string, string> = {
  "skyfactory-4": "skyfactory-4-1-12-2",
  "skyfactory4": "skyfactory-4-1-12-2",
  "skyfactory4-1-12-2": "skyfactory-4-1-12-2",
  "cobblemon": "cobblemon-1-21-1",
  "hypixel": "hypixel-26-2",
  "hypixel-modern": "hypixel-26-2",
  "hypixel-legacy": "hypixel-1-8-9",
  "sample": "szz-test-fabric",
};

/**
 * Escapes special HTML characters to prevent XSS in meta attribute injections.
 */
export function escapeHtml(str: string): string {
  return str
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/**
 * Validates and resolves server metadata for OpenGraph preview tags.
 * Falls back safely to default szz metadata if id is invalid or unknown.
 */
export function resolveServerMetadata(rawId: unknown, baseUrl = "https://szz.nz"): ServerOgMetadata {
  if (typeof rawId !== "string" || !SERVER_ID_SAFE_REGEX.test(rawId.trim())) {
    return { ...DEFAULT_SZZ_METADATA, url: baseUrl };
  }

  const cleanId = rawId.trim();
  const canonicalId = SERVER_ID_ALIASES[cleanId] || cleanId;
  const normClean = canonicalId.toLowerCase().replace(/[^a-z0-9]/g, "");

  // Find matching server manifest in bundled fixtures (exact serverId, alias, or exact alphanumeric match)
  const found = STATIC_MANIFESTS.find((m) => {
    if (m.serverId === canonicalId) return true;
    const normServer = m.serverId.toLowerCase().replace(/[^a-z0-9]/g, "");
    return normServer === normClean;
  });

  if (!found) {
    return { ...DEFAULT_SZZ_METADATA, url: `${baseUrl}/servers/${cleanId}` };
  }

  const loaderName = found.loader ? found.loader.type.toUpperCase() : "Vanilla";
  const modsCount = found.mods?.length ?? 0;
  const modsText = modsCount > 0 ? ` · ${modsCount}개 모드 포함` : "";

  return {
    serverId: found.serverId,
    title: `${found.name} — szz 마인크래프트`,
    description: `Minecraft ${found.minecraftVersion} (${loaderName})${modsText} | szz 원클릭 QuickPlay 플레이`,
    bannerUrl: `${baseUrl}/icons/icon.png`,
    url: `${baseUrl}/servers/${found.serverId}`,
    themeColor: "#A7C957",
    minecraftVersion: found.minecraftVersion,
    loader: loaderName,
    modsCount,
  };
}

/**
 * Generates an array of OpenGraph and Twitter HTML meta tags.
 */
export function buildMetaTagsHtml(meta: ServerOgMetadata): string {
  const escTitle = escapeHtml(meta.title);
  const escDesc = escapeHtml(meta.description);
  const escBanner = escapeHtml(meta.bannerUrl);
  const escUrl = escapeHtml(meta.url);
  const escTheme = escapeHtml(meta.themeColor);

  return [
    `<meta property="og:title" content="${escTitle}" />`,
    `<meta property="og:description" content="${escDesc}" />`,
    `<meta property="og:image" content="${escBanner}" />`,
    `<meta property="og:url" content="${escUrl}" />`,
    `<meta property="og:type" content="website" />`,
    `<meta name="twitter:card" content="summary_large_image" />`,
    `<meta name="twitter:title" content="${escTitle}" />`,
    `<meta name="twitter:description" content="${escDesc}" />`,
    `<meta name="twitter:image" content="${escBanner}" />`,
    `<meta name="theme-color" content="${escTheme}" />`,
  ].join("\n    ");
}
