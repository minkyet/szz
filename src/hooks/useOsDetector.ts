import { useState } from "react";

export type DetectedOS = "windows" | "macos" | "linux" | "unknown";

export interface OsReleaseAsset {
  os: DetectedOS;
  label: string;
  osName: string;
  filename: string;
  downloadUrl: string;
  badge: string;
  description: string;
}

export const RELEASE_ASSETS: Record<Exclude<DetectedOS, "unknown">, OsReleaseAsset> = {
  windows: {
    os: "windows",
    label: "Windows (64-bit)",
    osName: "Windows",
    filename: "szz-setup.exe",
    downloadUrl: "https://github.com/minkyet/szz/releases/latest/download/szz-setup.exe",
    badge: ".exe (Windows 10/11)",
    description: "Windows 10 / 11 64-bit 설치 인스톨러",
  },
  macos: {
    os: "macos",
    label: "macOS (Universal)",
    osName: "macOS",
    filename: "szz-universal.dmg",
    downloadUrl: "https://github.com/minkyet/szz/releases/latest/download/szz-universal.dmg",
    badge: ".dmg (macOS 11+)",
    description: "Apple Silicon (M1/M2/M3) 및 Intel Mac 범용 디스크 이미지",
  },
  linux: {
    os: "linux",
    label: "Linux (.deb / AppImage)",
    osName: "Linux",
    filename: "szz-linux-amd64.deb",
    downloadUrl: "https://github.com/minkyet/szz/releases/latest/download/szz-linux-amd64.deb",
    badge: ".deb / AppImage (x86_64)",
    description: "Ubuntu/Debian 패키지 및 모든 Linux 배포판용 실행 파일",
  },
};

export const detectClientOS = (userAgent: string, platform?: string): DetectedOS => {
  const ua = userAgent.toLowerCase();
  const plat = (platform || "").toLowerCase();

  // Mobile check first to avoid false positives (e.g. Android contains 'linux')
  if (ua.includes("android") || ua.includes("iphone") || ua.includes("ipad") || ua.includes("ipod")) {
    return "unknown";
  }

  if (plat.includes("win") || ua.includes("win")) {
    return "windows";
  }
  if (plat.includes("mac") || ua.includes("macintosh") || ua.includes("mac os x")) {
    return "macos";
  }
  if (plat.includes("linux") || ua.includes("linux") || ua.includes("x11")) {
    return "linux";
  }

  return "unknown";
};

export interface OsDetectorResult {
  os: DetectedOS;
  primaryAsset: OsReleaseAsset;
  allAssets: OsReleaseAsset[];
}

export const ALL_RELEASE_ASSETS: OsReleaseAsset[] = [
  RELEASE_ASSETS.windows,
  RELEASE_ASSETS.macos,
  RELEASE_ASSETS.linux,
];

export const useOsDetector = (): OsDetectorResult => {
  const [detectedOs] = useState<DetectedOS>(() => {
    if (typeof window === "undefined" || !window.navigator) {
      return "windows"; // SSR / build fallback
    }
    return detectClientOS(window.navigator.userAgent, window.navigator.platform);
  });

  const primaryAsset =
    detectedOs !== "unknown" ? RELEASE_ASSETS[detectedOs] : RELEASE_ASSETS.windows;

  return {
    os: detectedOs,
    primaryAsset,
    allAssets: ALL_RELEASE_ASSETS,
  };
};
