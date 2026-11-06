import React from "react";
import {
  ServerCatalogItem,
  InstanceLifecycleStatus,
  isInstanceInstalled,
  isInstanceRunning,
  isInstanceUpdateAvailable,
} from "../types/launcher";
import { AccountProfile } from "../types/auth";
import { AccountHeader } from "./auth/AccountHeader";
import { AccountDropdown } from "./auth/AccountDropdown";
import { Sparkles, Layers, Box, Cpu, Sun, Moon, Settings, RefreshCw } from "lucide-react";
import { ThemeMode } from "../types/theme";

interface ServerSidebarProps {
  servers: ServerCatalogItem[];
  selectedServerId: string;
  onSelectServer: (server: ServerCatalogItem) => void;
  isLaunching: boolean;
  statusMap: Record<string, InstanceLifecycleStatus>;
  activeAccount: AccountProfile | null;
  accounts: AccountProfile[];
  isDropdownOpen: boolean;
  onToggleDropdown: () => void;
  onSwitchAccount: (id: string) => void;
  onRemoveAccount: (id: string) => void;
  onCreateDevAccount: (username: string) => void;
  onOpenLoginModal: () => void;
  onCloseDropdown: () => void;
  theme?: ThemeMode;
  onToggleTheme?: () => void;
  onOpenSettingsModal?: () => void;
  onRefreshCatalog?: () => void;
  isRefreshingCatalog?: boolean;
}

const getServerIcon = (serverId: string) => {
  switch (serverId) {
    case "cobblemon-1-21-1":
      return <Sparkles className="w-5 h-5 text-szz-amber" />;
    case "hypixel-26-2":
    case "hypixel-1-8-9":
      return <Layers className="w-5 h-5 text-szz-matcha" />;
    case "skyfactory-4":
    case "skyfactory4-1-12-2":
      return <Box className="w-5 h-5 text-szz-bamboo" />;
    default:
      return <Cpu className="w-5 h-5 text-szz-matcha" />;
  }
};

const getInstanceStatusBadge = (
  isRunning: boolean,
  isInstalled: boolean,
  isUpdateAvailable: boolean
): { label: string; className: string } => {
  if (isRunning) return { label: "실행중", className: "text-szz-matcha font-semibold" };
  if (isUpdateAvailable) return { label: "업데이트 가능", className: "text-szz-amber font-bold" };
  if (isInstalled) return { label: "설치됨", className: "text-szz-matcha font-semibold" };
  return { label: "미설치", className: "text-szz-subtext-0 font-semibold" };
};

export const ServerSidebar: React.FC<ServerSidebarProps> = ({
  servers,
  selectedServerId,
  onSelectServer,
  isLaunching,
  statusMap,
  activeAccount,
  accounts,
  isDropdownOpen,
  onToggleDropdown,
  onSwitchAccount,
  onRemoveAccount,
  onCreateDevAccount,
  onOpenLoginModal,
  onCloseDropdown,
  theme = "latte",
  onToggleTheme,
  onOpenSettingsModal,
  onRefreshCatalog,
  isRefreshingCatalog = false,
}) => {
  const isOfflineMode = servers.some((s) => s.isOfflineCache);

  return (
    <aside className="fixed left-0 top-0 bottom-0 z-40 w-[72px] hover:w-[260px] bg-szz-crust border-r border-szz-border transition-[width] duration-300 ease-out group flex flex-col px-2 py-3 select-none transform-gpu [backface-visibility:hidden] will-change-[width]">
      <div className="flex flex-col h-full justify-between">
        {/* Top: Logo & Theme Switcher */}
        <div className="flex flex-col gap-1 shrink-0">
          <div className="flex items-center gap-2 px-2 h-12">
            <div className="w-10 h-10 flex items-center justify-center shrink-0 font-bold text-xl text-szz-text font-pixel tracking-wider">
              szz
            </div>
            <span className="text-[10px] font-pixel text-szz-matcha px-1.5 py-0.5 rounded bg-szz-surface-0 border border-szz-border opacity-0 max-w-0 overflow-hidden group-hover:opacity-100 group-hover:max-w-xs transition-all duration-300 whitespace-nowrap">
              LAUNCHER
            </span>
            {onToggleTheme && (
              <button
                onClick={onToggleTheme}
                className="ml-auto p-1.5 rounded-lg hover:bg-szz-surface-1 text-szz-subtext-0 hover:text-szz-text transition-all opacity-0 max-w-0 overflow-hidden group-hover:opacity-100 group-hover:max-w-xs cursor-pointer"
                title={theme === "dark" ? "Switch to Matcha Latte (Light)" : "Switch to Matcha Dark"}
              >
                {theme === "dark" ? (
                  <Sun className="w-4 h-4 text-szz-amber" />
                ) : (
                  <Moon className="w-4 h-4 text-szz-matcha" />
                )}
              </button>
            )}
          </div>

          {/* Offline Cache Mode Badge */}
          {isOfflineMode && (
            <div
              className="mx-1 px-2 py-1 rounded bg-szz-amber/15 border border-szz-amber/30 text-szz-amber flex items-center gap-2"
              title="원격 카탈로그 연결 불가로 로컬 오프라인 캐시를 사용 중입니다."
            >
              <span className="w-2 h-2 rounded-full bg-szz-amber animate-pulse shrink-0" />
              <span className="text-[10px] font-pixel font-bold opacity-0 max-w-0 overflow-hidden group-hover:opacity-100 group-hover:max-w-xs transition-all duration-300 whitespace-nowrap">
                오프라인 캐시 모드
              </span>
            </div>
          )}
        </div>

        {/* Middle: Server Navigation (Vertically Centered) */}
        <nav className="flex-1 flex flex-col justify-center gap-2.5 overflow-y-auto overflow-x-hidden py-4">
          <div className="px-2 flex items-center justify-between">
            <span className="text-[10px] font-bold uppercase tracking-wider text-szz-subtext-1 font-pixel opacity-0 max-w-0 overflow-hidden group-hover:opacity-100 group-hover:max-w-xs transition-all duration-300 whitespace-nowrap">
              Servers ({servers.length})
            </span>
            {onRefreshCatalog && (
              <button
                onClick={onRefreshCatalog}
                disabled={isRefreshingCatalog || isLaunching}
                className="p-1 rounded hover:bg-szz-surface-1 text-szz-subtext-0 hover:text-szz-text transition-colors opacity-0 max-w-0 overflow-hidden group-hover:opacity-100 group-hover:max-w-xs cursor-pointer disabled:opacity-40"
                title="카탈로그 새로고침"
              >
                <RefreshCw
                  className={`w-3.5 h-3.5 ${
                    isRefreshingCatalog ? "animate-spin text-szz-amber" : ""
                  }`}
                />
              </button>
            )}
          </div>

          {servers.map((server) => {
            const isSelected = server.id === selectedServerId;
            const status = statusMap[server.id] ?? server.status ?? "NotInstalled";
            const isInstalled = isInstanceInstalled(status);
            const isRunning = isInstanceRunning(status);
            const isUpdateAvailable = isInstanceUpdateAvailable(status);
            const mcVersion = server.manifest?.minecraftVersion ?? "1.20.1";
            const loader = server.manifest?.loader.type ?? "fabric";

            return (
              <button
                key={server.id}
                onClick={() => onSelectServer(server)}
                disabled={isLaunching && !isSelected}
                className={`w-full flex items-center gap-3 px-2 py-1.5 rounded-xl border border-transparent transition-colors text-left group/item relative cursor-pointer hover:bg-szz-surface-1 ${
                  isSelected
                    ? "text-szz-text bg-szz-surface-0/70 border-szz-border/40 font-bold"
                    : "text-szz-subtext-1"
                } ${isLaunching && !isSelected ? "opacity-40 cursor-not-allowed" : ""}`}
                title={server.name}
              >
                {/* Active Indicator Bar */}
                {isSelected && (
                  <div className="absolute left-0 top-2 bottom-2 w-1 bg-szz-matcha rounded-r shadow-sm" />
                )}

                {/* Server Icon with Update Dot */}
                <div className="w-10 h-10 rounded-lg flex items-center justify-center shrink-0 bg-szz-surface-2 relative">
                  {getServerIcon(server.id)}
                  {isUpdateAvailable && (
                    <span
                      className="absolute top-1 right-1 w-2.5 h-2.5 bg-szz-amber rounded-full ring-2 ring-szz-crust animate-pulse"
                      title="업데이트 가능"
                    />
                  )}
                </div>

                {/* Hover Reveal: Metadata */}
                <div className="flex flex-col min-w-0 opacity-0 max-w-0 overflow-hidden group-hover:opacity-100 group-hover:max-w-xs transition-all duration-300 whitespace-nowrap">
                  <span className="text-xs font-bold truncate block text-szz-text font-pixel">
                    {server.name}
                  </span>
                  <div className="flex items-center gap-1.5 text-[10px] text-szz-subtext-0 mt-0.5">
                    <span className="capitalize">{loader}</span>
                    <span>•</span>
                    <span>{mcVersion}</span>
                    <span>•</span>
                    {(() => {
                      const badge = getInstanceStatusBadge(isRunning, isInstalled, isUpdateAvailable);
                      return <span className={badge.className}>{badge.label}</span>;
                    })()}
                  </div>
                </div>
              </button>
            );
          })}
        </nav>

        {/* Bottom: Global Settings & Account Profile Popover Anchor */}
        <div className="border-t border-szz-border pt-2 flex flex-col gap-1 relative shrink-0">
          {onOpenSettingsModal && (
            <button
              onClick={onOpenSettingsModal}
              className="w-full flex items-center gap-3 px-2 py-1.5 rounded-xl border border-transparent transition-colors text-left hover:bg-szz-surface-1 text-szz-subtext-1 hover:text-szz-text cursor-pointer"
              title="런처 환경설정"
            >
              <div className="w-10 h-10 rounded-lg flex items-center justify-center shrink-0 bg-szz-surface-2">
                <Settings className="w-5 h-5 text-szz-matcha" />
              </div>
              <div className="flex flex-col min-w-0 opacity-0 max-w-0 overflow-hidden group-hover:opacity-100 group-hover:max-w-xs transition-all duration-300 whitespace-nowrap">
                <span className="text-xs font-bold truncate block text-szz-text font-pixel">
                  환경설정
                </span>
                <span className="text-[10px] text-szz-subtext-0">
                  Java · 해상도 · 저장공간
                </span>
              </div>
            </button>
          )}

          <AccountHeader
            activeAccount={activeAccount}
            isOpen={isDropdownOpen}
            onToggle={onToggleDropdown}
          />

          {isDropdownOpen && (
            <AccountDropdown
              accounts={accounts}
              onSwitchAccount={onSwitchAccount}
              onRemoveAccount={onRemoveAccount}
              onCreateDevAccount={onCreateDevAccount}
              onOpenLoginModal={onOpenLoginModal}
              onClose={onCloseDropdown}
              theme={theme}
              onToggleTheme={onToggleTheme}
            />
          )}
        </div>
      </div>
    </aside>
  );
};
