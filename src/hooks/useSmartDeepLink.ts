import { useState, useCallback, useRef, useEffect } from "react";
import { usePlatform } from "../platform";

export interface SmartDeepLinkOptions {
  /**
   * Timeout in milliseconds to wait for browser focus loss.
   * Defaults to 2500ms.
   */
  timeoutMs?: number;
  /**
   * Maximum safety timeout in milliseconds when launcher is assumed running in foreground.
   * Defaults to Math.max(timeoutMs * 2, 5000ms).
   */
  maxTimeoutMs?: number;
  /**
   * Callback triggered when launcher is detected as not installed / canceled.
   */
  onFallback?: (serverId: string) => void;
  /**
   * Callback triggered when launcher protocol appears to have succeeded.
   */
  onSuccess?: (serverId: string) => void;
}

/**
 * Pure detector class managing browser focus listeners and timeout logic.
 * Decoupled from React to enable direct unit testing in Node/browser environments.
 */
export class SmartDeepLinkDetector {
  private fallbackTimeoutId: ReturnType<typeof setTimeout> | null = null;
  private maxTimeoutId: ReturnType<typeof setTimeout> | null = null;
  private focusDebounceId: ReturnType<typeof setTimeout> | null = null;
  private cleanupListeners: (() => void) | null = null;
  private pendingResolve: ((val: boolean) => void) | null = null;
  private hasBlurred: boolean = false;
  private _isPending: boolean = false;

  constructor(
    private playFn: (serverId: string) => Promise<void>,
    private options: SmartDeepLinkOptions = {}
  ) {}

  get isPending(): boolean {
    return this._isPending;
  }

  private cleanup(): void {
    if (this.fallbackTimeoutId) {
      clearTimeout(this.fallbackTimeoutId);
      this.fallbackTimeoutId = null;
    }
    if (this.maxTimeoutId) {
      clearTimeout(this.maxTimeoutId);
      this.maxTimeoutId = null;
    }
    if (this.focusDebounceId) {
      clearTimeout(this.focusDebounceId);
      this.focusDebounceId = null;
    }
    if (this.cleanupListeners) {
      this.cleanupListeners();
      this.cleanupListeners = null;
    }
    this._isPending = false;
  }

  cancel(): void {
    this.cleanup();
    if (this.pendingResolve) {
      const resolve = this.pendingResolve;
      this.pendingResolve = null;
      resolve(false);
    }
  }

  async trigger(serverId: string): Promise<boolean> {
    this.cancel();
    this._isPending = true;
    this.hasBlurred = false;

    return new Promise<boolean>((resolve, reject) => {
      this.pendingResolve = resolve;

      const handleBlur = () => {
        this.hasBlurred = true;
        // Blur indicates the browser recognized the protocol scheme (e.g. showed prompt
        // or shifted focus to the desktop app). Cancel any pending fallback trigger and debounce.
        if (this.fallbackTimeoutId) {
          clearTimeout(this.fallbackTimeoutId);
          this.fallbackTimeoutId = null;
        }
        if (this.focusDebounceId) {
          clearTimeout(this.focusDebounceId);
          this.focusDebounceId = null;
        }
      };

      const handleFocus = () => {
        // When focus returns after blur, the user closed the browser prompt (clicked Open or Cancel).
        // Since blur occurred, the protocol was recognized by the OS/browser.
        // Resolve successfully and clear pending state without opening fallback modal.
        if (this.hasBlurred && this._isPending) {
          if (this.focusDebounceId) {
            clearTimeout(this.focusDebounceId);
          }
          this.focusDebounceId = setTimeout(() => {
            if (this._isPending) {
              const res = this.pendingResolve;
              this.pendingResolve = null;
              this.cleanup();
              this.options.onSuccess?.(serverId);
              res?.(true);
            }
          }, 200);
        }
      };

      const handleVisibilityChange = () => {
        if (typeof document !== "undefined") {
          if (document.hidden) {
            handleBlur();
          } else if (this.hasBlurred) {
            handleFocus();
          }
        }
      };

      if (typeof window !== "undefined") {
        window.addEventListener("blur", handleBlur);
        window.addEventListener("focus", handleFocus);
      }
      if (typeof document !== "undefined") {
        document.addEventListener("visibilitychange", handleVisibilityChange);
      }

      this.cleanupListeners = () => {
        if (typeof window !== "undefined") {
          window.removeEventListener("blur", handleBlur);
          window.removeEventListener("focus", handleFocus);
        }
        if (typeof document !== "undefined") {
          document.removeEventListener("visibilitychange", handleVisibilityChange);
        }
      };

      // Fallback timer: Only triggers if NO blur ever occurred within timeoutMs (protocol unknown)
      const timeoutMs = this.options.timeoutMs ?? 2500;
      this.fallbackTimeoutId = setTimeout(() => {
        if (this._isPending) {
          if (!this.hasBlurred) {
            const res = this.pendingResolve;
            this.pendingResolve = null;
            this.cleanup();
            this.options.onFallback?.(serverId);
            res?.(false);
          }
        }
      }, timeoutMs);

      // Max safety timeout
      const maxTimeoutMs =
        this.options.maxTimeoutMs ?? Math.max(timeoutMs * 2, 5000);
      this.maxTimeoutId = setTimeout(() => {
        if (this._isPending) {
          const res = this.pendingResolve;
          this.pendingResolve = null;
          this.cleanup();
          if (this.hasBlurred) {
            this.options.onSuccess?.(serverId);
            res?.(true);
          } else {
            this.options.onFallback?.(serverId);
            res?.(false);
          }
        }
      }, maxTimeoutMs);

      this.playFn(serverId).catch((err) => {
        this.pendingResolve = null;
        this.cleanup();
        reject(err);
      });
    });
  }
}

export interface UseSmartDeepLinkReturn {
  triggerPlay: (serverId: string) => Promise<boolean>;
  isPending: boolean;
  isFallbackOpen: boolean;
  closeFallback: () => void;
  lastServerId: string | null;
}

/**
 * Hook to trigger szz:// custom protocol deep links and detect if the desktop launcher
 * is installed by monitoring browser focus transitions (window.onblur and window.onfocus).
 */
export function useSmartDeepLink(options: SmartDeepLinkOptions = {}): UseSmartDeepLinkReturn {
  const adapter = usePlatform();
  const [isPending, setIsPending] = useState<boolean>(false);
  const [isFallbackOpen, setIsFallbackOpen] = useState<boolean>(false);
  const [lastServerId, setLastServerId] = useState<string | null>(null);

  const optionsRef = useRef<SmartDeepLinkOptions>(options);
  optionsRef.current = options;

  const detectorRef = useRef<SmartDeepLinkDetector | null>(null);

  useEffect(() => {
    detectorRef.current = new SmartDeepLinkDetector((id) => adapter.play(id), {
      ...options,
      onFallback: (id) => {
        setIsFallbackOpen(true);
        optionsRef.current.onFallback?.(id);
      },
      onSuccess: (id) => {
        setIsFallbackOpen(false);
        optionsRef.current.onSuccess?.(id);
      },
    });

    return () => {
      detectorRef.current?.cancel();
    };
  }, [adapter, options.timeoutMs, options.maxTimeoutMs]);

  const triggerPlay = useCallback(async (serverId: string): Promise<boolean> => {
    setLastServerId(serverId);
    setIsPending(true);
    try {
      const result = await (detectorRef.current
        ? detectorRef.current.trigger(serverId)
        : adapter.play(serverId).then(() => true));
      return result;
    } finally {
      setIsPending(false);
    }
  }, [adapter]);

  const closeFallback = useCallback(() => {
    detectorRef.current?.cancel();
    setIsFallbackOpen(false);
  }, []);

  return {
    triggerPlay,
    isPending,
    isFallbackOpen,
    closeFallback,
    lastServerId,
  };
}
