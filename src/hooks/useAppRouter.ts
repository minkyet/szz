import { useState, useCallback, useEffect } from "react";

export type AppView = "catalog" | "download" | "studio";

export interface ParsedRoute {
  view: AppView;
  serverId?: string;
}

/**
 * Parses pathname into a structured route containing the view and optional serverId.
 * e.g. /servers/cobblemon-1-21-1 -> { view: "catalog", serverId: "cobblemon-1-21-1" }
 */
export function parsePathnameToRoute(pathname: string): ParsedRoute {
  if (pathname.startsWith("/download")) {
    return { view: "download" };
  }
  if (pathname.startsWith("/studio")) {
    return { view: "studio" };
  }
  if (pathname.startsWith("/servers/")) {
    const rawId = pathname.slice("/servers/".length).split("/")[0].trim();
    if (rawId) {
      let decodedId = rawId;
      try {
        decodedId = decodeURIComponent(rawId);
      } catch {
        // Fall back to raw string on malformed URI sequence
      }
      return { view: "catalog", serverId: decodedId };
    }
  }
  return { view: "catalog" };
}

/**
 * Backward-compatible helper returning solely the AppView.
 */
export function parsePathnameToView(pathname: string): AppView {
  return parsePathnameToRoute(pathname).view;
}

/**
 * Formats view and optional serverId into a clean browser URL pathname.
 */
export function viewToPathname(view: AppView, serverId?: string): string {
  if (view === "download") return "/download";
  if (view === "studio") return "/studio";
  if (serverId) return `/servers/${encodeURIComponent(serverId)}`;
  return "/";
}

export interface UseAppRouterReturn {
  view: AppView;
  serverId?: string;
  navigate: (view: AppView, serverId?: string) => void;
  navigateToServer: (serverId: string) => void;
}

/**
 * Lightweight client-side route manager built on HTML5 History API.
 * Keeps desktop bundle footprint minimal while providing clean SPA path routing and server deep links on web.
 */
export function useAppRouter(initialView?: AppView, initialServerId?: string): UseAppRouterReturn {
  const [route, setRoute] = useState<ParsedRoute>(() => {
    if (initialView) {
      return { view: initialView, serverId: initialServerId };
    }
    if (typeof window !== "undefined" && window.location) {
      return parsePathnameToRoute(window.location.pathname);
    }
    return { view: "catalog" };
  });

  const navigate = useCallback((targetView: AppView, targetServerId?: string) => {
    setRoute((prev) => {
      // Guard against redundant re-renders and duplicate history stack pushes
      if (prev.view === targetView && prev.serverId === targetServerId) {
        return prev;
      }
      if (typeof window !== "undefined" && window.history) {
        const path = viewToPathname(targetView, targetServerId);
        window.history.pushState({ view: targetView, serverId: targetServerId }, "", path);
      }
      return { view: targetView, serverId: targetServerId };
    });
  }, []);

  const navigateToServer = useCallback(
    (targetServerId: string) => {
      navigate("catalog", targetServerId);
    },
    [navigate]
  );

  useEffect(() => {
    if (typeof window === "undefined") return;

    const handlePopState = (event: PopStateEvent) => {
      const state = event.state as ParsedRoute | undefined;
      if (state && (state.view === "catalog" || state.view === "download" || state.view === "studio")) {
        setRoute({ view: state.view, serverId: state.serverId });
      } else if (window.location) {
        setRoute(parsePathnameToRoute(window.location.pathname));
      }
    };

    window.addEventListener("popstate", handlePopState);
    return () => window.removeEventListener("popstate", handlePopState);
  }, []);

  return {
    view: route.view,
    serverId: route.serverId,
    navigate,
    navigateToServer,
  };
}
