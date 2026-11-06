import React from "react";
import { AccountProfile } from "../../types/auth";
import { ChevronDown } from "lucide-react";
import { PlayerAvatar } from "./PlayerAvatar";

interface AccountHeaderProps {
  activeAccount: AccountProfile | null;
  isOpen: boolean;
  onToggle: () => void;
}

export const AccountHeader: React.FC<AccountHeaderProps> = ({
  activeAccount,
  isOpen,
  onToggle,
}) => {
  const isMicrosoft = activeAccount?.account_type === "microsoft";

  return (
    <button
      onClick={onToggle}
      className={`w-full flex items-center gap-3 px-2 py-1.5 rounded-xl border border-transparent transition-colors cursor-pointer select-none text-left ${
        isOpen
          ? "bg-szz-surface-1 text-szz-text"
          : "hover:bg-szz-surface-0/60 text-szz-subtext-1"
      }`}
      title="Manage accounts"
    >
      <div className="w-10 h-10 rounded-lg flex items-center justify-center shrink-0 bg-szz-surface-2">
        <PlayerAvatar
          uuid={activeAccount?.uuid}
          username={activeAccount?.username}
          isMicrosoft={isMicrosoft}
          className="w-6 h-6 rounded"
          iconClassName="w-5 h-5 text-szz-subtext-0"
        />
      </div>

      <div className="flex flex-col min-w-0 opacity-0 max-w-0 overflow-hidden group-hover:opacity-100 group-hover:max-w-xs transition-all duration-300 whitespace-nowrap">
        <span className="text-xs font-bold text-szz-text truncate max-w-[130px] font-pixel">
          {activeAccount ? activeAccount.username : "Offline Player"}
        </span>
        <div className="flex items-center gap-1.5 mt-0.5">
          {isMicrosoft ? (
            <span className="text-[9px] font-bold px-1.5 py-0.2 rounded bg-szz-matcha/20 text-szz-matcha border border-szz-matcha/40 font-pixel">
              MSA
            </span>
          ) : (
            <span className="text-[9px] font-bold px-1.5 py-0.2 rounded bg-szz-surface-1 text-szz-subtext-1 border border-szz-border font-pixel">
              DEV
            </span>
          )}
          <span className="text-[10px] text-szz-subtext-0">Account</span>
        </div>
      </div>

      <ChevronDown
        className={`w-4 h-4 text-szz-subtext-0 ml-auto shrink-0 transition-all duration-200 hidden group-hover:block ${
          isOpen ? "rotate-180 text-szz-text" : ""
        }`}
      />
    </button>
  );
};
