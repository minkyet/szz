import { PlatformAdapter } from "./types";
import { isTauri } from "./utils";
import { DesktopAdapter } from "./desktopAdapter";
import { WebAdapter } from "./webAdapter";

export * from "./types";
export * from "./errors";
export * from "./utils";
export * from "./desktopAdapter";
export * from "./webAdapter";
export * from "./context";

let singletonAdapter: PlatformAdapter | null = null;

/**
 * Factory function returning the appropriate PlatformAdapter for the current runtime.
 */
export function createPlatformAdapter(forceNew: boolean = false): PlatformAdapter {
  if (singletonAdapter && !forceNew) {
    return singletonAdapter;
  }

  if (isTauri()) {
    singletonAdapter = new DesktopAdapter();
  } else {
    singletonAdapter = new WebAdapter();
  }

  return singletonAdapter;
}
