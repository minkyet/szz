import React, { createContext, useContext, useMemo } from "react";
import { PlatformAdapter } from "./types";
import { createPlatformAdapter } from "./index";

const PlatformContext = createContext<PlatformAdapter | null>(null);

export interface PlatformProviderProps {
  adapter?: PlatformAdapter;
  children: React.ReactNode;
}

/**
 * Provides the PlatformAdapter to the React component tree.
 */
export const PlatformProvider: React.FC<PlatformProviderProps> = ({ adapter, children }) => {
  const activeAdapter = useMemo(() => adapter ?? createPlatformAdapter(), [adapter]);

  return (
    <PlatformContext.Provider value={activeAdapter}>
      {children}
    </PlatformContext.Provider>
  );
};

/**
 * Hook to access the active PlatformAdapter.
 */
export const usePlatform = (): PlatformAdapter => {
  const context = useContext(PlatformContext);
  if (!context) {
    throw new Error("usePlatform must be used within a PlatformProvider");
  }
  return context;
};
