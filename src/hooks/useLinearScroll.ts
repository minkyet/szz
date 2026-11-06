import { useEffect, RefObject } from "react";
import { isTauri } from "../platform";
import { detectClientOS } from "./useOsDetector";

interface LinearScrollOptions {
  speed?: number;
  enabled?: boolean;
}

/**
 * Normalizes mouse wheel and keyboard scrolling on Linux WebKitGTK.
 *
 * WebKitGTK has a known upstream defect where its native ScrollAnimatorGtk
 * causes kinetic momentum stuttering, erratic wheel acceleration, and key-repeat throttling.
 * This hook bypasses the buggy native animator on Linux by calling e.preventDefault()
 * and applying consistent, 1:1 linear scroll steps directly.
 *
 * On macOS and Windows, native scrolling is 100% preserved.
 */
export function useLinearScroll(
  containerRefOrElement: RefObject<HTMLElement | null> | HTMLElement | null,
  options?: LinearScrollOptions
) {
  const enabled = options?.enabled ?? true;
  const speed = options?.speed ?? 1.0;
  const container =
    containerRefOrElement && "current" in containerRefOrElement
      ? containerRefOrElement.current
      : containerRefOrElement;

  useEffect(() => {
    if (!enabled) return;

    // Only activate on Linux WebKitGTK desktop app, never in standard Linux web browsers
    const isLinux =
      typeof navigator !== "undefined" &&
      detectClientOS(navigator.userAgent, navigator.platform) === "linux";
    if (!isLinux || !isTauri()) return;

    if (!container) return;

    const handleWheel = (e: WheelEvent) => {
      // Check if mouse is hovering over an inner scrollable element
      const delta = (e.deltaMode === 1 ? e.deltaY * 32 : e.deltaY) * speed;
      let target = e.target as HTMLElement | null;
      let handled = false;

      while (target && target !== container) {
        const overflowY = window.getComputedStyle(target).overflowY;
        if (
          (overflowY === "auto" || overflowY === "scroll") &&
          target.scrollHeight > target.clientHeight
        ) {
          const canDown =
            target.scrollTop + target.clientHeight < target.scrollHeight - 1;
          const canUp = target.scrollTop > 1;

          if ((delta > 0 && canDown) || (delta < 0 && canUp)) {
            e.preventDefault();
            target.scrollTop += delta;
            handled = true;
            break;
          }
        }
        target = target.parentElement;
      }

      if (!handled) {
        const canDown =
          container.scrollTop + container.clientHeight <
          container.scrollHeight - 1;
        const canUp = container.scrollTop > 1;

        if ((delta > 0 && canDown) || (delta < 0 && canUp)) {
          e.preventDefault();
          container.scrollTop += delta;
        }
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      // Do not intercept arrow keys if focused inside input / textarea
      const target = e.target as HTMLElement | null;
      if (
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable)
      ) {
        return;
      }

      // If a modal dialog is active, prevent background scrollers from scrolling
      const activeModal = document.querySelector(".z-50");
      if (activeModal && !activeModal.contains(container)) {
        return;
      }

      const step = 48 * speed;

      if (e.key === "ArrowDown") {
        if (
          container.scrollTop + container.clientHeight <
          container.scrollHeight
        ) {
          e.preventDefault();
          container.scrollTop += step;
        }
      } else if (e.key === "ArrowUp") {
        if (container.scrollTop > 0) {
          e.preventDefault();
          container.scrollTop -= step;
        }
      } else if (e.key === "PageDown" || (e.key === " " && !e.shiftKey)) {
        e.preventDefault();
        container.scrollTop += container.clientHeight * 0.8;
      } else if (e.key === "PageUp" || (e.key === " " && e.shiftKey)) {
        e.preventDefault();
        container.scrollTop -= container.clientHeight * 0.8;
      }
    };

    container.addEventListener("wheel", handleWheel, { passive: false });
    window.addEventListener("keydown", handleKeyDown);

    return () => {
      container.removeEventListener("wheel", handleWheel);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [container, speed, enabled]);
}
