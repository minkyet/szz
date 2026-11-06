import React, { useState } from "react";
import { useClipboard } from "../hooks/useClipboard";
import {
  Terminal,
  ChevronUp,
  ChevronDown,
  Copy,
  Check,
  ToggleLeft,
  ToggleRight,
  Trash2,
} from "lucide-react";

export interface LogEntry {
  id: string;
  timestamp: string;
  level: "info" | "step" | "cmd" | "error" | "success";
  text: string;
}

interface LogDrawerProps {
  logs: LogEntry[];
  dryRun: boolean;
  onToggleDryRun: (dryRun: boolean) => void;
  lastCommand: string | null;
  onClearLogs: () => void;
  isLaunching: boolean;
}

const LOG_LEVEL_STYLES: Record<LogEntry["level"], { label: string; className: string }> = {
  step: { label: "STEP", className: "bg-szz-bamboo/20 text-szz-bamboo border-szz-bamboo/40" },
  cmd: { label: "EXEC", className: "bg-szz-mauve/20 text-szz-mauve border-szz-mauve/40" },
  error: { label: "ERR", className: "bg-szz-berry/20 text-szz-berry border-szz-berry/40" },
  success: { label: "OK", className: "bg-szz-matcha/20 text-szz-matcha border-szz-matcha/40" },
  info: { label: "INFO", className: "bg-szz-surface-1 text-szz-subtext-0 border-transparent" },
};

export const LogDrawer: React.FC<LogDrawerProps> = ({
  logs,
  dryRun,
  onToggleDryRun,
  lastCommand,
  onClearLogs,
  isLaunching,
}) => {
  const [isOpen, setIsOpen] = useState(false);
  const { copied, copy } = useClipboard();

  const handleCopyCommand = () => {
    if (lastCommand) {
      copy(lastCommand);
    }
  };

  const renderBadge = (level: LogEntry["level"]) => {
    const style = LOG_LEVEL_STYLES[level] ?? LOG_LEVEL_STYLES.info;
    return (
      <span
        className={`px-1.5 py-0.5 rounded font-mono text-[10px] uppercase border ${style.className}`}
      >
        {style.label}
      </span>
    );
  };

  return (
    <div className="border-t border-szz-border bg-szz-crust flex flex-col shrink-0 transition-all select-none">
      <div className="h-10 px-4 flex items-center justify-between text-xs bg-szz-mantle border-b border-szz-border">
        <div className="flex items-center gap-4">
          <button
            onClick={() => setIsOpen(!isOpen)}
            className="flex items-center gap-2 font-medium text-szz-text hover:text-szz-matcha transition-colors"
          >
            <Terminal className="w-4 h-4 text-szz-matcha" />
            <span>Launch Console</span>
            <span className="px-1.5 py-0.5 rounded-full bg-szz-surface-1 text-szz-subtext-0 text-[10px]">
              {logs.length}
            </span>
            {isOpen ? (
              <ChevronDown className="w-3.5 h-3.5 text-szz-subtext-0" />
            ) : (
              <ChevronUp className="w-3.5 h-3.5 text-szz-subtext-0" />
            )}
          </button>

          <div className="flex items-center gap-2 pl-4 border-l border-szz-border">
            <span className="text-szz-subtext-0 text-xs">Dry-Run:</span>
            <button
              onClick={() => onToggleDryRun(!dryRun)}
              disabled={isLaunching}
              title="When enabled, builds launcher command and verifies pipeline without spawning the game window"
              className={`flex items-center gap-1.5 px-2 py-0.5 rounded font-mono text-xs transition-colors ${
                dryRun
                  ? "bg-szz-amber/20 text-szz-amber border border-szz-amber/40 font-semibold"
                  : "bg-szz-surface-1 text-szz-subtext-0 hover:text-szz-text border border-szz-border"
              }`}
            >
              {dryRun ? (
                <>
                  <ToggleRight className="w-4 h-4 text-szz-amber" />
                  <span>ENABLED</span>
                </>
              ) : (
                <>
                  <ToggleLeft className="w-4 h-4 text-szz-subtext-0" />
                  <span>OFF</span>
                </>
              )}
            </button>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {lastCommand && (
            <button
              onClick={handleCopyCommand}
              className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-szz-surface-1 hover:bg-szz-surface-2 text-szz-text text-xs transition-colors border border-szz-border"
              title="Copy the final generated Java command"
            >
              {copied ? (
                <>
                  <Check className="w-3 h-3 text-szz-matcha" />
                  <span className="text-szz-matcha font-semibold">Copied</span>
                </>
              ) : (
                <>
                  <Copy className="w-3 h-3 text-szz-subtext-0" />
                  <span>Copy Command</span>
                </>
              )}
            </button>
          )}

          {logs.length > 0 && (
            <button
              onClick={onClearLogs}
              className="p-1 rounded text-szz-subtext-0 hover:text-szz-text hover:bg-szz-surface-1 transition-colors"
              title="Clear logs"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          )}
        </div>
      </div>

      {isOpen && (
        <div className="h-56 overflow-y-auto p-3 font-mono text-xs bg-szz-crust text-szz-text space-y-1.5 select-text">
          {lastCommand && (
            <div className="p-2 mb-2 rounded bg-szz-mauve/15 border border-szz-mauve/30 text-[11px] text-szz-text">
              <div className="font-semibold text-szz-mauve mb-1 flex items-center gap-1">
                <span>$ Java Launch Command:</span>
              </div>
              <div className="break-all whitespace-pre-wrap selection:bg-szz-mauve/40 selection:text-szz-text font-mono">
                {lastCommand}
              </div>
            </div>
          )}

          {logs.length === 0 ? (
            <div className="text-szz-subtext-0 italic py-4 text-center select-none font-pixel">
              No console logs recorded yet. Press [Play Game] to initiate launch pipeline.
            </div>
          ) : (
            logs.map((log) => (
              <div
                key={log.id}
                className="flex items-start gap-2.5 leading-relaxed hover:bg-szz-surface-0/60 px-1 py-0.5 rounded"
              >
                <span className="text-szz-subtext-0 shrink-0 text-[10px]">
                  {log.timestamp}
                </span>
                <span className="shrink-0">{renderBadge(log.level)}</span>
                <span className="break-all whitespace-pre-wrap">{log.text}</span>
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
};
