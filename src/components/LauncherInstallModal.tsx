import React from "react";
import { X, Download, RefreshCw, Zap, HardDrive, Cpu } from "lucide-react";

export interface LauncherInstallModalProps {
  isOpen: boolean;
  serverName?: string;
  minecraftVersion?: string;
  loaderType?: string;
  onClose: () => void;
  onDownload: () => void;
  onRetry: () => void;
  isRetrying?: boolean;
}

export const LauncherInstallModal: React.FC<LauncherInstallModalProps> = ({
  isOpen,
  serverName = "마인크래프트 서버",
  minecraftVersion,
  loaderType,
  onClose,
  onDownload,
  onRetry,
  isRetrying = false,
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4 animate-in fade-in duration-200">
      <div className="bg-szz-mantle border border-szz-border text-szz-text rounded-2xl max-w-md w-full shadow-2xl overflow-hidden font-pixel flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-szz-border bg-szz-crust/50">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-szz-matcha/20 border border-szz-matcha/40 flex items-center justify-center text-szz-matcha font-bold text-sm">
              szz
            </div>
            <div>
              <h3 className="font-bold text-sm tracking-wide text-szz-text">
                데스크톱 런처가 감지되지 않았습니다
              </h3>
              <p className="text-[11px] text-szz-subtext-0">
                원클릭 실행을 위해 szz 런처가 필요합니다
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 rounded-lg text-szz-subtext-0 hover:text-szz-text hover:bg-szz-surface-0 transition-colors cursor-pointer"
            aria-label="닫기"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Body Content */}
        <div className="p-6 flex flex-col gap-4">
          {/* Target Server Card */}
          <div className="bg-szz-surface-0/60 border border-szz-border rounded-xl p-3.5 flex items-center justify-between">
            <div className="flex flex-col">
              <span className="text-xs text-szz-subtext-0">플레이 대상 서버</span>
              <span className="font-bold text-sm text-szz-text truncate max-w-[240px]">
                {serverName}
              </span>
            </div>
            {(minecraftVersion || loaderType) && (
              <div className="flex items-center gap-1.5 shrink-0">
                {minecraftVersion && (
                  <span className="text-[10px] px-2 py-0.5 rounded bg-szz-surface-1 border border-szz-border text-szz-subtext-1">
                    v{minecraftVersion}
                  </span>
                )}
                {loaderType && (
                  <span className="text-[10px] px-2 py-0.5 rounded bg-szz-matcha/10 border border-szz-matcha/30 text-szz-matcha font-semibold uppercase">
                    {loaderType}
                  </span>
                )}
              </div>
            )}
          </div>

          <p className="text-xs text-szz-subtext-1 leading-relaxed">
            웹 브라우저에서는 Java 런타임과 마인크래프트 클라이언트를 직접 실행할 수 없습니다.
            <strong> szz 데스크톱 런처</strong>를 설치하면 클릭 한 번으로 모든 모드와 런타임이 자동 구성됩니다.
          </p>

          {/* Value Propositions */}
          <div className="grid grid-cols-1 gap-2.5 pt-1">
            <div className="flex items-start gap-2.5 text-xs text-szz-subtext-0">
              <Zap className="w-4 h-4 text-szz-matcha shrink-0 mt-0.5" />
              <span>
                <strong className="text-szz-text">원클릭 직행 (QuickPlay)</strong>: 모드팩 압축 해제 없이 즉시 서버 로비 입장
              </span>
            </div>
            <div className="flex items-start gap-2.5 text-xs text-szz-subtext-0">
              <Cpu className="w-4 h-4 text-szz-amber shrink-0 mt-0.5" />
              <span>
                <strong className="text-szz-text">Java 수동 설치 제로</strong>: 필요한 Java 버전을 샌드박스로 자동 격리
              </span>
            </div>
            <div className="flex items-start gap-2.5 text-xs text-szz-subtext-0">
              <HardDrive className="w-4 h-4 text-szz-bamboo shrink-0 mt-0.5" />
              <span>
                <strong className="text-szz-text">디스크 용량 50% 절감</strong>: CAS 하드링크로 중복 모드 파일 공유
              </span>
            </div>
          </div>
        </div>

        {/* Footer Actions */}
        <div className="p-4 border-t border-szz-border bg-szz-crust/50 flex items-center justify-between gap-3">
          <button
            type="button"
            onClick={onRetry}
            disabled={isRetrying}
            className="flex items-center gap-1.5 px-3 py-2 rounded-xl text-xs text-szz-subtext-0 hover:text-szz-text hover:bg-szz-surface-0 transition-colors disabled:opacity-50 cursor-pointer"
          >
            <RefreshCw className={`w-3.5 h-3.5 ${isRetrying ? "animate-spin" : ""}`} />
            <span>다시 시도</span>
          </button>

          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={onClose}
              className="px-3.5 py-2 rounded-xl text-xs text-szz-subtext-0 hover:text-szz-text hover:bg-szz-surface-0 transition-colors cursor-pointer"
            >
              나중에 하기
            </button>
            <button
              type="button"
              onClick={onDownload}
              className="flex items-center gap-2 px-4 py-2 rounded-xl bg-szz-matcha hover:bg-szz-matcha-hover active:bg-szz-matcha-active text-szz-base font-bold text-xs shadow-lg shadow-szz-matcha/20 transition-all cursor-pointer"
            >
              <Download className="w-4 h-4" />
              <span>szz 런처 다운로드</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
