import { isTauri as tauriIsTauri } from "@tauri-apps/api/core";

/**
 * Returns true if the application is executing inside a Tauri webview desktop environment.
 */
export function isTauri(): boolean {
  try {
    if (typeof tauriIsTauri === "function") {
      return tauriIsTauri();
    }
  } catch {
    // Fall back to global object detection if @tauri-apps/api/core is not initialized
  }

  const globalScope = (typeof globalThis !== "undefined"
    ? globalThis
    : typeof window !== "undefined"
    ? window
    : {}) as { isTauri?: boolean; __TAURI_INTERNALS__?: unknown };

  return Boolean(globalScope.isTauri || globalScope.__TAURI_INTERNALS__);
}
