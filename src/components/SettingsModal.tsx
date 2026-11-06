import React, { useEffect, useState, useMemo, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useLinearScroll } from "../hooks/useLinearScroll";
import {
  UserSettings,
  SystemMemoryInfo,
  LauncherBehaviorOnLaunch,
} from "../types/settings";
import {
  StorageUsageReport,
  CleanOptions,
  CleanSummary,
  formatBytes,
} from "../types/storage";
import {
  Settings,
  Gamepad2,
  Cpu,
  HardDrive,
  Monitor,
  Info,
  X,
  RotateCcw,
  Save,
  RefreshCw,
  Trash2,
  AlertTriangle,
  CheckCircle2,
  FolderOpen,
  FileCode,
  Box,
  Layers,
  Loader2,
  Sun,
  Moon,
} from "lucide-react";

export type SettingsTab = "game" | "java" | "storage" | "launcher" | "system";

interface SettingsModalProps {
  isOpen: boolean;
  initialTab?: SettingsTab;
  onClose: () => void;
  settings: UserSettings;
  onSaveSettings: (newSettings: UserSettings) => Promise<void>;
  onResetSettings: () => Promise<void>;
  isAnyInstanceRunning?: boolean;
  runningInstanceId?: string | null;
}

const STORAGE_CATEGORIES = [
  {
    key: "mods" as const,
    title: "CAS 모드 저장소",
    path: "(~/.szz/store/)",
    icon: Box,
    iconColor: "text-szz-matcha",
    unit: "개 모드",
    reclaimLabel: "고아 모드",
  },
  {
    key: "runtimes" as const,
    title: "Java JRE 런타임",
    path: "(~/.szz/runtimes/)",
    icon: Cpu,
    iconColor: "text-szz-bamboo",
    unit: "개 버전",
    reclaimLabel: "미사용",
  },
  {
    key: "assets" as const,
    title: "공식 Mojang 에셋",
    path: "(~/.szz/assets/)",
    icon: Layers,
    iconColor: "text-szz-amber",
    unit: "개 오브젝트",
    reclaimLabel: "고아 오브젝트",
  },
  {
    key: "libraries" as const,
    title: "메이븐 라이브러리",
    path: "(~/.szz/libraries/)",
    icon: FileCode,
    iconColor: "text-szz-subtext-0",
    unit: "개 파일",
    reclaimLabel: "정리 가능",
  },
] as const;

const RESOLUTION_PRESETS = [
  { label: "1280×720 (HD)", width: 1280, height: 720 },
  { label: "1600×900 (HD+)", width: 1600, height: 900 },
  { label: "1920×1080 (FHD)", width: 1920, height: 1080 },
  { label: "2560×1440 (QHD)", width: 2560, height: 1440 },
] as const;

let cachedStorageUsage: StorageUsageReport | null = null;
let cachedSystemMemory: SystemMemoryInfo | null = null;

export const SettingsModal: React.FC<SettingsModalProps> = ({
  isOpen,
  initialTab = "game",
  onClose,
  settings,
  onSaveSettings,
  onResetSettings,
  isAnyInstanceRunning = false,
  runningInstanceId = null,
}) => {
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  useLinearScroll(scrollContainerRef, { enabled: isOpen });

  const [activeTab, setActiveTab] = useState<SettingsTab>(initialTab);
  const [draft, setDraft] = useState<UserSettings>(settings);
  const [jvmArgsText, setJvmArgsText] = useState<string>(
    settings.java.customJvmArgs.join(" ")
  );
  const [sysMem, setSysMem] = useState<SystemMemoryInfo>(
    cachedSystemMemory ?? { totalMemoryMb: 16384, availableMemoryMb: 8192 }
  );
  const [isSaving, setIsSaving] = useState<boolean>(false);
  const [toastMsg, setToastMsg] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [unsavedCloseWarning, setUnsavedCloseWarning] = useState<boolean>(false);

  // Storage Tab State
  const [usage, setUsage] = useState<StorageUsageReport | null>(cachedStorageUsage);
  const [loadingStorage, setLoadingStorage] = useState<boolean>(!cachedStorageUsage);
  const [isCleaning, setIsCleaning] = useState<boolean>(false);
  const [confirmCleanOpen, setConfirmCleanOpen] = useState<boolean>(false);
  const [selectedCategories, setSelectedCategories] = useState({
    mods: true,
    runtimes: true,
    assets: true,
    libraries: true,
  });

  // Sync draft when settings prop updates (e.g. after save or reset) without clearing toastMsg
  useEffect(() => {
    setDraft(settings);
    setJvmArgsText(settings.java.customJvmArgs.join(" "));
  }, [settings]);

  // Initialize modal state only when opening
  useEffect(() => {
    if (isOpen) {
      setToastMsg(null);
      setErrorMsg(null);
      setUnsavedCloseWarning(false);
      invoke<SystemMemoryInfo>("get_system_memory_info")
        .then((info) => {
          cachedSystemMemory = info;
          setSysMem(info);
        })
        .catch((err) => {
          console.error("[szz-ui] Failed to fetch system memory info:", err);
        });
    }
  }, [isOpen]);

  const parsedJvmArgs = useMemo(
    () => jvmArgsText.split(/\s+/).filter(Boolean),
    [jvmArgsText]
  );

  const isDirty = useMemo(() => {
    if (
      draft.game.resolutionWidth !== settings.game.resolutionWidth ||
      draft.game.resolutionHeight !== settings.game.resolutionHeight ||
      draft.game.fullscreen !== settings.game.fullscreen ||
      draft.game.autoConnect !== settings.game.autoConnect ||
      draft.java.allocatedMemoryMb !== settings.java.allocatedMemoryMb ||
      draft.launcher.behaviorOnLaunch !== settings.launcher.behaviorOnLaunch ||
      draft.launcher.theme !== settings.launcher.theme
    ) {
      return true;
    }
    if (parsedJvmArgs.length !== settings.java.customJvmArgs.length) {
      return true;
    }
    for (let i = 0; i < parsedJvmArgs.length; i++) {
      if (parsedJvmArgs[i] !== settings.java.customJvmArgs[i]) {
        return true;
      }
    }
    return false;
  }, [draft, parsedJvmArgs, settings]);

  // Clear previous save toast when user starts editing again, and clear close warning once clean
  useEffect(() => {
    if (isDirty) {
      setToastMsg(null);
    } else {
      setUnsavedCloseWarning(false);
    }
  }, [isDirty]);

  const handleAttemptClose = useCallback(() => {
    if (isSaving || isCleaning) return;
    if (isDirty) {
      setUnsavedCloseWarning(true);
      return;
    }
    onClose();
  }, [isSaving, isCleaning, isDirty, onClose]);

  const handleDiscardChanges = useCallback(() => {
    setDraft(settings);
    setJvmArgsText(settings.java.customJvmArgs.join(" "));
    setUnsavedCloseWarning(false);
    setErrorMsg(null);
  }, [settings]);

  useEffect(() => {
    if (isOpen && initialTab) {
      setActiveTab(initialTab);
    }
  }, [isOpen, initialTab]);

  const fetchStorageUsage = useCallback(async () => {
    try {
      if (!cachedStorageUsage) {
        setLoadingStorage(true);
      }
      setErrorMsg(null);
      const res = await invoke<StorageUsageReport>("get_storage_usage");
      cachedStorageUsage = res;
      setUsage(res);
    } catch (err: unknown) {
      console.error("[szz-ui] Failed to load storage usage:", err);
      setErrorMsg(String(err));
    } finally {
      setLoadingStorage(false);
    }
  }, []);

  useEffect(() => {
    if (isOpen && activeTab === "storage") {
      void fetchStorageUsage();
    }
  }, [isOpen, activeTab, fetchStorageUsage]);

  const maxAllowedMemoryMb = useMemo(() => {
    const rawMax = Math.floor((sysMem.totalMemoryMb * 0.9) / 512) * 512;
    return Math.max(2048, Math.min(32768, rawMax));
  }, [sysMem.totalMemoryMb]);

  const memoryRatio = useMemo(() => {
    if (sysMem.totalMemoryMb <= 0) return 0;
    return draft.java.allocatedMemoryMb / sysMem.totalMemoryMb;
  }, [draft.java.allocatedMemoryMb, sysMem.totalMemoryMb]);

  const selectedReclaimableBytes = useMemo(() => {
    if (!usage) return 0;
    let sum = 0;
    if (selectedCategories.mods) sum += usage.mods.reclaimableBytes;
    if (selectedCategories.runtimes) sum += usage.runtimes.reclaimableBytes;
    if (selectedCategories.assets) sum += usage.assets.reclaimableBytes;
    if (selectedCategories.libraries) sum += usage.libraries.reclaimableBytes;
    return sum;
  }, [usage, selectedCategories]);

  const handleToggleCategory = (category: keyof typeof selectedCategories) => {
    setSelectedCategories((prev) => ({
      ...prev,
      [category]: !prev[category],
    }));
  };

  const handleCleanStorage = async () => {
    if (!usage || selectedReclaimableBytes <= 0 || isAnyInstanceRunning || isCleaning) {
      return;
    }

    try {
      setIsCleaning(true);
      setErrorMsg(null);
      setToastMsg(null);
      setConfirmCleanOpen(false);

      const options: CleanOptions = {
        cleanMods: selectedCategories.mods,
        cleanRuntimes: selectedCategories.runtimes,
        cleanAssets: selectedCategories.assets,
        cleanLibraries: selectedCategories.libraries,
        dryRun: false,
      };

      const summary = await invoke<CleanSummary>("clean_storage_cache", {
        options,
      });

      setToastMsg(
        `${formatBytes(summary.reclaimedBytes)}의 디스크 공간이 성공적으로 확보되었습니다.`
      );
      await fetchStorageUsage();
    } catch (err: unknown) {
      console.error("[szz-ui] Failed to clean storage cache:", err);
      setErrorMsg(String(err));
    } finally {
      setIsCleaning(false);
    }
  };

  const handleSave = async () => {
    try {
      setIsSaving(true);
      setErrorMsg(null);
      setUnsavedCloseWarning(false);

      const nextSettings: UserSettings = {
        ...draft,
        java: {
          ...draft.java,
          customJvmArgs: parsedJvmArgs,
        },
      };

      await onSaveSettings(nextSettings);
      setToastMsg("환경설정이 안전하게 저장되었습니다.");
    } catch (err: unknown) {
      setErrorMsg(String(err));
    } finally {
      setIsSaving(false);
    }
  };

  const handleReset = async () => {
    try {
      setIsSaving(true);
      setErrorMsg(null);
      setUnsavedCloseWarning(false);
      await onResetSettings();
      setToastMsg("모든 설정이 기본값으로 복원되었습니다.");
    } catch (err: unknown) {
      setErrorMsg(String(err));
    } finally {
      setIsSaving(false);
    }
  };

  const getMemoryBarColorClass = (): string => {
    if (memoryRatio > 0.8) return "bg-szz-berry";
    if (memoryRatio > 0.65) return "bg-szz-amber";
    return "bg-szz-matcha";
  };

  const getFooterContainerClass = (): string => {
    if (unsavedCloseWarning) return "bg-szz-berry/15 border-szz-berry/60";
    if (isDirty) return "bg-szz-surface-1 border-szz-matcha/40";
    return "bg-szz-mantle border-szz-border";
  };

  if (!isOpen) return null;

  const tabs: { id: SettingsTab; label: string; icon: React.ElementType }[] = [
    { id: "game", label: "게임 (Game)", icon: Gamepad2 },
    { id: "java", label: "자바 & 메모리", icon: Cpu },
    { id: "storage", label: "저장 공간", icon: HardDrive },
    { id: "launcher", label: "런처 동작", icon: Monitor },
    { id: "system", label: "시스템 정보", icon: Info },
  ];

  return (
    <div
      onClick={handleAttemptClose}
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm select-none"
    >
      <div
        className={`w-full max-w-3xl min-h-[600px] max-h-[90vh] bg-szz-base border rounded-2xl shadow-2xl overflow-hidden flex flex-col text-szz-text transition-colors ${
          unsavedCloseWarning ? "border-szz-berry" : "border-szz-border"
        }`}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Modal Header */}
        <div className="px-6 py-4 border-b border-szz-border flex items-center justify-between bg-szz-mantle shrink-0">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-szz-matcha/15 text-szz-matcha border border-szz-matcha/30 flex items-center justify-center shrink-0">
              <Settings className="w-5 h-5" />
            </div>
            <div>
              <h2 className="text-base font-bold font-pixel text-szz-text flex items-center gap-2">
                런처 환경설정
                <span className="text-[10px] font-sans px-2 py-0.5 rounded-full bg-szz-surface-1 text-szz-subtext-0 border border-szz-border">
                  Global Launcher Settings
                </span>
              </h2>
              <p className="text-xs text-szz-subtext-0 mt-0.5">
                모든 마인크래프트 서버에 공통 적용되는 전역 그래픽, 메모리 및 저장 공간 관리
              </p>
            </div>
          </div>
          <button
            onClick={handleAttemptClose}
            disabled={isSaving || isCleaning}
            className="p-1.5 rounded-lg text-szz-subtext-0 hover:text-szz-text hover:bg-szz-surface-0 transition-colors cursor-pointer disabled:opacity-40"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Navigation Tabs */}
        <div className="px-6 pt-3 border-b border-szz-border bg-szz-mantle/50 flex items-center gap-1.5 overflow-x-auto shrink-0">
          {tabs.map((tab) => {
            const Icon = tab.icon;
            const active = activeTab === tab.id;
            return (
              <button
                key={tab.id}
                onClick={() => {
                  setActiveTab(tab.id);
                  setErrorMsg(null);
                }}
                className={`px-3.5 py-2.5 rounded-t-xl font-pixel text-xs font-bold flex items-center gap-2 border-b-2 transition-all cursor-pointer whitespace-nowrap ${
                  active
                    ? "border-szz-matcha text-szz-matcha bg-szz-base"
                    : "border-transparent text-szz-subtext-0 hover:text-szz-text hover:bg-szz-surface-0/50"
                }`}
              >
                <Icon className="w-4 h-4" />
                <span>{tab.label}</span>
              </button>
            );
          })}
        </div>

        {/* Body Content */}
        <div
          ref={scrollContainerRef}
          className="p-6 overflow-y-auto space-y-6 flex-1 min-h-[420px]"
        >
          {unsavedCloseWarning && (
            <div className="p-3.5 rounded-xl bg-szz-berry/15 border border-szz-berry/50 text-szz-berry text-xs flex items-center justify-between gap-2.5 animate-pulse">
              <div className="flex items-center gap-2.5">
                <AlertTriangle className="w-4 h-4 shrink-0" />
                <span className="font-bold">
                  저장하지 않은 변경사항이 있습니다! 하단에서 [설정 저장] 또는 [되돌리기]를 먼저 선택해 주세요.
                </span>
              </div>
            </div>
          )}

          {errorMsg && (
            <div className="p-3.5 rounded-xl bg-szz-berry/10 border border-szz-berry/30 text-szz-berry text-xs flex items-center gap-2.5">
              <AlertTriangle className="w-4 h-4 shrink-0" />
              <span className="font-medium">{errorMsg}</span>
            </div>
          )}

          {toastMsg && (
            <div className="p-3.5 rounded-xl bg-szz-matcha/15 border border-szz-matcha/30 text-szz-matcha text-xs flex items-center justify-between gap-2.5">
              <div className="flex items-center gap-2.5">
                <CheckCircle2 className="w-4 h-4 shrink-0" />
                <span className="font-medium">{toastMsg}</span>
              </div>
              <button
                type="button"
                onClick={() => setToastMsg(null)}
                className="text-szz-matcha/70 hover:text-szz-matcha cursor-pointer"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            </div>
          )}

          {/* TAB 1: GAME */}
          {activeTab === "game" && (
            <div className="space-y-5">
              <div className="p-4 rounded-xl bg-szz-surface-0 border border-szz-border space-y-4">
                <div>
                  <h3 className="text-xs font-bold font-pixel text-szz-text">
                    게임 창 해상도 (Game Window Resolution)
                  </h3>
                  <p className="text-[11px] text-szz-subtext-0 mt-0.5">
                    마인크래프트 창 모드 실행 시 기본 가로 × 세로 픽셀 크기를 지정합니다.
                  </p>
                </div>

                <div className="flex flex-wrap items-center gap-3">
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-szz-subtext-0 font-pixel">가로</span>
                    <input
                      type="number"
                      min={320}
                      max={7680}
                      value={draft.game.resolutionWidth}
                      onChange={(e) =>
                        setDraft((prev) => ({
                          ...prev,
                          game: {
                            ...prev.game,
                            resolutionWidth: Math.max(320, Number(e.target.value) || 1280),
                          },
                        }))
                      }
                      className="w-24 px-3 py-1.5 rounded-lg bg-szz-base border border-szz-border font-mono text-xs text-szz-text focus:outline-none focus:border-szz-matcha"
                    />
                  </div>
                  <span className="text-xs text-szz-subtext-0 font-bold">×</span>
                  <div className="flex items-center gap-2">
                    <span className="text-xs text-szz-subtext-0 font-pixel">세로</span>
                    <input
                      type="number"
                      min={240}
                      max={4320}
                      value={draft.game.resolutionHeight}
                      onChange={(e) =>
                        setDraft((prev) => ({
                          ...prev,
                          game: {
                            ...prev.game,
                            resolutionHeight: Math.max(240, Number(e.target.value) || 720),
                          },
                        }))
                      }
                      className="w-24 px-3 py-1.5 rounded-lg bg-szz-base border border-szz-border font-mono text-xs text-szz-text focus:outline-none focus:border-szz-matcha"
                    />
                  </div>
                </div>

                <div className="flex flex-wrap gap-2 pt-1">
                  {RESOLUTION_PRESETS.map((preset) => {
                    const isSelected =
                      draft.game.resolutionWidth === preset.width &&
                      draft.game.resolutionHeight === preset.height;
                    return (
                      <button
                        key={preset.label}
                        type="button"
                        onClick={() =>
                          setDraft((prev) => ({
                            ...prev,
                            game: {
                              ...prev.game,
                              resolutionWidth: preset.width,
                              resolutionHeight: preset.height,
                            },
                          }))
                        }
                        className={`px-2.5 py-1 rounded-lg text-[11px] font-mono border transition-colors cursor-pointer ${
                          isSelected
                            ? "bg-szz-matcha/15 border-szz-matcha text-szz-matcha font-bold"
                            : "bg-szz-surface-1 border-szz-border text-szz-subtext-0 hover:text-szz-text"
                        }`}
                      >
                        {preset.label}
                      </button>
                    );
                  })}
                </div>
              </div>

              {/* Fullscreen Toggle */}
              <label className="p-4 rounded-xl bg-szz-surface-0 border border-szz-border flex items-center justify-between cursor-pointer hover:border-szz-matcha/40 transition-colors">
                <div>
                  <div className="text-xs font-bold font-pixel text-szz-text">
                    전체화면으로 실행 (Launch in Fullscreen)
                  </div>
                  <div className="text-[11px] text-szz-subtext-0 mt-0.5">
                    게임 기동 시 자동으로 모니터 전체화면 모드(--fullscreen)로 진입합니다.
                  </div>
                </div>
                <input
                  type="checkbox"
                  checked={draft.game.fullscreen}
                  onChange={(e) =>
                    setDraft((prev) => ({
                      ...prev,
                      game: { ...prev.game, fullscreen: e.target.checked },
                    }))
                  }
                  className="w-4 h-4 rounded accent-szz-matcha cursor-pointer"
                />
              </label>

              {/* QuickPlay Auto-Connect Toggle */}
              <label className="p-4 rounded-xl bg-szz-surface-0 border border-szz-border flex items-center justify-between cursor-pointer hover:border-szz-matcha/40 transition-colors">
                <div>
                  <div className="text-xs font-bold font-pixel text-szz-text flex items-center gap-2">
                    서버 자동 직행 (QuickPlay Direct Entry)
                    <span className="text-[10px] px-1.5 py-0.5 rounded bg-szz-matcha/15 text-szz-matcha font-sans">
                      권장
                    </span>
                  </div>
                  <div className="text-[11px] text-szz-subtext-0 mt-0.5">
                    활성화 시 타이틀 메뉴를 건너뛰고 선택한 멀티플레이 서버로 즉시 접속합니다. 리소스팩이나 키 세팅을 먼저 바꾸려면 비활성화하세요.
                  </div>
                </div>
                <input
                  type="checkbox"
                  checked={draft.game.autoConnect}
                  onChange={(e) =>
                    setDraft((prev) => ({
                      ...prev,
                      game: { ...prev.game, autoConnect: e.target.checked },
                    }))
                  }
                  className="w-4 h-4 rounded accent-szz-matcha cursor-pointer"
                />
              </label>
            </div>
          )}

          {/* TAB 2: JAVA & MEMORY */}
          {activeTab === "java" && (
            <div className="space-y-5">
              <div className="p-5 rounded-xl bg-szz-surface-0 border border-szz-border space-y-4">
                <div className="flex items-center justify-between">
                  <div>
                    <h3 className="text-xs font-bold font-pixel text-szz-text">
                      JVM 최대 힙 메모리 할당 (Max RAM / -Xmx)
                    </h3>
                    <p className="text-[11px] text-szz-subtext-0 mt-0.5">
                      대형 모드팩(Cobblemon, SkyFactory 4 등)은 4GB ~ 6GB(4096MB ~ 6144MB) 할당을 권장합니다.
                    </p>
                  </div>
                  <div className="text-right">
                    <span className="text-lg font-mono font-bold text-szz-matcha">
                      {(draft.java.allocatedMemoryMb / 1024).toFixed(1)} GB
                    </span>
                    <span className="block text-[10px] font-mono text-szz-subtext-0">
                      ({draft.java.allocatedMemoryMb} MB)
                    </span>
                  </div>
                </div>

                <input
                  type="range"
                  min={1024}
                  max={maxAllowedMemoryMb}
                  step={512}
                  value={draft.java.allocatedMemoryMb}
                  onChange={(e) =>
                    setDraft((prev) => ({
                      ...prev,
                      java: {
                        ...prev.java,
                        allocatedMemoryMb: Number(e.target.value),
                      },
                    }))
                  }
                  className="w-full accent-szz-matcha cursor-pointer"
                />

                <div className="flex justify-between text-[10px] font-mono text-szz-subtext-0">
                  <span>최소: 1.0 GB (1024 MB)</span>
                  <span>권장: 4.0 GB ~ 6.0 GB</span>
                  <span>최대: {(maxAllowedMemoryMb / 1024).toFixed(1)} GB</span>
                </div>

                {/* System Memory Status Bar */}
                <div className="p-3.5 rounded-lg bg-szz-base border border-szz-border space-y-2">
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-szz-subtext-0 font-pixel">
                      시스템 물리 메모리 대비 할당 비율
                    </span>
                    <span className="font-mono text-szz-text font-semibold">
                      총 {(sysMem.totalMemoryMb / 1024).toFixed(1)} GB 중{" "}
                      {Math.round(memoryRatio * 100)}% 할당 (현재 가용:{" "}
                      {(sysMem.availableMemoryMb / 1024).toFixed(1)} GB)
                    </span>
                  </div>
                  <div className="w-full h-2.5 rounded-full bg-szz-surface-2 overflow-hidden">
                    <div
                      style={{ width: `${Math.min(100, Math.round(memoryRatio * 100))}%` }}
                      className={`h-full transition-all duration-200 ${getMemoryBarColorClass()}`}
                    />
                  </div>
                  {memoryRatio > 0.7 && (
                    <div className="text-[11px] text-szz-amber flex items-center gap-1.5 pt-1">
                      <AlertTriangle className="w-3.5 h-3.5 shrink-0" />
                      <span>
                        시스템 전체 RAM의 70% 이상을 할당하면 운영체제 및 백그라운드 앱이 느려질 수 있습니다.
                      </span>
                    </div>
                  )}
                </div>
              </div>

              {/* Custom JVM Args */}
              <div className="p-4 rounded-xl bg-szz-surface-0 border border-szz-border space-y-2.5">
                <div>
                  <h3 className="text-xs font-bold font-pixel text-szz-text">
                    추가 JVM 시작 인자 (Custom JVM Arguments)
                  </h3>
                  <p className="text-[11px] text-szz-subtext-0 mt-0.5">
                    가비지 컬렉터(G1GC, ZGC) 튜닝 등 고급 사용자를 위한 추가 플래그입니다 (공백으로 구분).
                  </p>
                </div>
                <input
                  type="text"
                  placeholder="예: -XX:+UseG1GC -XX:MaxGCPauseMillis=50"
                  value={jvmArgsText}
                  onChange={(e) => setJvmArgsText(e.target.value)}
                  className="w-full px-3.5 py-2 rounded-lg bg-szz-base border border-szz-border font-mono text-xs text-szz-text focus:outline-none focus:border-szz-matcha"
                />
              </div>
            </div>
          )}

          {/* TAB 3: STORAGE MANAGEMENT */}
          {activeTab === "storage" && (
            <div className="space-y-5">
              {isAnyInstanceRunning && (
                <div className="p-3.5 rounded-xl bg-szz-amber/15 border border-szz-amber/30 text-szz-amber text-xs flex items-center gap-2.5">
                  <AlertTriangle className="w-4 h-4 shrink-0" />
                  <span className="font-medium">
                    게임 클라이언트가 실행 중입니다 (
                    <span className="font-mono">{runningInstanceId ?? "active"}</span>
                    ). 캐시 정리를 위해 게임을 먼저 종료해 주세요.
                  </span>
                </div>
              )}

              {loadingStorage && !usage ? (
                <div className="space-y-4 animate-pulse">
                  <div className="h-24 rounded-xl bg-szz-surface-0 border border-szz-border" />
                  <div className="h-12 rounded-xl bg-szz-surface-0 border border-szz-border" />
                  <div className="h-48 rounded-xl bg-szz-surface-0 border border-szz-border" />
                </div>
              ) : usage ? (
                <>
                  {/* Overall Usage & Gauge */}
                  <div className="p-4 rounded-xl bg-szz-surface-0 border border-szz-border space-y-3">
                    <div className="flex items-center justify-between text-xs">
                      <span className="font-bold text-szz-subtext-1 flex items-center gap-1.5">
                        <FolderOpen className="w-4 h-4 text-szz-matcha" />
                        SZZ 런처 총 디스크 점유량
                      </span>
                      <div className="flex items-center gap-2">
                        <span className="font-mono font-bold text-sm text-szz-text">
                          {formatBytes(usage.totalBytes)}
                        </span>
                        <button
                          type="button"
                          onClick={fetchStorageUsage}
                          disabled={loadingStorage || isCleaning}
                          className="p-1 rounded-lg hover:bg-szz-surface-1 text-szz-subtext-0 hover:text-szz-text transition-colors cursor-pointer"
                          title="용량 다시 계산"
                        >
                          <RefreshCw
                            className={`w-3.5 h-3.5 ${loadingStorage ? "animate-spin" : ""}`}
                          />
                        </button>
                      </div>
                    </div>

                    <div className="w-full h-3 rounded-full bg-szz-surface-2 overflow-hidden flex">
                      {usage.totalBytes > 0 && (
                        <>
                          <div
                            style={{
                              width: `${(usage.mods.totalBytes / usage.totalBytes) * 100}%`,
                            }}
                            className="bg-szz-matcha transition-all duration-300"
                            title={`CAS Mods: ${formatBytes(usage.mods.totalBytes)}`}
                          />
                          <div
                            style={{
                              width: `${(usage.runtimes.totalBytes / usage.totalBytes) * 100}%`,
                            }}
                            className="bg-szz-bamboo transition-all duration-300"
                            title={`Java Runtimes: ${formatBytes(usage.runtimes.totalBytes)}`}
                          />
                          <div
                            style={{
                              width: `${(usage.assets.totalBytes / usage.totalBytes) * 100}%`,
                            }}
                            className="bg-szz-amber transition-all duration-300"
                            title={`Mojang Assets: ${formatBytes(usage.assets.totalBytes)}`}
                          />
                          <div
                            style={{
                              width: `${(usage.libraries.totalBytes / usage.totalBytes) * 100}%`,
                            }}
                            className="bg-szz-subtext-0 transition-all duration-300"
                            title={`Libraries: ${formatBytes(usage.libraries.totalBytes)}`}
                          />
                          <div
                            style={{
                              width: `${(usage.instances.totalBytes / usage.totalBytes) * 100}%`,
                            }}
                            className="bg-szz-surface-2 transition-all duration-300"
                            title={`Instances: ${formatBytes(usage.instances.totalBytes)}`}
                          />
                        </>
                      )}
                    </div>

                    <div className="flex flex-wrap items-center gap-4 text-[11px] text-szz-subtext-0 pt-1">
                      <span className="flex items-center gap-1.5">
                        <span className="w-2.5 h-2.5 rounded-full bg-szz-matcha" />
                        모드 ({formatBytes(usage.mods.totalBytes)})
                      </span>
                      <span className="flex items-center gap-1.5">
                        <span className="w-2.5 h-2.5 rounded-full bg-szz-bamboo" />
                        Java JRE ({formatBytes(usage.runtimes.totalBytes)})
                      </span>
                      <span className="flex items-center gap-1.5">
                        <span className="w-2.5 h-2.5 rounded-full bg-szz-amber" />
                        에셋 ({formatBytes(usage.assets.totalBytes)})
                      </span>
                      <span className="flex items-center gap-1.5">
                        <span className="w-2.5 h-2.5 rounded-full bg-szz-subtext-0" />
                        라이브러리 ({formatBytes(usage.libraries.totalBytes)})
                      </span>
                      <span className="flex items-center gap-1.5">
                        <span className="w-2.5 h-2.5 rounded-full bg-szz-surface-2 border border-szz-border" />
                        인스턴스 ({formatBytes(usage.instances.totalBytes)})
                      </span>
                    </div>
                  </div>

                  {/* Reclaimable Banner */}
                  {usage.reclaimableBytes > 0 ? (
                    <div className="p-3.5 rounded-xl bg-szz-matcha/15 border border-szz-matcha/30 flex items-center justify-between gap-3">
                      <div className="flex items-center gap-2.5">
                        <CheckCircle2 className="w-4 h-4 text-szz-matcha shrink-0" />
                        <span className="text-xs font-medium text-szz-text">
                          현재 어떤 서버에서도 사용하지 않는 리소스{" "}
                          <strong className="text-szz-matcha font-mono">
                            {formatBytes(usage.reclaimableBytes)}
                          </strong>
                          를 안전하게 정리할 수 있습니다.
                        </span>
                      </div>
                      <button
                        type="button"
                        onClick={() => setConfirmCleanOpen(true)}
                        disabled={
                          loadingStorage ||
                          isCleaning ||
                          selectedReclaimableBytes <= 0 ||
                          isAnyInstanceRunning
                        }
                        className="px-3.5 py-1.5 rounded-lg bg-szz-matcha text-szz-chassis text-xs font-bold font-pixel hover:opacity-90 transition-all cursor-pointer disabled:opacity-40 whitespace-nowrap flex items-center gap-1.5"
                      >
                        {isCleaning ? (
                          <Loader2 className="w-3.5 h-3.5 animate-spin" />
                        ) : (
                          <Trash2 className="w-3.5 h-3.5" />
                        )}
                        <span>선택 캐시 비우기 ({formatBytes(selectedReclaimableBytes)})</span>
                      </button>
                    </div>
                  ) : (
                    <div className="p-3.5 rounded-xl bg-szz-surface-0 border border-szz-border flex items-center gap-2.5 text-szz-subtext-0 text-xs">
                      <CheckCircle2 className="w-4 h-4 text-szz-subtext-0 shrink-0" />
                      <span>
                        모든 캐시가 최적화되어 있습니다. 현재 정리 가능한 미사용 리소스가 없습니다 (0 B).
                      </span>
                    </div>
                  )}

                  {/* Category Cards */}
                  <div className="space-y-2">
                    {STORAGE_CATEGORIES.map((cat) => {
                      const isSelected = selectedCategories[cat.key];
                      const data = usage[cat.key];
                      const Icon = cat.icon;
                      return (
                        <label
                          key={cat.key}
                          className={`p-3 rounded-xl border transition-all cursor-pointer flex items-center justify-between select-none ${
                            isSelected
                              ? "bg-szz-surface-0 border-szz-matcha/40 shadow-sm"
                              : "bg-szz-surface-0/40 border-szz-border opacity-70"
                          }`}
                        >
                          <div className="flex items-center gap-3">
                            <input
                              type="checkbox"
                              checked={isSelected}
                              onChange={() => handleToggleCategory(cat.key)}
                              className="w-4 h-4 rounded text-szz-matcha accent-szz-matcha cursor-pointer"
                            />
                            <div
                              className={`w-8 h-8 rounded-lg bg-szz-surface-1 flex items-center justify-center ${cat.iconColor}`}
                            >
                              <Icon className="w-4 h-4" />
                            </div>
                            <div>
                              <div className="text-xs font-bold text-szz-text font-pixel flex items-center gap-2">
                                {cat.title}
                                <span className="text-[10px] font-sans text-szz-subtext-0">
                                  {cat.path}
                                </span>
                              </div>
                              <div className="text-[11px] text-szz-subtext-0 mt-0.5">
                                총 {data.itemCount}
                                {cat.unit} ({formatBytes(data.totalBytes)})
                              </div>
                            </div>
                          </div>
                          <div className="text-right">
                            <span className="text-xs font-mono font-bold text-szz-matcha block">
                              +{formatBytes(data.reclaimableBytes)}
                            </span>
                            <span className="text-[10px] text-szz-subtext-0">
                              {cat.reclaimLabel} {data.reclaimableCount}
                              {cat.key === "runtimes" ? "개 버전" : "개"}
                            </span>
                          </div>
                        </label>
                      );
                    })}
                  </div>
                </>
              ) : null}
            </div>
          )}

          {/* TAB 4: LAUNCHER BEHAVIOR */}
          {activeTab === "launcher" && (
            <div className="space-y-5">
              <div className="p-4 rounded-xl bg-szz-surface-0 border border-szz-border space-y-3">
                <div>
                  <h3 className="text-xs font-bold font-pixel text-szz-text">
                    게임 실행 시 런처 창 동작 (Behavior on Game Launch)
                  </h3>
                  <p className="text-[11px] text-szz-subtext-0 mt-0.5">
                    마인크래프트 프로세스가 시작될 때 szz 데스크톱 런처 창의 상태를 결정합니다.
                  </p>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-3 gap-2.5">
                  {(
                    [
                      {
                        value: "keep_open" as LauncherBehaviorOnLaunch,
                        title: "런처 창 계속 유지",
                        desc: "실시간 로그 및 프로세스 감시를 위해 창을 열어둡니다.",
                      },
                      {
                        value: "minimize" as LauncherBehaviorOnLaunch,
                        title: "작업 표시줄로 최소화",
                        desc: "게임 중에는 최소화하고, 게임 종료 시 자동으로 복원합니다.",
                      },
                      {
                        value: "close" as LauncherBehaviorOnLaunch,
                        title: "런처 즉시 종료",
                        desc: "저사양 PC에서 시스템 메모리를 아끼기 위해 런처를 닫습니다.",
                      },
                    ] as const
                  ).map((opt) => {
                    const selected = draft.launcher.behaviorOnLaunch === opt.value;
                    return (
                      <button
                        key={opt.value}
                        type="button"
                        onClick={() =>
                          setDraft((prev) => ({
                            ...prev,
                            launcher: {
                              ...prev.launcher,
                              behaviorOnLaunch: opt.value,
                            },
                          }))
                        }
                        className={`p-3.5 rounded-xl border text-left transition-all cursor-pointer ${
                          selected
                            ? "bg-szz-matcha/15 border-szz-matcha text-szz-text shadow-sm"
                            : "bg-szz-base border-szz-border text-szz-subtext-0 hover:text-szz-text"
                        }`}
                      >
                        <div className="text-xs font-bold font-pixel text-szz-text">
                          {opt.title}
                        </div>
                        <div className="text-[11px] text-szz-subtext-0 mt-1 leading-relaxed">
                          {opt.desc}
                        </div>
                      </button>
                    );
                  })}
                </div>
              </div>

              {/* Theme Selection */}
              <div className="p-4 rounded-xl bg-szz-surface-0 border border-szz-border space-y-3">
                <div>
                  <h3 className="text-xs font-bold font-pixel text-szz-text">
                    컬러 테마 (Matcha Theme System)
                  </h3>
                  <p className="text-[11px] text-szz-subtext-0 mt-0.5">
                    Catppuccin 파스텔 계층 기반의 라이트 모드(Matcha Latte)와 다크 모드(Matcha Dark)를 전환합니다.
                  </p>
                </div>

                <div className="grid grid-cols-2 gap-3">
                  <button
                    type="button"
                    onClick={() =>
                      setDraft((prev) => ({
                        ...prev,
                        launcher: { ...prev.launcher, theme: "latte" },
                      }))
                    }
                    className={`p-3.5 rounded-xl border flex items-center gap-3 cursor-pointer transition-all ${
                      draft.launcher.theme === "latte"
                        ? "bg-szz-matcha/15 border-szz-matcha text-szz-text"
                        : "bg-szz-base border-szz-border text-szz-subtext-0"
                    }`}
                  >
                    <Sun className="w-5 h-5 text-szz-amber shrink-0" />
                    <div className="text-left">
                      <div className="text-xs font-bold font-pixel">Matcha Latte</div>
                      <div className="text-[10px] text-szz-subtext-0">Warm Light Mode</div>
                    </div>
                  </button>

                  <button
                    type="button"
                    onClick={() =>
                      setDraft((prev) => ({
                        ...prev,
                        launcher: { ...prev.launcher, theme: "dark" },
                      }))
                    }
                    className={`p-3.5 rounded-xl border flex items-center gap-3 cursor-pointer transition-all ${
                      draft.launcher.theme === "dark"
                        ? "bg-szz-matcha/15 border-szz-matcha text-szz-text"
                        : "bg-szz-base border-szz-border text-szz-subtext-0"
                    }`}
                  >
                    <Moon className="w-5 h-5 text-szz-matcha shrink-0" />
                    <div className="text-left">
                      <div className="text-xs font-bold font-pixel">Matcha Dark</div>
                      <div className="text-[10px] text-szz-subtext-0">Deep Moss Dark Mode</div>
                    </div>
                  </button>
                </div>
              </div>
            </div>
          )}

          {/* TAB 5: SYSTEM INFO */}
          {activeTab === "system" && (
            <div className="space-y-4">
              <div className="p-5 rounded-xl bg-szz-surface-0 border border-szz-border space-y-3">
                <div className="flex items-center justify-between border-b border-szz-border pb-3">
                  <div>
                    <div className="text-sm font-bold font-pixel text-szz-text">
                      szz (새짹짹) 데스크톱 런처
                    </div>
                    <div className="text-xs text-szz-subtext-0 mt-0.5">
                      Pure Rust Native Minecraft Modded Server Platform
                    </div>
                  </div>
                  <span className="px-2.5 py-1 rounded-full bg-szz-matcha/15 text-szz-matcha border border-szz-matcha/30 font-mono text-xs font-bold">
                    v0.2.0
                  </span>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-3 text-xs">
                  <div className="p-3 rounded-lg bg-szz-base border border-szz-border">
                    <span className="text-szz-subtext-0 block text-[10px]">코어 아키텍처</span>
                    <span className="font-mono font-bold text-szz-text">
                      szz-core (Rust) + Tauri v2
                    </span>
                  </div>
                  <div className="p-3 rounded-lg bg-szz-base border border-szz-border">
                    <span className="text-szz-subtext-0 block text-[10px]">호스트 물리 메모리</span>
                    <span className="font-mono font-bold text-szz-text">
                      총 {(sysMem.totalMemoryMb / 1024).toFixed(1)} GB (가용{" "}
                      {(sysMem.availableMemoryMb / 1024).toFixed(1)} GB)
                    </span>
                  </div>
                  <div className="p-3 rounded-lg bg-szz-base border border-szz-border">
                    <span className="text-szz-subtext-0 block text-[10px]">모드 스토리지 방식</span>
                    <span className="font-mono font-bold text-szz-text">
                      SHA-1/512 CAS & OS Hardlink
                    </span>
                  </div>
                  <div className="p-3 rounded-lg bg-szz-base border border-szz-border">
                    <span className="text-szz-subtext-0 block text-[10px]">Java JRE 프로비저닝</span>
                    <span className="font-mono font-bold text-szz-text">
                      Adoptium Temurin API v3 격리
                    </span>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Modal Footer (Discord-style Unsaved Changes Bar when dirty) */}
        <div
          className={`px-6 py-4 border-t flex items-center justify-between shrink-0 transition-colors ${getFooterContainerClass()}`}
        >
          {isDirty ? (
            <>
              <div className="flex items-center gap-2 text-xs font-pixel">
                <AlertTriangle
                  className={`w-4 h-4 shrink-0 ${
                    unsavedCloseWarning ? "text-szz-berry" : "text-szz-amber"
                  }`}
                />
                <span
                  className={`font-bold ${
                    unsavedCloseWarning ? "text-szz-berry" : "text-szz-text"
                  }`}
                >
                  {unsavedCloseWarning
                    ? "저장하지 않은 변경사항이 있어 창을 닫을 수 없습니다!"
                    : "조심하세요 — 저장하지 않은 변경사항이 있습니다!"}
                </span>
              </div>

              <div className="flex items-center gap-2.5">
                <button
                  type="button"
                  onClick={handleDiscardChanges}
                  disabled={isSaving || isCleaning}
                  className="px-4 py-2 rounded-xl text-xs font-semibold text-szz-subtext-1 hover:text-szz-text hover:bg-szz-surface-0 transition-colors cursor-pointer disabled:opacity-40"
                >
                  되돌리기
                </button>

                <button
                  type="button"
                  onClick={handleSave}
                  disabled={isSaving || isCleaning}
                  className="flex items-center gap-2 px-5 py-2.5 rounded-xl text-xs font-bold font-pixel bg-szz-matcha text-szz-chassis hover:opacity-90 transition-all shadow-md cursor-pointer active:scale-95 disabled:opacity-50"
                >
                  {isSaving ? (
                    <Loader2 className="w-4 h-4 animate-spin" />
                  ) : (
                    <Save className="w-4 h-4" />
                  )}
                  <span>설정 저장</span>
                </button>
              </div>
            </>
          ) : (
            <>
              <button
                type="button"
                onClick={handleReset}
                disabled={isSaving || isCleaning}
                className="flex items-center gap-1.5 px-3.5 py-2 rounded-xl text-xs font-semibold text-szz-subtext-1 hover:text-szz-text hover:bg-szz-surface-0 border border-szz-border transition-colors cursor-pointer disabled:opacity-40"
              >
                <RotateCcw className="w-3.5 h-3.5" />
                <span>기본값으로 초기화</span>
              </button>

              <button
                type="button"
                onClick={handleAttemptClose}
                disabled={isSaving || isCleaning}
                className="px-4 py-2 rounded-xl text-xs font-semibold text-szz-subtext-1 hover:text-szz-text hover:bg-szz-surface-0 transition-colors cursor-pointer disabled:opacity-40"
              >
                닫기
              </button>
            </>
          )}
        </div>
      </div>

      {/* Storage Clean Confirmation Sub-Modal */}
      {confirmCleanOpen && (
        <div
          onClick={() => setConfirmCleanOpen(false)}
          className="fixed inset-0 z-60 flex items-center justify-center p-4 bg-black/70 backdrop-blur-sm"
        >
          <div
            onClick={(e) => e.stopPropagation()}
            className="w-full max-w-md bg-szz-base border border-szz-border rounded-2xl p-6 shadow-2xl space-y-4 text-szz-text"
          >
            <div className="flex items-center gap-3 text-szz-amber">
              <div className="w-10 h-10 rounded-xl bg-szz-amber/15 border border-szz-amber/30 flex items-center justify-center shrink-0">
                <AlertTriangle className="w-5 h-5" />
              </div>
              <h3 className="text-sm font-bold font-pixel text-szz-text">
                선택한 캐시 리소스 정리
              </h3>
            </div>

            <p className="text-xs text-szz-subtext-1 leading-relaxed">
              선택한 항목{" "}
              <strong className="text-szz-matcha font-mono">
                {formatBytes(selectedReclaimableBytes)}
              </strong>
              를 로컬 디스크에서 영구 삭제하시겠습니까?
              <br />
              현재 설치된 어떤 서버에서도 사용하지 않는 리소스만 안전하게 선별 삭제됩니다.
            </p>

            <div className="flex items-center justify-end gap-3 pt-2">
              <button
                type="button"
                onClick={() => setConfirmCleanOpen(false)}
                className="px-4 py-2 rounded-xl text-xs font-semibold text-szz-subtext-1 hover:text-szz-text hover:bg-szz-surface-0 transition-colors cursor-pointer"
              >
                취소
              </button>
              <button
                type="button"
                onClick={handleCleanStorage}
                className="px-4 py-2 rounded-xl text-xs font-bold font-pixel bg-szz-matcha text-szz-chassis hover:opacity-90 transition-all cursor-pointer shadow-md"
              >
                정리 실행 (영구 삭제)
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
