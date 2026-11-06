import React, { useEffect, useState, useCallback } from "react";
import { usePlatform, isTauri, UnsubscribeFn } from "./platform";
import {
  ServerCatalogItem,
  ProgressPayload,
  InstanceLifecycleStatus,
  normalizeStatus,
} from "./types/launcher";
import { AccountProfile } from "./types/auth";
import {
  UserSettings,
  ServerStatusPayload,
  DEFAULT_USER_SETTINGS,
} from "./types/settings";
import { ServerSidebar } from "./components/ServerSidebar";
import { ServerDetail } from "./components/ServerDetail";
import { LogDrawer, LogEntry } from "./components/LogDrawer";
import { LoginModal } from "./components/auth/LoginModal";
import { SettingsModal } from "./components/SettingsModal";
import { LauncherLayout } from "./components/LauncherLayout";
import { WebHeader } from "./components/WebHeader";
import { LauncherInstallModal } from "./components/LauncherInstallModal";
import { useSmartDeepLink } from "./hooks/useSmartDeepLink";
import { useAppRouter } from "./hooks/useAppRouter";
import { DownloadLandingPage } from "./pages/DownloadLandingPage";
import { StudioPreview } from "./pages/StudioPreview";
import { AlertCircle } from "lucide-react";
import { useTheme } from "./hooks/useTheme";

const MAX_LOG_ENTRIES = 500;

// Helper to construct normalized status dictionary from catalog items
const buildStatusMap = (catalog: ServerCatalogItem[]): Record<string, InstanceLifecycleStatus> => {
  const map: Record<string, InstanceLifecycleStatus> = {};
  for (const item of catalog) {
    map[item.id] = normalizeStatus(item.status);
  }
  return map;
};

export const App: React.FC = () => {
  const adapter = usePlatform();
  const { theme, setTheme, toggleTheme } = useTheme();
  const [servers, setServers] = useState<ServerCatalogItem[]>([]);
  const [selectedServerId, setSelectedServerId] = useState<string>("");
  const [statusMap, setStatusMap] = useState<Record<string, InstanceLifecycleStatus>>({});
  const [isOperating, setIsOperating] = useState<boolean>(false);
  const [progress, setProgress] = useState<ProgressPayload | null>(null);
  const [dryRun, setDryRun] = useState<boolean>(true);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [lastCommand, setLastCommand] = useState<string | null>(null);
  const [errorBanner, setErrorBanner] = useState<string | null>(null);

  // Authentication & Settings State
  const [accounts, setAccounts] = useState<AccountProfile[]>([]);
  const activeAccount = accounts.find((a) => a.is_active) ?? null;
  const [isDropdownOpen, setIsDropdownOpen] = useState<boolean>(false);
  const [isLoginModalOpen, setIsLoginModalOpen] = useState<boolean>(false);
  const [isSettingsModalOpen, setIsSettingsModalOpen] = useState<boolean>(false);
  const [settings, setSettings] = useState<UserSettings>(DEFAULT_USER_SETTINGS);

  // Live SLP Ping State
  const [serverStatuses, setServerStatuses] = useState<Record<string, ServerStatusPayload>>({});
  const [pingingServerId, setPingingServerId] = useState<string | null>(null);
  const [isRefreshingCatalog, setIsRefreshingCatalog] = useState<boolean>(false);

  const applyUserSettings = useCallback(
    (next: UserSettings) => {
      setSettings(next);
      setTheme(next.launcher.theme);
    },
    [setTheme]
  );

  const addLog = (level: LogEntry["level"], text: string) => {
    const timeStr = new Date().toLocaleTimeString("en-US", {
      hour12: false,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
    setLogs((prev) => {
      const next: LogEntry[] = [
        ...prev,
        {
          id: crypto.randomUUID(),
          timestamp: timeStr,
          level,
          text,
        },
      ];
      return next.length > MAX_LOG_ENTRIES ? next.slice(-MAX_LOG_ENTRIES) : next;
    });
  };

  const isWeb = !isTauri();
  const { view, serverId: routeServerId, navigate, navigateToServer } = useAppRouter();

  const {
    triggerPlay,
    isPending: isDeepLinkPending,
    isFallbackOpen: isInstallModalOpen,
    closeFallback: closeInstallModal,
  } = useSmartDeepLink({
    timeoutMs: 2000,
    onFallback: (serverId) => {
      addLog("info", `[Web] Launcher not detected for '${serverId}'. Displaying install guide modal.`);
    },
    onSuccess: (serverId) => {
      addLog("success", `[Web] szz launcher launched successfully for '${serverId}'!`);
    },
  });

  useEffect(() => {
    let unlisten: UnsubscribeFn | undefined;
    let unlistenStatus: UnsubscribeFn | undefined;
    let unlistenDeepLink: UnsubscribeFn | undefined;
    let isCancelled = false;

    // Attach event listeners immediately
    const attachListeners = async () => {
      try {
        const unsub = await adapter.onLaunchProgress((payload) => {
          if (isCancelled) return;
          setProgress(payload);
          addLog("step", `[${payload.stepNumber}/${payload.totalSteps}] ${payload.message}`);

          if (payload.step === "complete") {
            setIsOperating(false);
          }
        });

        if (isCancelled) {
          unsub();
        } else {
          unlisten = unsub;
        }

        const unsubStatus = await adapter.onInstanceStatusChanged(
          ({ instanceId, status: newStatus }) => {
            console.log("[szz-ui] instance-status-changed event received:", instanceId, newStatus);
            if (isCancelled) return;
            const norm = normalizeStatus(newStatus);
            setStatusMap((prev) => {
              if (prev[instanceId] === "Running" && norm === "Installed") {
                addLog("info", `Game client '${instanceId}' has exited. Status restored to Ready.`);
              }
              return prev[instanceId] === norm ? prev : { ...prev, [instanceId]: norm };
            });
          }
        );

        if (isCancelled) {
          unsubStatus();
        } else {
          unlistenStatus = unsubStatus;
        }

        const unsubDeepLink = await adapter.onDeepLink((targetServerId) => {
          console.log("[szz-ui] deep-link-received event:", targetServerId);
          if (isCancelled) return;
          setSelectedServerId(targetServerId);
          navigate("catalog");
          addLog("info", `[DeepLink] Activated server instance '${targetServerId}'`);
        });

        if (isCancelled) {
          unsubDeepLink();
        } else {
          unlistenDeepLink = unsubDeepLink;
        }
      } catch (err) {
        console.warn("[szz-ui] Could not attach progress or status listeners:", err);
      }
    };

    const setupApp = async () => {
      try {
        addLog("info", "Initializing szz launcher platform connection...");

        const [catalog, authState, loadedSettings] = await Promise.all([
          adapter.getCatalog().catch((catErr) => {
            console.warn("Could not load catalog:", catErr);
            return [];
          }),
          adapter.getAuthState().catch((authErr) => {
            console.warn("Could not load auth state:", authErr);
            return null;
          }),
          adapter.getSettings().catch((settingsErr) => {
            console.warn("Could not load user settings, using defaults:", settingsErr);
            return null;
          }),
        ]);

        if (isCancelled) return;

        if (loadedSettings) {
          applyUserSettings(loadedSettings);
        }

        setServers(catalog);
        if (catalog.length > 0) {
          if (routeServerId && catalog.some((s) => s.id === routeServerId)) {
            setSelectedServerId(routeServerId);
          } else {
            setSelectedServerId(catalog[0].id);
          }
        }

        // Initialize status map for all catalog items
        setStatusMap(buildStatusMap(catalog));
        addLog("success", `Loaded server catalog (${catalog.length} servers configured)`);

        if (authState) {
          setAccounts(authState.accounts);
          const active = authState.accounts.find((a) => a.is_active);
          if (active) {
            addLog("info", `Active session: ${active.username} (${active.account_type.toUpperCase()})`);
          }
        }
      } catch (err) {
        if (isCancelled) return;
        const errMsg = String(err);
        setErrorBanner(`Failed to load server catalog: ${errMsg}`);
        addLog("error", `Initialization error: ${errMsg}`);
      }
    };

    attachListeners();
    setupApp();

    return () => {
      isCancelled = true;
      if (unlisten) {
        unlisten();
      }
      if (unlistenStatus) {
        unlistenStatus();
      }
      if (unlistenDeepLink) {
        unlistenDeepLink();
      }
    };
  }, []);

  // Safety net: Real-time polling fallback when any instance is in Running state
  useEffect(() => {
    const runningInstanceIds = Object.entries(statusMap)
      .filter(([_, s]) => s === "Running")
      .map(([id]) => id);

    if (runningInstanceIds.length === 0) return;

    const interval = setInterval(async () => {
      await Promise.all(
        runningInstanceIds.map(async (instanceId) => {
          try {
            const rawStatus = await adapter.getInstanceStatus(instanceId);
            const norm = normalizeStatus(rawStatus);
            if (norm !== "Running") {
              console.log(`[szz-ui] Poller detected instance '${instanceId}' exited (status: ${norm})`);
              setStatusMap((prev) => (prev[instanceId] === norm ? prev : { ...prev, [instanceId]: norm }));
              addLog("info", `Game client '${instanceId}' exit detected. Status restored to Ready.`);
            }
          } catch (err) {
            console.warn(`[szz-ui] Status polling error for '${instanceId}':`, err);
          }
        })
      );
    }, 1000);

    return () => clearInterval(interval);
  }, [statusMap]);

  const handleSwitchAccount = async (accountId: string) => {
    if (activeAccount?.id === accountId) {
      setIsDropdownOpen(false);
      return;
    }

    try {
      await adapter.switchAccount(accountId);
      setAccounts((prev) =>
        prev.map((a) => ({
          ...a,
          is_active: a.id === accountId,
        }))
      );
      setIsDropdownOpen(false);
      const switched = accounts.find((a) => a.id === accountId);
      if (switched) {
        addLog("info", `Switched active account to '${switched.username}' (${switched.account_type.toUpperCase()})`);
      }
    } catch (err) {
      const errMsg = String(err);
      setErrorBanner(`Failed to switch account: ${errMsg}`);
      addLog("error", `Failed to switch account: ${errMsg}`);
    }
  };

  const handleRemoveAccount = async (accountId: string) => {
    try {
      await adapter.removeAccount(accountId);
      const authState = await adapter.getAuthState();
      setAccounts(authState.accounts);
      addLog("info", "Removed account.");
    } catch (err) {
      const errMsg = String(err);
      setErrorBanner(`Failed to remove account: ${errMsg}`);
      addLog("error", `Failed to remove account: ${errMsg}`);
    }
  };

  const handleCreateDevAccount = async (username: string) => {
    try {
      const newProfile = await adapter.createDevAccount(username);
      const authState = await adapter.getAuthState();
      setAccounts(authState.accounts);
      setIsDropdownOpen(false);
      addLog("success", `Created and selected Dev account '${newProfile.username}'`);
    } catch (err) {
      const errMsg = String(err);
      setErrorBanner(`Failed to create dev account: ${errMsg}`);
      addLog("error", `Failed to create dev account: ${errMsg}`);
    }
  };

  const handleLoginSuccess = async (profile: AccountProfile) => {
    try {
      const authState = await adapter.getAuthState();
      setAccounts(authState.accounts);
      addLog("success", `Successfully authenticated Microsoft account: ${profile.username}`);
    } catch (err) {
      console.warn("Error refreshing auth state after login:", err);
    }
  };

  const selectedServer =
    servers.find((s) => s.id === selectedServerId) ?? servers[0];
  const currentStatus = (selectedServer ? statusMap[selectedServer.id] : null) ?? "NotInstalled";

  // Ping active server status via SLP
  const handlePingServer = useCallback(async (serverItem?: ServerCatalogItem) => {
    const target = serverItem ?? selectedServer;
    if (!target || !target.manifest?.serverAddress) return;

    setPingingServerId(target.id);
    try {
      const statusPayload = await adapter.pingServer(target.manifest.serverAddress);
      setServerStatuses((prev) => ({ ...prev, [target.id]: statusPayload }));
    } catch (err) {
      console.warn(`[szz-ui] SLP ping error for '${target.id}':`, err);
    } finally {
      setPingingServerId((prev) => (prev === target.id ? null : prev));
    }
  }, [selectedServer]);

  useEffect(() => {
    if (selectedServer) {
      handlePingServer(selectedServer);
    }
  }, [selectedServerId, servers.length]);

  const handleSaveSettings = async (newSettings: UserSettings) => {
    const saved = await adapter.updateSettings(newSettings);
    applyUserSettings(saved);
    addLog("info", `Updated launcher settings (Allocated RAM: ${saved.java.allocatedMemoryMb} MB, Resolution: ${saved.game.resolutionWidth}x${saved.game.resolutionHeight})`);
  };

  const handleResetSettings = async () => {
    const reset = await adapter.resetSettings();
    applyUserSettings(reset);
    addLog("info", "Restored launcher settings to factory defaults.");
  };

  const handleToggleThemeWithSync = async () => {
    const nextTheme = theme === "latte" ? "dark" : "latte";
    toggleTheme();
    try {
      const updated = await adapter.updateSettings({
        ...settings,
        launcher: {
          ...settings.launcher,
          theme: nextTheme,
        },
      });
      applyUserSettings(updated);
    } catch {
      // ignore if backend not reachable in pure browser preview
    }
  };

  // Stage 1: Install instance
  const handleInstall = async () => {
    if (!selectedServer || isOperating) return;

    setErrorBanner(null);
    setIsOperating(true);
    setProgress(null);
    setStatusMap((prev) => ({ ...prev, [selectedServer.id]: "Installing" }));

    addLog("cmd", `Starting pre-install sync pipeline for '${selectedServer.name}' (dryRun=${dryRun})...`);

    try {
      await adapter.installInstance({
        manifestPath: selectedServer.manifestPath,
        instanceId: selectedServer.id,
        dryRun,
      });

      setStatusMap((prev) => ({ ...prev, [selectedServer.id]: "Installed" }));
      setIsOperating(false);
      addLog("success", `Instance '${selectedServer.name}' pre-installed successfully! Ready to play.`);
    } catch (err) {
      const errMsg = String(err);
      setErrorBanner(`Installation failed: ${errMsg}`);
      addLog("error", `Install error: ${errMsg}`);
      setStatusMap((prev) => ({ ...prev, [selectedServer.id]: "NotInstalled" }));
      setProgress(null);
      setIsOperating(false);
    }
  };

  // Stage 2: Launch installed instance
  const handleLaunch = async () => {
    if (!selectedServer || isOperating) return;

    if (
      statusMap[selectedServer.id] === "UpdateAvailable" ||
      statusMap[selectedServer.id] === "Updating"
    ) {
      setErrorBanner("최신 버전 매니페스트가 존재합니다. 게임을 실행하기 전에 먼저 업데이트를 완료해주세요.");
      return;
    }

    setErrorBanner(null);
    setIsOperating(true);
    setProgress(null);
    setStatusMap((prev) => ({ ...prev, [selectedServer.id]: "Running" }));

    addLog("cmd", `Launching instance '${selectedServer.id}' (dryRun=${dryRun})...`);

    try {
      const result = await adapter.launchInstance({
        instanceId: selectedServer.id,
        dryRun,
      });

      setLastCommand(result.commandLine);
      if (dryRun) {
        addLog("success", "Dry-run verification completed successfully! Game launch command generated.");
        setStatusMap((prev) => ({ ...prev, [selectedServer.id]: "Installed" }));
      } else {
        addLog("success", `Game client launched successfully! (PID: ${result.pid ?? "Unknown"})`);
        setStatusMap((prev) => ({ ...prev, [selectedServer.id]: "Running" }));
      }
      setIsOperating(false);
    } catch (err) {
      const errMsg = String(err);
      setErrorBanner(`Launch failed: ${errMsg}`);
      addLog("error", `Launch error: ${errMsg}`);
      setStatusMap((prev) => ({ ...prev, [selectedServer.id]: "Installed" }));
      setProgress(null);
      setIsOperating(false);
    }
  };

  // Stage 3: Incremental Update
  const handleUpdate = async () => {
    if (!selectedServer || isOperating) return;
    if (!selectedServer.manifest) {
      setErrorBanner(`Cannot update '${selectedServer.name}': Remote manifest is missing.`);
      return;
    }

    setErrorBanner(null);
    setIsOperating(true);
    setProgress(null);
    setStatusMap((prev) => ({ ...prev, [selectedServer.id]: "Updating" }));

    addLog(
      "cmd",
      `Starting incremental update for '${selectedServer.name}' to manifest v${
        selectedServer.latestManifestVersion ?? selectedServer.manifest.manifestVersion
      }...`
    );

    try {
      await adapter.updateInstance({
        manifest: selectedServer.manifest,
        instanceId: selectedServer.id,
      });

      setStatusMap((prev) => ({ ...prev, [selectedServer.id]: "Installed" }));
      setIsOperating(false);
      addLog(
        "success",
        `Instance '${selectedServer.name}' incrementally updated successfully! Ready to play.`
      );
      await handleRefreshCatalog();
    } catch (err) {
      const errMsg = String(err);
      setErrorBanner(`Incremental update failed: ${errMsg}`);
      addLog("error", `Update error: ${errMsg}`);
      setStatusMap((prev) => ({ ...prev, [selectedServer.id]: "UpdateAvailable" }));
      setProgress(null);
      setIsOperating(false);
    }
  };

  // Web custom protocol play trigger with smart fallback detection
  const handlePlay = async () => {
    if (!selectedServer) return;
    addLog("info", `[Web] Triggering szz:// protocol for '${selectedServer.name}'...`);
    try {
      await triggerPlay(selectedServer.id);
    } catch (err) {
      const errMsg = String(err);
      setErrorBanner(`Play failed: ${errMsg}`);
      addLog("error", `Play error: ${errMsg}`);
    }
  };

  // Refresh Catalog
  const handleRefreshCatalog = async () => {
    setIsRefreshingCatalog(true);
    addLog("cmd", "Refreshing server catalog from remote endpoint...");
    try {
      const catalog = await adapter.getCatalog();
      setServers(catalog);

      const initialStatusMap = buildStatusMap(catalog);
      setStatusMap((prev) => {
        const hasChanged = catalog.some((item) => prev[item.id] !== initialStatusMap[item.id]);
        return hasChanged ? initialStatusMap : prev;
      });
      addLog("success", `Server catalog refreshed (${catalog.length} servers found).`);
    } catch (err) {
      const errMsg = String(err);
      setErrorBanner(`Failed to refresh catalog: ${errMsg}`);
      addLog("error", `Catalog refresh error: ${errMsg}`);
    } finally {
      setIsRefreshingCatalog(false);
    }
  };

  // Uninstall instance
  const handleUninstall = async () => {
    if (!selectedServer || isOperating) return;

    const confirmed = window.confirm(`'${selectedServer.name}' 인스턴스를 삭제하시겠습니까? (다른 인스턴스에서 사용하지 않는 고아 모드 파일은 자동으로 정리됩니다)`);
    if (!confirmed) return;

    setErrorBanner(null);
    try {
      await adapter.uninstallInstance(selectedServer.id);
      setStatusMap((prev) => ({ ...prev, [selectedServer.id]: "NotInstalled" }));
      addLog("info", `Uninstalled instance '${selectedServer.id}'.`);
    } catch (err) {
      const errMsg = String(err);
      setErrorBanner(`Uninstall failed: ${errMsg}`);
      addLog("error", `Uninstall error: ${errMsg}`);
    }
  };

  // Sync selectedServerId when browser history navigation (back/forward) changes routeServerId
  useEffect(() => {
    if (routeServerId && routeServerId !== selectedServerId && servers.some((s) => s.id === routeServerId)) {
      setSelectedServerId(routeServerId);
    }
  }, [routeServerId, servers, selectedServerId]);

  const runningInstanceEntry = Object.entries(statusMap).find(([_, s]) => s === "Running");
  const isAnyInstanceRunning = Boolean(runningInstanceEntry);
  const runningInstanceId = runningInstanceEntry ? runningInstanceEntry[0] : null;

  const renderMainContent = () => {
    if (view === "download") {
      return <DownloadLandingPage onBack={() => navigate("catalog")} />;
    }
    if (view === "studio") {
      return <StudioPreview onBack={() => navigate("catalog")} />;
    }
    if (selectedServer) {
      return (
        <ServerDetail
          server={selectedServer}
          status={currentStatus}
          progress={progress}
          isOperating={isOperating || isDeepLinkPending}
          onInstall={handleInstall}
          onLaunch={handleLaunch}
          onUninstall={handleUninstall}
          onUpdate={handleUpdate}
          onPlay={handlePlay}
          dryRun={dryRun}
          onToggleDryRun={setDryRun}
          serverStatus={serverStatuses[selectedServer.id] ?? null}
          isPinging={pingingServerId === selectedServer.id}
          onRefreshPing={() => handlePingServer(selectedServer)}
        />
      );
    }
    return (
      <div className="flex-1 flex items-center justify-center text-szz-subtext-1 text-sm font-pixel">
        서버를 불러오는 중입니다...
      </div>
    );
  };

  return (
    <LauncherLayout
      isWeb={isWeb}
      header={
        <WebHeader
          currentView={view}
          onNavigate={navigate}
          theme={theme}
          onToggleTheme={handleToggleThemeWithSync}
        />
      }
      sidebar={
        <ServerSidebar
          servers={servers}
          selectedServerId={selectedServerId}
          onSelectServer={(s) => {
            setSelectedServerId(s.id);
            if (view !== "catalog") {
              navigate("catalog", s.id);
            } else if (isWeb) {
              navigateToServer(s.id);
            }
          }}
          isLaunching={isOperating || isDeepLinkPending}
          statusMap={statusMap}
          activeAccount={activeAccount}
          accounts={accounts}
          isDropdownOpen={isDropdownOpen}
          onToggleDropdown={() => setIsDropdownOpen((prev) => !prev)}
          onSwitchAccount={handleSwitchAccount}
          onRemoveAccount={handleRemoveAccount}
          onCreateDevAccount={handleCreateDevAccount}
          onOpenLoginModal={() => {
            setIsDropdownOpen(false);
            setIsLoginModalOpen(true);
          }}
          onCloseDropdown={() => setIsDropdownOpen(false)}
          theme={theme}
          onToggleTheme={handleToggleThemeWithSync}
          onOpenSettingsModal={() => setIsSettingsModalOpen(true)}
          onRefreshCatalog={handleRefreshCatalog}
          isRefreshingCatalog={isRefreshingCatalog}
        />
      }
      errorBanner={
        errorBanner ? (
          <div className="bg-szz-berry/90 border-b border-szz-berry text-white px-4 py-2.5 text-xs flex items-center justify-between shrink-0 z-30 shadow-md">
            <div className="flex items-center gap-2">
              <AlertCircle className="w-4 h-4 text-white shrink-0" />
              <span>{errorBanner}</span>
            </div>
            <button
              onClick={() => setErrorBanner(null)}
              className="hover:text-white px-2 py-0.5 rounded hover:bg-black/20 cursor-pointer text-xs"
            >
              닫기
            </button>
          </div>
        ) : null
      }
      drawer={
        <LogDrawer
          logs={logs}
          dryRun={dryRun}
          onToggleDryRun={setDryRun}
          lastCommand={lastCommand}
          onClearLogs={() => setLogs([])}
          isLaunching={isOperating || isDeepLinkPending}
        />
      }
      modals={
        <>
          {isLoginModalOpen && (
            <LoginModal
              isOpen={isLoginModalOpen}
              onClose={() => setIsLoginModalOpen(false)}
              onLoginSuccess={handleLoginSuccess}
            />
          )}
          {isSettingsModalOpen && (
            <SettingsModal
              isOpen={isSettingsModalOpen}
              onClose={() => setIsSettingsModalOpen(false)}
              settings={settings}
              onSaveSettings={handleSaveSettings}
              onResetSettings={handleResetSettings}
              isAnyInstanceRunning={isAnyInstanceRunning || isOperating}
              runningInstanceId={runningInstanceId}
            />
          )}
          <LauncherInstallModal
            isOpen={isInstallModalOpen}
            serverName={selectedServer?.name}
            minecraftVersion={selectedServer?.manifest?.minecraftVersion}
            loaderType={selectedServer?.manifest?.loader?.type}
            onClose={closeInstallModal}
            onDownload={() => {
              closeInstallModal();
              navigate("download");
            }}
            onRetry={() => {
              if (selectedServer) {
                handlePlay();
              }
            }}
            isRetrying={isDeepLinkPending}
          />
        </>
      }
    >
      {renderMainContent()}
    </LauncherLayout>
  );
};
