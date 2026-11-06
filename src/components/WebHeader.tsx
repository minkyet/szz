import React from "react";
import { Download, Sparkles, Sun, Moon, Compass } from "lucide-react";
import { AppView } from "../hooks/useAppRouter";
import { ThemeMode } from "../types/theme";

export interface WebHeaderProps {
  currentView: AppView;
  onNavigate: (view: AppView) => void;
  theme?: ThemeMode;
  onToggleTheme?: () => void;
}

export const WebHeader: React.FC<WebHeaderProps> = ({
  currentView,
  onNavigate,
  theme = "latte",
  onToggleTheme,
}) => {
  return (
    <header className="h-12 border-b border-szz-border bg-szz-mantle/80 backdrop-blur-md px-6 flex items-center justify-between shrink-0 z-30 select-none font-pixel">
      {/* Left: Branding & Breadcrumb */}
      <div className="flex items-center gap-3">
        <button
          onClick={() => onNavigate("catalog")}
          className="flex items-center gap-2 group cursor-pointer focus:outline-none"
        >
          <span className="font-bold text-sm tracking-wider text-szz-text group-hover:text-szz-matcha transition-colors">
            szz.nz
          </span>
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-szz-matcha/10 border border-szz-matcha/30 text-szz-matcha font-semibold uppercase">
            Web
          </span>
        </button>
        <span className="text-szz-subtext-0 text-xs hidden sm:inline">•</span>
        <span className="text-xs text-szz-subtext-1 hidden sm:inline">
          {currentView === "catalog" && "서버 카탈로그"}
          {currentView === "download" && "데스크톱 런처 다운로드"}
          {currentView === "studio" && "크리에이터 스튜디오"}
        </span>
      </div>

      {/* Right: Navigation Actions & Theme */}
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={() => onNavigate("catalog")}
          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
            currentView === "catalog"
              ? "bg-szz-surface-1 text-szz-text border border-szz-border shadow-sm"
              : "text-szz-subtext-0 hover:text-szz-text hover:bg-szz-surface-0"
          }`}
        >
          <Compass className="w-3.5 h-3.5" />
          <span className="hidden sm:inline">서버 둘러보기</span>
        </button>

        <button
          type="button"
          onClick={() => onNavigate("studio")}
          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
            currentView === "studio"
              ? "bg-szz-surface-1 text-szz-text border border-szz-border shadow-sm"
              : "text-szz-subtext-0 hover:text-szz-text hover:bg-szz-surface-0"
          }`}
        >
          <Sparkles className="w-3.5 h-3.5 text-szz-amber" />
          <span className="hidden sm:inline">내 서버 개설</span>
        </button>

        <button
          type="button"
          onClick={() => onNavigate("download")}
          className={`flex items-center gap-1.5 px-3.5 py-1.5 rounded-lg text-xs font-bold transition-all cursor-pointer ${
            currentView === "download"
              ? "bg-szz-matcha text-szz-base shadow-md shadow-szz-matcha/30"
              : "bg-szz-matcha/90 hover:bg-szz-matcha text-szz-base shadow-sm"
          }`}
        >
          <Download className="w-3.5 h-3.5" />
          <span>런처 다운로드</span>
        </button>

        {onToggleTheme && (
          <button
            type="button"
            onClick={onToggleTheme}
            className="p-1.5 ml-1 rounded-lg text-szz-subtext-0 hover:text-szz-text hover:bg-szz-surface-0 transition-colors cursor-pointer"
            title={`테마 전환 (${theme === "latte" ? "다크" : "라떼"})`}
          >
            {theme === "latte" ? <Moon className="w-4 h-4" /> : <Sun className="w-4 h-4 text-szz-amber" />}
          </button>
        )}
      </div>
    </header>
  );
};
