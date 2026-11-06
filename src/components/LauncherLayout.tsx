import React from "react";

export interface LauncherLayoutProps {
  isWeb: boolean;
  header?: React.ReactNode;
  sidebar: React.ReactNode;
  errorBanner?: React.ReactNode;
  children: React.ReactNode;
  drawer?: React.ReactNode;
  modals?: React.ReactNode;
}

/**
 * Unified layout shell used by both Desktop (Tauri) and Web (Browser) environments.
 * Manages the floating sidebar, responsive header, error banner, active view content,
 * log drawer, and modal containers.
 */
export const LauncherLayout: React.FC<LauncherLayoutProps> = ({
  isWeb,
  header,
  sidebar,
  errorBanner,
  children,
  drawer,
  modals,
}) => {
  return (
    <div className="flex h-screen w-screen overflow-hidden bg-szz-base text-szz-text font-pixel select-none">
      {/* Floating Hover-Expanding Rail Sidebar */}
      {sidebar}

      {/* Main Content Area (Protected by pl-[72px] padding for floating sidebar) */}
      <main className="flex-1 flex flex-col h-full overflow-hidden pl-[72px] bg-szz-base text-szz-text relative transform-gpu [backface-visibility:hidden] isolate">
        {/* Web Platform Navigation Header */}
        {isWeb && header}

        {/* Global Error Banner */}
        {errorBanner}

        {/* Primary Content View Container */}
        <div className="flex-1 overflow-hidden relative flex flex-col">
          {children}
        </div>

        {/* Log Drawer */}
        {drawer}
      </main>

      {/* Modal Container */}
      {modals}
    </div>
  );
};
