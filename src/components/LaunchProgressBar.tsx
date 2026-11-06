import React from "react";
import { LaunchStep, ProgressPayload } from "../types/launcher";
import { Loader2, CheckCircle2 } from "lucide-react";

interface LaunchProgressBarProps {
  progress: ProgressPayload | null;
  isActive: boolean;
  isCompleted: boolean;
  title?: string;
}

const LAUNCH_STEPS = [
  { num: 1, label: "Instance" },
  { num: 2, label: "Runtime" },
  { num: 3, label: "Mods" },
  { num: 4, label: "Assets" },
  { num: 5, label: "Classpath" },
  { num: 6, label: "Ready" },
] as const;

const STEP_TITLES: Record<LaunchStep, string> = {
  preparingInstance: "Preparing Instance Directories",
  checkingJavaRuntime: "Validating Isolated Java Runtime",
  syncingMods: "Syncing Content-Addressable Mods",
  assemblingClasspath: "Assembling Classpath & Assets",
  spawningClient: "Launching Minecraft Client",
  complete: "Pipeline Completed",
};

export const LaunchProgressBar: React.FC<LaunchProgressBarProps> = ({
  progress,
  isActive,
  isCompleted,
  title,
}) => {
  if (!isActive && !isCompleted && !progress) {
    return null;
  }

  const percentage = isCompleted
    ? 100
    : Math.min(100, Math.max(5, progress?.percentage ?? 10));

  const stepNumber = progress?.stepNumber ?? (isCompleted ? 6 : 1);
  const totalSteps = progress?.totalSteps ?? 6;
  const message = isCompleted
    ? "Synchronization completed successfully!"
    : progress?.message ?? "Preparing sync pipeline...";

  const stepTitle = progress?.step ? STEP_TITLES[progress.step] : "Processing";

  return (
    <div className="w-full bg-szz-surface-0/95 border border-szz-border rounded-xl p-4 shadow-xl backdrop-blur-sm select-none">
      <div className="flex items-center justify-between text-xs mb-2">
        <div className="flex items-center gap-2">
          {isActive ? (
            <Loader2 className="w-4 h-4 text-szz-matcha animate-spin shrink-0" />
          ) : isCompleted ? (
            <CheckCircle2 className="w-4 h-4 text-szz-matcha shrink-0" />
          ) : null}
          <span className="font-bold text-szz-text font-pixel tracking-wide">
            {title
              ? title
              : isCompleted
              ? "Completed"
              : `Step ${stepNumber}/${totalSteps}: ${stepTitle}`}
          </span>
        </div>
        <span className="font-mono font-bold text-szz-text text-xs">
          {Math.round(percentage)}%
        </span>
      </div>

      <div className="w-full h-2.5 bg-szz-surface-1 rounded-full overflow-hidden p-[1px] border border-szz-border">
        <div
          className={`h-full rounded-full transition-all duration-300 ease-out ${
            isCompleted
              ? "bg-szz-matcha"
              : "bg-gradient-to-r from-szz-bamboo to-szz-matcha animate-pulse"
          }`}
          style={{ width: `${percentage}%` }}
        />
      </div>

      <div className="mt-2 text-xs text-szz-subtext-0 truncate font-pixel">
        {message}
      </div>

      <div className="mt-3 pt-3 border-t border-szz-border/50 grid grid-cols-6 gap-1.5">
        {LAUNCH_STEPS.map((s) => {
          const isCurrent = s.num === stepNumber && isActive;
          const isDone = s.num < stepNumber || isCompleted;

          return (
            <div
              key={s.num}
              className={`flex items-center justify-center gap-1 px-1 py-1 rounded text-[10px] font-pixel truncate ${
                isCurrent
                  ? "bg-szz-matcha/20 border border-szz-matcha/40 text-szz-text font-bold"
                  : isDone
                  ? "bg-szz-matcha/15 text-szz-matcha"
                  : "text-szz-subtext-0/50"
              }`}
            >
              {isDone ? (
                <CheckCircle2 className="w-3 h-3 text-szz-matcha shrink-0" />
              ) : isCurrent ? (
                <span className="w-1.5 h-1.5 rounded-full bg-szz-matcha animate-ping shrink-0" />
              ) : null}
              <span className="truncate">{s.label}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
};
