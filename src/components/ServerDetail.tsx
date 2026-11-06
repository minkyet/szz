import React, { useState, useMemo, useRef } from "react";
import {
  ServerCatalogItem,
  ProgressPayload,
  InstanceLifecycleStatus,
  isInstanceInstalled,
  isInstanceRunning,
  isInstanceUpdateAvailable,
  isInstanceUpdating,
  isModEntryVerified,
  isManifestVerified,
} from "../types/launcher";
import { ServerStatusPayload } from "../types/settings";
import { LaunchProgressBar } from "./LaunchProgressBar";
import { useClipboard } from "../hooks/useClipboard";
import { useLinearScroll } from "../hooks/useLinearScroll";
import {
  Play,
  Download,
  Trash2,
  Loader2,
  Copy,
  Check,
  Package,
  Layers,
  HardDrive,
  Cpu,
  Globe,
  Radio,
  FileCode,
  Image as ImageIcon,
  ShieldCheck,
  ShieldAlert,
  RefreshCw,
  Users,
  Wifi,
  AlertTriangle,
  X,
  Share2,
} from "lucide-react";
import { usePlatform, isTauri } from "../platform";
import { viewToPathname } from "../hooks/useAppRouter";

interface ServerDetailProps {
  server: ServerCatalogItem;
  status: InstanceLifecycleStatus;
  progress: ProgressPayload | null;
  isOperating: boolean;
  onInstall: () => void;
  onLaunch: () => void;
  onUninstall: () => void;
  onUpdate?: () => void;
  onPlay?: () => void;
  dryRun: boolean;
  onToggleDryRun: (v: boolean) => void;
  serverStatus?: ServerStatusPayload | null;
  isPinging?: boolean;
  onRefreshPing?: () => void;
}

const ARCH_FEATURES = [
  {
    icon: Cpu,
    iconStyle: "text-szz-amber bg-szz-amber/15 border-szz-amber/30",
    title: "Isolated Java Runtime",
    desc: "Zero host PATH pollution. Automatically provisions Adoptium Temurin OpenJDK directly into ~/.szz/runtimes/.",
  },
  {
    icon: HardDrive,
    iconStyle: "text-szz-matcha bg-szz-matcha/15 border-szz-matcha/30",
    title: "Content-Addressable Storage",
    desc: "Zero disk space waste. Mods are cached once in CAS store and hardlinked directly into instances without file duplication.",
  },
  {
    icon: Globe,
    iconStyle: "text-szz-bamboo bg-szz-bamboo/15 border-szz-bamboo/30",
    title: "QuickPlay Direct Entry",
    desc: "Bypasses title screen and server menus. Launches directly into multiplayer target via Mojang QuickPlay protocol.",
  },
] as const;

export const ServerDetail: React.FC<ServerDetailProps> = ({
  server,
  status,
  progress,
  isOperating,
  onInstall,
  onLaunch,
  onUninstall,
  onUpdate,
  onPlay,
  dryRun,
  onToggleDryRun,
  serverStatus,
  isPinging = false,
  onRefreshPing,
}) => {
  const adapter = usePlatform();
  const canManageLocal = adapter.capabilities.canManageLocalInstances;

  const [activeTab, setActiveTab] = useState<"overview" | "mods" | "manifest">(
    "overview"
  );
  const [showUnverifiedModal, setShowUnverifiedModal] = useState<boolean>(false);
  const [unverifiedAction, setUnverifiedAction] = useState<"install" | "update">("install");
  const { copied: copiedAddress, copy: copyAddress } = useClipboard();
  const { copied: copiedShareLink, copy: copyShareLink } = useClipboard();

  const handleCopyShareLink = async () => {
    const isLocal =
      typeof window !== "undefined" &&
      (window.location.hostname === "localhost" ||
        window.location.hostname === "127.0.0.1");

    const baseOrigin =
      isTauri() || isLocal || typeof window === "undefined"
        ? "https://szz.nz"
        : window.location.origin;

    const path = viewToPathname("catalog", server.id);
    await copyShareLink(`${baseOrigin}${path}`);
  };

  const manifest = server.manifest;
  const mods = manifest?.mods ?? [];
  const mcVersion = manifest?.minecraftVersion ?? "1.20.1";
  const loaderType = manifest?.loader.type ?? "fabric";
  const loaderVersion = manifest?.loader.version ?? "0.16.10";
  const serverAddress = manifest?.serverAddress ?? "play.szz-mc.net:25565";

  const isVerified = useMemo(() => isManifestVerified(manifest), [manifest]);
  const customMods = useMemo(
    () => mods.filter((m) => !isModEntryVerified(m)),
    [mods]
  );

  const formattedManifest = useMemo(
    () => (manifest ? JSON.stringify(manifest, null, 2) : "{}"),
    [manifest]
  );

  const isInstalled = isInstanceInstalled(status);
  const isRunning = isInstanceRunning(status);
  const isUpdateAvailable = isInstanceUpdateAvailable(status);
  const isUpdating = isInstanceUpdating(status);
  const isOffline = Boolean(serverStatus && !serverStatus.online && !isPinging);
  const isBlockedOffline = !dryRun && isOffline;

  const handleInstallClick = () => {
    if (!isVerified && customMods.length > 0) {
      setUnverifiedAction("install");
      setShowUnverifiedModal(true);
    } else {
      onInstall();
    }
  };

  const handleUpdateClick = () => {
    if (!isVerified && customMods.length > 0) {
      setUnverifiedAction("update");
      setShowUnverifiedModal(true);
    } else {
      onUpdate?.();
    }
  };

  const renderSlpStatusContent = () => {
    if (isPinging) {
      return (
        <>
          <Loader2 className="w-3 h-3 text-szz-amber animate-spin" />
          <span className="text-szz-subtext-0">PINGING...</span>
        </>
      );
    }
    if (serverStatus?.online) {
      return (
        <>
          <span className="w-2 h-2 rounded-full bg-szz-matcha animate-pulse" />
          <span className="text-szz-matcha">ONLINE</span>
          {serverStatus.playersOnline !== null &&
            serverStatus.playersOnline !== undefined &&
            serverStatus.playersMax !== null &&
            serverStatus.playersMax !== undefined && (
              <span className="flex items-center gap-1 pl-1.5 ml-1 border-l border-szz-border text-szz-text font-mono">
                <Users className="w-3 h-3 text-szz-subtext-0" />
                {serverStatus.playersOnline}/{serverStatus.playersMax}
              </span>
            )}
          {serverStatus.latencyMs !== null && serverStatus.latencyMs !== undefined && (
            <span className="flex items-center gap-1 pl-1.5 ml-1 border-l border-szz-border text-szz-subtext-0 font-mono text-[11px]">
              <Wifi className="w-3 h-3 text-szz-matcha" />
              {serverStatus.latencyMs}ms
            </span>
          )}
        </>
      );
    }
    if (serverStatus && !serverStatus.online) {
      return (
        <>
          <span className="w-2 h-2 rounded-full bg-szz-berry" />
          <span className="text-szz-berry">OFFLINE</span>
        </>
      );
    }
    return (
      <>
        <span className="w-2 h-2 rounded-full bg-szz-matcha animate-pulse" />
        <span>ONLINE</span>
      </>
    );
  };

  const renderLaunchButtonIcon = () => {
    if (isRunning) {
      return <Radio className="w-5 h-5 animate-pulse text-szz-actionPlayText" />;
    }
    if (isOperating) {
      return <Loader2 className="w-5 h-5 animate-spin" />;
    }
    if (isBlockedOffline) {
      return <AlertTriangle className="w-5 h-5 text-szz-berry" />;
    }
    return <Play className="w-5 h-5 fill-current" />;
  };

  const getLaunchButtonLabel = () => {
    if (isRunning) return "실행 중 (RUNNING)";
    if (isOperating) return "실행 준비 중...";
    if (isBlockedOffline) return "서버 오프라인 (점검 중)";
    if (dryRun) return "드라이런 검증";
    return "접속하기 (PLAY)";
  };

  const scrollContainerRef = useRef<HTMLDivElement>(null);
  useLinearScroll(scrollContainerRef);

  return (
    <div
      ref={scrollContainerRef}
      className="flex-1 flex flex-col h-full overflow-y-auto bg-szz-base select-none text-szz-text"
    >
      {/* Top Hero Banner Section */}
      <div className="relative shrink-0 border-b border-szz-border bg-gradient-to-b from-szz-mantle via-szz-base to-szz-base p-6 lg:p-8">
        {/* Glow backdrop decoration */}
        <div className="absolute top-0 right-0 w-96 h-64 bg-szz-matcha/10 rounded-full blur-3xl pointer-events-none transform-gpu [backface-visibility:hidden]" />

        <div className="relative z-10 max-w-5xl mx-auto flex flex-col gap-6">
          {/* Title & Badges */}
          <div>
            <div className="flex flex-wrap items-center gap-2 mb-3">
              {/* Live SLP Status Badge */}
              <div className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-szz-surface-2 text-szz-text border border-szz-border font-pixel text-xs font-bold shadow-sm">
                {renderSlpStatusContent()}
                {onRefreshPing && (
                  <button
                    onClick={onRefreshPing}
                    disabled={isPinging}
                    className="ml-1 p-0.5 rounded hover:bg-szz-surface-1 text-szz-subtext-0 hover:text-szz-text transition-colors cursor-pointer disabled:opacity-40"
                    title="서버 상태 새로고침 (SLP Ping)"
                  >
                    <RefreshCw className={`w-3 h-3 ${isPinging ? "animate-spin" : ""}`} />
                  </button>
                )}
              </div>

              {/* Official Verified / Unverified Security Badge */}
              {isVerified ? (
                <span
                  className="flex items-center gap-1 px-2.5 py-1 rounded bg-szz-matcha/15 border border-szz-matcha/40 text-szz-matcha font-pixel text-xs font-bold"
                  title="모든 모드가 공식 Modrinth/CurseForge 레지스트리 및 SHA-1 무결성 검증을 통과했습니다."
                >
                  <ShieldCheck className="w-3.5 h-3.5" />
                  <span>Official Verified</span>
                </span>
              ) : (
                <span
                  className="flex items-center gap-1 px-2.5 py-1 rounded bg-szz-amber/15 border border-szz-amber/40 text-szz-amber font-pixel text-xs font-bold"
                  title={`미검증 외부(custom) 모드 ${customMods.length}개가 포함되어 있습니다.`}
                >
                  <ShieldAlert className="w-3.5 h-3.5" />
                  <span>Custom Mods ({customMods.length})</span>
                </span>
              )}

              <span className="px-2.5 py-1 rounded bg-szz-surface-0 border border-szz-border text-szz-subtext-0 font-pixel text-xs">
                MC {mcVersion}
              </span>

              <span className="px-2.5 py-1 rounded bg-szz-surface-0 border border-szz-border text-szz-subtext-0 font-pixel text-xs capitalize">
                {loaderType} {loaderVersion}
              </span>

              <span className="px-2.5 py-1 rounded bg-szz-surface-0 border border-szz-border text-szz-subtext-0 font-pixel text-xs">
                {mods.length} Mods
              </span>
            </div>

            <h1 className="text-3xl lg:text-5xl font-bold text-szz-text font-pixel tracking-wide">
              {server.name}
            </h1>
            <p className="text-sm lg:text-base text-szz-subtext-0 font-pixel mt-2 max-w-2xl">
              {serverStatus?.online && serverStatus.motd ? serverStatus.motd : server.tagline}
            </p>
          </div>

          {/* Action CTA & Server IP */}
          <div className="flex flex-wrap items-center justify-between gap-4 pt-2">
            {/* 2-Stage Lifecycle Buttons or Web Play CTA */}
            <div className="flex items-center gap-3">
              {!canManageLocal ? (
                <button
                  onClick={() => (onPlay ? onPlay() : adapter.play(server.id))}
                  disabled={isOperating}
                  className="h-12 px-8 rounded-lg bg-szz-matcha hover:opacity-90 active:scale-[0.98] text-szz-actionPlayText font-pixel font-bold text-sm tracking-wider flex items-center gap-2.5 shadow-lg shadow-szz-matcha/20 cursor-pointer disabled:opacity-60 disabled:cursor-wait"
                >
                  {isOperating ? (
                    <Loader2 className="w-5 h-5 animate-spin" />
                  ) : (
                    <Play className="w-5 h-5 fill-current" />
                  )}
                  <span>
                    {isOperating
                      ? "szz 런처 연결 확인 중..."
                      : "szz로 플레이 (PLAY ON SZZ)"}
                  </span>
                </button>
              ) : (
                <>
                  {!isInstalled ? (
                    <button
                      onClick={handleInstallClick}
                      disabled={isOperating}
                      className="h-12 px-8 rounded-lg bg-szz-surface-2 hover:bg-szz-surface-1 border border-szz-matcha text-szz-matcha active:scale-[0.98] font-pixel font-bold text-sm tracking-wider flex items-center gap-2.5 shadow-md shadow-szz-matcha/10 transition-all cursor-pointer disabled:opacity-50 disabled:cursor-wait"
                    >
                      {isOperating ? (
                        <Loader2 className="w-5 h-5 animate-spin" />
                      ) : (
                        <Download className="w-5 h-5" />
                      )}
                      <span>{isOperating ? "설치 진행 중..." : "설치 (INSTALL)"}</span>
                    </button>
                  ) : (
                    <div className="flex items-center gap-2">
                      {isUpdateAvailable || isUpdating ? (
                        <button
                          onClick={handleUpdateClick}
                          disabled={isOperating || isUpdating}
                          className="h-12 px-8 rounded-lg bg-szz-amber hover:opacity-90 active:scale-[0.98] text-szz-actionPlayText font-pixel font-bold text-sm tracking-wider flex items-center gap-2.5 shadow-lg shadow-szz-amber/20 transition-all cursor-pointer disabled:opacity-50"
                        >
                          {isOperating || isUpdating ? (
                            <Loader2 className="w-5 h-5 animate-spin" />
                          ) : (
                            <RefreshCw className="w-5 h-5" />
                          )}
                          <span>
                            {isOperating || isUpdating
                              ? "업데이트 진행 중..."
                              : "업데이트 (UPDATE)"}
                          </span>
                        </button>
                      ) : (
                        <button
                          onClick={onLaunch}
                          disabled={isOperating || isRunning || isBlockedOffline}
                          title={
                            isBlockedOffline
                              ? "현재 서버가 오프라인(점검 중) 상태이므로 접속할 수 없습니다."
                              : undefined
                          }
                          className={`h-12 px-8 rounded-lg font-pixel font-bold text-sm tracking-wider flex items-center gap-2.5 transition-all ${
                            isBlockedOffline
                              ? "bg-szz-surface-2 border border-szz-border text-szz-subtext-0 opacity-60 cursor-not-allowed shadow-none"
                              : "bg-szz-matcha hover:opacity-90 active:scale-[0.98] text-szz-actionPlayText shadow-lg shadow-szz-matcha/20 cursor-pointer disabled:opacity-50"
                          }`}
                        >
                          {renderLaunchButtonIcon()}
                          <span>{getLaunchButtonLabel()}</span>
                        </button>
                      )}

                      <button
                        onClick={onUninstall}
                        disabled={isOperating || isRunning}
                        className="h-12 px-3 rounded-lg bg-szz-surface-0 hover:bg-szz-berry/20 border border-szz-border hover:border-szz-berry text-szz-subtext-0 hover:text-szz-berry transition-colors flex items-center justify-center cursor-pointer disabled:opacity-40"
                        title="인스턴스 제거 (Uninstall)"
                      >
                        <Trash2 className="w-4 h-4" />
                      </button>
                    </div>
                  )}

                  {/* Dry-Run Toggle */}
                  <label className="flex items-center gap-2 text-xs font-pixel text-szz-subtext-0 cursor-pointer pl-2">
                    <input
                      type="checkbox"
                      checked={dryRun}
                      onChange={(e) => onToggleDryRun(e.target.checked)}
                      className="rounded border-szz-border bg-szz-surface-0 text-szz-matcha focus:ring-0"
                    />
                    <span>Dry-Run 모드</span>
                  </label>
                </>
              )}
            </div>

            {/* Server IP Address Pill & Share Link */}
            <div className="flex items-center gap-2">
              <button
                onClick={() => copyAddress(serverAddress)}
                className="flex items-center gap-2 px-3 py-2 rounded-lg bg-szz-surface-0/90 border border-szz-border text-xs hover:border-szz-matcha transition-colors cursor-pointer group"
                title="Click to copy server IP"
              >
                <span className="text-szz-subtext-0 font-pixel">SERVER IP:</span>
                <span className="font-mono font-bold text-szz-text">
                  {serverAddress}
                </span>
                {copiedAddress ? (
                  <Check className="w-3.5 h-3.5 text-szz-matcha" />
                ) : (
                  <Copy className="w-3.5 h-3.5 text-szz-subtext-0 group-hover:text-szz-text" />
                )}
              </button>

              <button
                onClick={handleCopyShareLink}
                className="flex items-center gap-1.5 px-3 py-2 rounded-lg bg-szz-surface-0/90 border border-szz-border text-xs hover:border-szz-matcha text-szz-subtext-0 hover:text-szz-text transition-colors cursor-pointer group"
                title="서버 공유 링크 복사 (웹 브라우저 및 디스코드 공유용)"
              >
                {copiedShareLink ? (
                  <>
                    <Check className="w-3.5 h-3.5 text-szz-matcha" />
                    <span className="font-pixel text-szz-matcha font-bold">링크 복사됨!</span>
                  </>
                ) : (
                  <>
                    <Share2 className="w-3.5 h-3.5 text-szz-subtext-0 group-hover:text-szz-matcha" />
                    <span className="font-pixel">공유 링크</span>
                  </>
                )}
              </button>
            </div>
          </div>

          {/* Offline Warning Notice */}
          {isOffline && !isRunning && (
            <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-szz-berry/15 border border-szz-berry/30 text-szz-berry text-xs font-pixel">
              <AlertTriangle className="w-4 h-4 shrink-0 text-szz-berry" />
              <span>현재 서버가 오프라인(점검 중) 상태이므로 접속할 수 없습니다. 서버 상태가 복구된 후 다시 시도해 주세요.</span>
            </div>
          )}

          {/* Real-time Progress Bar (shown when installing or launching) */}
          {(isOperating || progress) && (
            <div className="pt-2">
              <LaunchProgressBar
                progress={progress}
                isActive={isOperating}
                isCompleted={status === "Installed" && !isOperating}
                title={
                  status === "Installing"
                    ? "사전 동기화 파이프라인 (Pre-install Pipeline)"
                    : status === "Updating"
                    ? "증분 업데이트 파이프라인 (Incremental Update Pipeline)"
                    : "클라이언트 런처 파이프라인"
                }
              />
            </div>
          )}
        </div>
      </div>

      {/* Main Content Tabs & Views */}
      <div className="flex-1 max-w-5xl w-full mx-auto p-6 lg:p-8 flex flex-col gap-6">
        {/* Navigation Tabs */}
        <div className="flex items-center gap-2 border-b border-szz-border pb-3">
          <button
            onClick={() => setActiveTab("overview")}
            className={`px-4 py-2 rounded-lg font-pixel text-xs font-bold transition-all cursor-pointer ${
              activeTab === "overview"
                ? "bg-szz-surface-2 text-szz-text shadow-sm border border-szz-matcha/40"
                : "text-szz-subtext-0 hover:text-szz-text hover:bg-szz-surface-0/60"
            }`}
          >
            서버 개요 (Overview)
          </button>

          <button
            onClick={() => setActiveTab("mods")}
            className={`px-4 py-2 rounded-lg font-pixel text-xs font-bold transition-all cursor-pointer flex items-center gap-1.5 ${
              activeTab === "mods"
                ? "bg-szz-surface-2 text-szz-text shadow-sm border border-szz-matcha/40"
                : "text-szz-subtext-0 hover:text-szz-text hover:bg-szz-surface-0/60"
            }`}
          >
            <Package className="w-3.5 h-3.5" />
            <span>모드 목록 ({mods.length})</span>
          </button>

          <button
            onClick={() => setActiveTab("manifest")}
            className={`px-4 py-2 rounded-lg font-pixel text-xs font-bold transition-all cursor-pointer flex items-center gap-1.5 ${
              activeTab === "manifest"
                ? "bg-szz-surface-2 text-szz-text shadow-sm border border-szz-matcha/40"
                : "text-szz-subtext-0 hover:text-szz-text hover:bg-szz-surface-0/60"
            }`}
          >
            <FileCode className="w-3.5 h-3.5" />
            <span>매니페스트 (JSON)</span>
          </button>
        </div>

        {/* Tab 1: Overview */}
        {activeTab === "overview" && (
          <div className="space-y-8">
            {/* Screenshot Rail */}
            <div>
              <div className="flex items-center gap-2 mb-3">
                <ImageIcon className="w-4 h-4 text-szz-matcha" />
                <h3 className="font-pixel text-sm font-bold text-szz-text">
                  인게임 스크린샷 갤러리
                </h3>
              </div>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                {[1, 2, 3].map((idx) => (
                  <div
                    key={idx}
                    className="aspect-video rounded-xl bg-szz-surface-0 border border-szz-border overflow-hidden flex flex-col items-center justify-center text-szz-subtext-0 relative group shadow-sm"
                  >
                    <div className="w-full h-full bg-gradient-to-br from-szz-surface-1 via-szz-surface-0 to-szz-surface-1 flex flex-col items-center justify-center p-4 text-center">
                      <Layers className="w-8 h-8 text-szz-overlay-0 mb-2 group-hover:text-szz-matcha transition-colors" />
                      <span className="font-pixel text-xs text-szz-text">
                        {server.name} #0{idx}
                      </span>
                      <span className="text-[10px] text-szz-subtext-0 font-pixel mt-0.5">
                        16:9 In-Game Preview
                      </span>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Architecture Highlights */}
            <div>
              <h3 className="font-pixel text-sm font-bold text-szz-text mb-3">
                핵심 아키텍처 특장점
              </h3>
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                {ARCH_FEATURES.map((feat, idx) => {
                  const Icon = feat.icon;
                  return (
                    <div
                      key={idx}
                      className="p-4 rounded-xl bg-szz-surface-0 border border-szz-border flex flex-col gap-2.5 shadow-sm"
                    >
                      <div className="flex items-center gap-2.5">
                        <div
                          className={`w-8 h-8 rounded-lg flex items-center justify-center border ${feat.iconStyle}`}
                        >
                          <Icon className="w-4 h-4" />
                        </div>
                        <span className="font-pixel font-bold text-xs text-szz-text">
                          {feat.title}
                        </span>
                      </div>
                      <p className="text-xs text-szz-subtext-0 font-pixel leading-relaxed">
                        {feat.desc}
                      </p>
                    </div>
                  );
                })}
              </div>
            </div>

            {/* Operator Notice Feed */}
            <div className="p-5 rounded-xl bg-szz-surface-0 border border-szz-border flex flex-col gap-3 shadow-sm">
              <div className="flex items-center justify-between border-b border-szz-border/50 pb-2.5">
                <span className="font-pixel font-bold text-sm text-szz-text">
                  운영자 공지사항 (Notice)
                </span>
                <span className="text-[10px] text-szz-subtext-0 font-pixel">
                  2026-10-16
                </span>
              </div>
              <p className="text-xs text-szz-text font-pixel leading-relaxed">
                환영합니다! 본 서버는 <strong>szz</strong> 경량 런처를 통해 1-클릭으로
                자동 구성됩니다. 별도의 자바 설치나 수동 모드 복사 없이{" "}
                <span className="text-szz-matcha font-bold">[설치]</span> 클릭 후 바로{" "}
                <span className="text-szz-matcha font-bold">[접속하기]</span>를
                이용하실 수 있습니다.
              </p>
            </div>
          </div>
        )}

        {/* Tab 2: Mods List */}
        {activeTab === "mods" && (
          <div className="space-y-3">
            <div className="flex items-center justify-between text-xs text-szz-subtext-0 font-pixel pb-1">
              <span>동기화 대상 모드 ({mods.length}개)</span>
              <span>SHA-1 무결성 검증 & CAS 하드링크 적용</span>
            </div>

            {mods.length === 0 ? (
              <div className="p-8 text-center text-xs text-szz-subtext-0 font-pixel rounded-xl bg-szz-surface-0 border border-szz-border">
                바닐라 프로필입니다. 추가된 모드가 없습니다.
              </div>
            ) : (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-2.5 max-h-[500px] overflow-y-auto pr-1">
                {mods.map((mod, idx) => {
                  const isModVerified = isModEntryVerified(mod);
                  return (
                    <div
                      key={idx}
                      className="p-3 rounded-lg bg-szz-surface-0 border border-szz-border flex items-center justify-between text-xs font-pixel"
                    >
                      <div className="flex items-center gap-2.5 min-w-0">
                        <Package className="w-4 h-4 text-szz-subtext-0 shrink-0" />
                        <div className="min-w-0">
                          <div className="font-bold text-szz-text truncate">
                            {mod.name}
                          </div>
                          <div className="text-[10px] text-szz-subtext-0 truncate">
                            Source: {mod.source}
                          </div>
                        </div>
                      </div>
                      <div className="flex items-center gap-1.5 shrink-0">
                        <span
                          className={`text-[9px] px-1.5 py-0.5 rounded border ${
                            isModVerified
                              ? "bg-szz-matcha/10 border-szz-matcha/30 text-szz-matcha"
                              : "bg-szz-amber/10 border-szz-amber/30 text-szz-amber"
                          }`}
                        >
                          {isModVerified ? "Verified" : "Custom"}
                        </span>
                        <span className="text-[9px] px-1.5 py-0.5 rounded bg-szz-surface-1 border border-szz-border text-szz-subtext-0">
                          CAS
                        </span>
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        )}

        {/* Tab 3: Manifest JSON */}
        {activeTab === "manifest" && (
          <div className="space-y-2">
            <div className="flex items-center justify-between text-xs text-szz-subtext-0 font-pixel">
              <span>ServerManifest 원본 스펙</span>
              <span className="font-mono text-[10px]">{server.manifestPath}</span>
            </div>
            <pre className="p-4 rounded-xl bg-szz-crust border border-szz-border text-xs font-mono text-szz-text overflow-x-auto max-h-[500px]">
              {formattedManifest}
            </pre>
          </div>
        )}
      </div>

      {/* Unverified Custom Mod Security Warning Modal */}
      {showUnverifiedModal && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
          onClick={() => setShowUnverifiedModal(false)}
        >
          <div
            className="w-full max-w-md rounded-2xl bg-szz-base border border-szz-amber/50 shadow-2xl overflow-hidden flex flex-col font-pixel text-szz-text"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between px-5 py-4 border-b border-szz-border bg-szz-amber/10">
              <div className="flex items-center gap-2.5 text-szz-amber">
                <AlertTriangle className="w-5 h-5" />
                <span className="text-sm font-bold">미검증 외부 모드 보안 경고</span>
              </div>
              <button
                onClick={() => setShowUnverifiedModal(false)}
                className="p-1 rounded-lg hover:bg-szz-surface-1 text-szz-subtext-0 hover:text-szz-text cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="p-5 space-y-4 text-xs leading-relaxed">
              <p className="text-szz-subtext-0">
                이 서버 매니페스트에는 공식 레지스트리(Modrinth / CurseForge)에서 검증되지 않은{" "}
                <strong className="text-szz-amber">
                  외부 커스텀 모드({customMods.length}개)
                </strong>
                가 포함되어 있습니다. 신뢰할 수 있는 서버 운영자가 배포한 경우에만 설치를 진행해 주세요.
              </p>

              <div className="rounded-xl bg-szz-surface-0 border border-szz-border p-3 max-h-40 overflow-y-auto space-y-1.5">
                {customMods.map((mod, idx) => (
                  <div
                    key={idx}
                    className="flex items-center justify-between text-[11px] font-mono text-szz-text"
                  >
                    <span className="truncate font-bold">{mod.name}</span>
                    <span className="text-[10px] text-szz-amber shrink-0 ml-2">
                      source: {mod.source}
                    </span>
                  </div>
                ))}
              </div>
            </div>

            <div className="flex items-center justify-end gap-2 px-5 py-3.5 border-t border-szz-border bg-szz-mantle">
              <button
                onClick={() => setShowUnverifiedModal(false)}
                className="px-4 py-2 rounded-lg bg-szz-surface-1 hover:bg-szz-surface-2 text-xs text-szz-subtext-0 hover:text-szz-text cursor-pointer"
              >
                취소
              </button>
              <button
                onClick={() => {
                  setShowUnverifiedModal(false);
                  if (unverifiedAction === "update") {
                    onUpdate?.();
                  } else {
                    onInstall();
                  }
                }}
                className="px-4 py-2 rounded-lg bg-szz-amber text-szz-crust text-xs font-bold hover:opacity-90 cursor-pointer"
              >
                신뢰하고 {unverifiedAction === "update" ? "업데이트" : "설치"} 진행
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
