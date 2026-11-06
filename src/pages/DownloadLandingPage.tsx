import React, { useState } from "react";
import {
  Download,
  ArrowLeft,
  Zap,
  HardDrive,
  Cpu,
  ShieldCheck,
  ExternalLink,
  Monitor,
  CheckCircle2,
  Sparkles,
  Terminal,
} from "lucide-react";
import { useOsDetector, DetectedOS, OsReleaseAsset } from "../hooks/useOsDetector";

export interface DownloadLandingPageProps {
  onBack?: () => void;
}

export const DownloadLandingPage: React.FC<DownloadLandingPageProps> = ({
  onBack,
}) => {
  const { os: detectedOs, primaryAsset, allAssets } = useOsDetector();
  const [selectedOs, setSelectedOs] = useState<DetectedOS>(
    detectedOs !== "unknown" ? detectedOs : "windows"
  );

  const handleBack = () => {
    onBack?.();
  };

  const currentAsset: OsReleaseAsset =
    allAssets.find((a) => a.os === selectedOs) || primaryAsset;

  return (
    <div className="flex-1 overflow-y-auto px-4 sm:px-8 py-10 flex flex-col items-center font-pixel text-szz-text">
      <div className="max-w-4xl w-full flex flex-col items-center gap-10">
        {/* Navigation Bar */}
        <div className="w-full flex items-center justify-between">
          <button
            type="button"
            onClick={handleBack}
            className="inline-flex items-center gap-2 text-xs text-szz-subtext-0 hover:text-szz-text transition-colors cursor-pointer group px-3 py-1.5 rounded-lg bg-szz-surface-0/60 border border-szz-border hover:border-szz-border-active"
          >
            <ArrowLeft className="w-3.5 h-3.5 group-hover:-translate-x-0.5 transition-transform text-szz-matcha" />
            <span>웹 카탈로그로 돌아가기</span>
          </button>

          <a
            href="https://github.com/minkyet/szz"
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center gap-1.5 text-xs text-szz-subtext-1 hover:text-szz-text transition-colors"
          >
            <span>GitHub v0.1.0</span>
            <ExternalLink className="w-3 h-3" />
          </a>
        </div>

        {/* Hero Section */}
        <div className="flex flex-col items-center text-center gap-4 max-w-2xl">
          <div className="inline-flex items-center gap-2 px-3.5 py-1.5 rounded-full bg-szz-matcha/10 border border-szz-matcha/30 text-szz-matcha text-xs font-semibold shadow-sm">
            <Sparkles className="w-3.5 h-3.5" />
            <span>Steam-style Desktop Modpack Platform</span>
          </div>

          <h1 className="text-3xl sm:text-5xl font-extrabold tracking-tight text-szz-text leading-tight">
            모드팩 설치와 서버 접속을 <br className="hidden sm:inline" />
            <span className="text-szz-matcha">1-클릭</span>으로 완료하세요
          </h1>

          <p className="text-sm sm:text-base text-szz-subtext-0 max-w-xl mx-auto leading-relaxed">
            복잡한 자바 수동 설치와 모드팩 압축 해제, 중복 다운로드로 인한 디스크 낭비 없이
            웹 브라우저나 데스크톱 런처에서 한 번에 마인크래프트 서버에 접속합니다.
          </p>

          {/* OS Switcher Tabs */}
          <div className="flex items-center gap-1 p-1 rounded-xl bg-szz-surface-0 border border-szz-border mt-2">
            {allAssets.map((asset) => {
              const isSelected = asset.os === selectedOs;
              const isDetected = asset.os === detectedOs;
              return (
                <button
                  key={asset.os}
                  type="button"
                  onClick={() => setSelectedOs(asset.os)}
                  className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer ${
                    isSelected
                      ? "bg-szz-matcha text-szz-base font-bold shadow-sm"
                      : "text-szz-subtext-0 hover:text-szz-text hover:bg-szz-surface-1"
                  }`}
                >
                  <span>{asset.osName}</span>
                  {isDetected && (
                    <span
                      className={`text-[10px] px-1 py-0.2 rounded ${
                        isSelected
                          ? "bg-szz-base/20 text-szz-base"
                          : "bg-szz-matcha/20 text-szz-matcha"
                      }`}
                    >
                      감지됨
                    </span>
                  )}
                </button>
              );
            })}
          </div>

          {/* Primary Download CTA Button */}
          <div className="flex flex-col sm:flex-row items-center gap-3 w-full sm:w-auto mt-2">
            <a
              href={currentAsset.downloadUrl}
              target="_blank"
              rel="noreferrer"
              className="flex items-center justify-center gap-2.5 px-8 py-3.5 rounded-xl bg-szz-matcha hover:bg-szz-matcha-hover active:bg-szz-matcha-active text-szz-base font-bold text-sm sm:text-base shadow-xl shadow-szz-matcha/25 transition-all cursor-pointer w-full sm:w-auto"
            >
              <Download className="w-5 h-5" />
              <span>{currentAsset.osName}용 szz 다운로드</span>
              <ExternalLink className="w-4 h-4 opacity-70" />
            </a>

            <button
              type="button"
              onClick={handleBack}
              className="px-6 py-3.5 rounded-xl bg-szz-surface-0 hover:bg-szz-surface-1 border border-szz-border text-szz-text text-xs sm:text-sm font-semibold transition-all cursor-pointer w-full sm:w-auto"
            >
              웹에서 서버 먼저 둘러보기
            </button>
          </div>

          {/* Target File details & Protocol note */}
          <div className="flex flex-col items-center gap-1 text-[11px] text-szz-subtext-1">
            <p>
              파일: <span className="text-szz-text font-mono">{currentAsset.filename}</span> ({currentAsset.badge})
            </p>
            <p className="text-szz-matcha/90">
              ✓ 설치 시 웹 브라우저에서 szz:// 1-클릭 실행 프로토콜이 자동 등록됩니다.
            </p>
          </div>
        </div>

        {/* Desktop UI Mockup Card */}
        <div className="w-full rounded-2xl bg-szz-mantle border border-szz-border shadow-2xl overflow-hidden flex flex-col">
          {/* Mockup Window Header */}
          <div className="px-4 py-2.5 bg-szz-crust border-b border-szz-border flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="w-3 h-3 rounded-full bg-szz-berry/80 inline-block" />
              <span className="w-3 h-3 rounded-full bg-szz-amber/80 inline-block" />
              <span className="w-3 h-3 rounded-full bg-szz-matcha/80 inline-block" />
            </div>
            <div className="flex items-center gap-2 text-xs text-szz-subtext-1 font-mono">
              <Monitor className="w-3.5 h-3.5 text-szz-matcha" />
              <span>szz Desktop Launcher Preview</span>
            </div>
            <div className="text-[10px] text-szz-subtext-1 font-mono">v0.1.0 (Native Core)</div>
          </div>

          {/* Mockup Window Body */}
          <div className="grid grid-cols-1 md:grid-cols-12 min-h-[280px]">
            {/* Mockup Left Sidebar */}
            <div className="md:col-span-4 p-4 bg-szz-crust/50 border-r border-szz-border flex flex-col gap-2">
              <div className="text-[11px] font-bold text-szz-subtext-1 uppercase tracking-wider mb-1">
                서버 라이브러리
              </div>
              <div className="p-2.5 rounded-lg bg-szz-surface-0 border border-szz-matcha/40 flex items-center gap-3">
                <div className="w-9 h-9 rounded bg-szz-matcha/20 text-szz-matcha flex items-center justify-center font-bold text-xs">
                  CB
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-xs font-bold text-szz-text truncate">Cobblemon Official</div>
                  <div className="text-[10px] text-szz-matcha truncate">1.20.1 • Fabric • 준비 완료</div>
                </div>
              </div>

              <div className="p-2.5 rounded-lg bg-szz-surface-0/30 border border-szz-border/40 flex items-center gap-3 opacity-75">
                <div className="w-9 h-9 rounded bg-szz-surface-1 text-szz-subtext-0 flex items-center justify-center font-bold text-xs">
                  PP
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-xs font-bold text-szz-text truncate">Paper 26.3 SMP</div>
                  <div className="text-[10px] text-szz-subtext-1 truncate">1.20.4 • Vanilla</div>
                </div>
              </div>

              <div className="p-2.5 rounded-lg bg-szz-surface-0/30 border border-szz-border/40 flex items-center gap-3 opacity-75">
                <div className="w-9 h-9 rounded bg-szz-surface-1 text-szz-subtext-0 flex items-center justify-center font-bold text-xs">
                  SF
                </div>
                <div className="flex-1 min-w-0">
                  <div className="text-xs font-bold text-szz-text truncate">SkyFactory 4</div>
                  <div className="text-[10px] text-szz-subtext-1 truncate">1.12.2 • Forge</div>
                </div>
              </div>
            </div>

            {/* Mockup Main View */}
            <div className="md:col-span-8 p-6 flex flex-col justify-between bg-szz-base">
              <div className="flex flex-col gap-3">
                <div className="flex items-center gap-2">
                  <span className="px-2 py-0.5 rounded text-[10px] bg-szz-matcha/20 text-szz-matcha font-bold">
                    Minecraft 1.20.1
                  </span>
                  <span className="px-2 py-0.5 rounded text-[10px] bg-szz-surface-1 text-szz-subtext-0">
                    Fabric Loader 0.16.10
                  </span>
                  <span className="px-2 py-0.5 rounded text-[10px] bg-szz-tea-cream/20 text-szz-tea-cream">
                    Adoptium Java 17
                  </span>
                </div>
                <h3 className="text-xl font-bold text-szz-text">Cobblemon Adventure Server</h3>
                <p className="text-xs text-szz-subtext-0 leading-relaxed">
                  포켓몬과 마인크래프트의 만남. 전용 커스텀 모드 48종과 텍스처팩이 매니페스트로 완벽하게 관리됩니다.
                </p>
              </div>

              {/* Mockup Play Action & Terminal Output */}
              <div className="flex flex-col gap-3 mt-6">
                <div className="flex items-center gap-3">
                  <div className="flex items-center gap-2 px-5 py-2.5 rounded-xl bg-szz-matcha text-szz-base font-bold text-xs shadow-lg shadow-szz-matcha/20">
                    <Zap className="w-4 h-4 fill-current" />
                    <span>QuickPlay 1-클릭 플레이</span>
                  </div>
                  <div className="text-[11px] text-szz-matcha flex items-center gap-1 font-mono">
                    <CheckCircle2 className="w-3.5 h-3.5" />
                    <span>최신 모드팩 동기화 완료</span>
                  </div>
                </div>

                <div className="p-2.5 rounded-lg bg-szz-crust border border-szz-border text-[10px] font-mono text-szz-subtext-1 flex flex-col gap-1">
                  <div className="flex items-center gap-1.5 text-szz-matcha">
                    <Terminal className="w-3 h-3" />
                    <span>[CAS] 48 mod JARs mapped via OS hardlinks (0 MB duplicated)</span>
                  </div>
                  <div className="text-szz-subtext-1">
                    [Runtime] Adoptium OpenJDK 17.0.10 JRE isolated at ~/.szz/runtimes/java-17
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Feature & Value Proposition Grid (4 Cards) */}
        <div className="w-full flex flex-col gap-4">
          <div className="text-center">
            <h2 className="text-xl sm:text-2xl font-bold text-szz-text">왜 szz인가요?</h2>
            <p className="text-xs text-szz-subtext-0 mt-1">
              마인크래프트 플레이어와 서버 관리자를 위한 4대 핵심 기술
            </p>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 w-full text-left">
            {/* Card 1 */}
            <div className="p-5 rounded-2xl bg-szz-surface-0/70 border border-szz-border hover:border-szz-matcha/50 transition-all flex flex-col gap-2.5">
              <div className="flex items-center gap-2.5 text-szz-matcha font-bold text-sm">
                <div className="p-2 rounded-lg bg-szz-matcha/10">
                  <Zap className="w-4 h-4" />
                </div>
                <span>원클릭 로비 직행 (QuickPlay)</span>
              </div>
              <p className="text-xs text-szz-subtext-0 leading-relaxed">
                웹이나 런처에서 버튼 하나만 누르면 필요한 모드 다운로드부터 마인크래프트 실행, 서버 주소 입력 없이 멀티플레이어 로비 자동 접속까지 원스톱으로 처리합니다.
              </p>
            </div>

            {/* Card 2 */}
            <div className="p-5 rounded-2xl bg-szz-surface-0/70 border border-szz-border hover:border-szz-bamboo/50 transition-all flex flex-col gap-2.5">
              <div className="flex items-center gap-2.5 text-szz-bamboo font-bold text-sm">
                <div className="p-2 rounded-lg bg-szz-bamboo/10">
                  <HardDrive className="w-4 h-4" />
                </div>
                <span>디스크 용량 50%+ 절감 (CAS 하드링크)</span>
              </div>
              <p className="text-xs text-szz-subtext-0 leading-relaxed">
                여러 서버 인스턴스를 설치해도 동일한 모드 파일은 중앙 저장소(<code className="text-szz-bamboo font-mono">~/.szz/store</code>)에 단 한 번만 캐시되고 OS 하드링크로 연결되어 SSD 용량을 획기적으로 아낍니다.
              </p>
            </div>

            {/* Card 3 */}
            <div className="p-5 rounded-2xl bg-szz-surface-0/70 border border-szz-border hover:border-szz-amber/50 transition-all flex flex-col gap-2.5">
              <div className="flex items-center gap-2.5 text-szz-amber font-bold text-sm">
                <div className="p-2 rounded-lg bg-szz-amber/10">
                  <Cpu className="w-4 h-4" />
                </div>
                <span>자바 수동 설치 제로 (Adoptium 자동 격리)</span>
              </div>
              <p className="text-xs text-szz-subtext-0 leading-relaxed">
                마인크래프트 버전에 따라 요구되는 Java 8, 17, 21을 런처가 자동 감지하여 격리된 전용 디렉터리에 설치합니다. 복잡한 환경 변수(PATH) 설정이 전혀 필요 없습니다.
              </p>
            </div>

            {/* Card 4 */}
            <div className="p-5 rounded-2xl bg-szz-surface-0/70 border border-szz-border hover:border-szz-mauve/50 transition-all flex flex-col gap-2.5">
              <div className="flex items-center gap-2.5 text-szz-mauve font-bold text-sm">
                <div className="p-2 rounded-lg bg-szz-mauve/10">
                  <ShieldCheck className="w-4 h-4" />
                </div>
                <span>표준 모드팩 지원 & szz:// 프로토콜</span>
              </div>
              <p className="text-xs text-szz-subtext-0 leading-relaxed">
                Modrinth(<code className="text-szz-mauve font-mono">.mrpack</code>) 및 CurseForge 표준 매니페스트를 지원하며, <code className="text-szz-mauve font-mono">szz://launch/&lt;id&gt;</code> 프로토콜을 OS에 등록하여 웹과 데스크톱이 끊김 없이 연결됩니다.
              </p>
            </div>
          </div>
        </div>

        {/* System Requirements & Architecture Footer */}
        <div className="w-full p-6 rounded-2xl bg-szz-surface-0/40 border border-szz-border flex flex-col sm:flex-row items-center justify-between gap-4 text-xs text-szz-subtext-1">
          <div className="flex flex-col gap-1 text-center sm:text-left">
            <div className="font-bold text-szz-text">시스템 요구 사양</div>
            <p>Windows 10/11 (64-bit) • macOS 11+ (Apple Silicon/Intel) • Linux (Ubuntu/Debian/Arch)</p>
          </div>
          <div className="flex items-center gap-2">
            <span className="px-2.5 py-1 rounded bg-szz-surface-1 border border-szz-border text-[11px] font-mono text-szz-subtext-0">
              Pure Rust Core • Tauri v2
            </span>
          </div>
        </div>
      </div>
    </div>
  );
};
