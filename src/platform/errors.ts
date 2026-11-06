import { PlatformCapabilities } from "./types";

/**
 * Error thrown when a desktop-only operation is attempted on an environment lacking the capability.
 */
export class UnsupportedPlatformError extends Error {
  constructor(
    public readonly capability: keyof PlatformCapabilities,
    public readonly method: string,
    message?: string
  ) {
    super(
      message ??
        `Platform does not support '${method}' (requires capability: ${capability})`
    );
    this.name = "UnsupportedPlatformError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}
