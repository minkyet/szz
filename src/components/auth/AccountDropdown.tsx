import React, { useState, useRef, useEffect } from "react";
import { AccountProfile } from "../../types/auth";
import { Check, Trash2, Plus, Terminal, Sun, Moon } from "lucide-react";
import { PlayerAvatar } from "./PlayerAvatar";
import { ThemeMode } from "../../types/theme";

interface AccountDropdownProps {
  accounts: AccountProfile[];
  onSwitchAccount: (id: string) => void;
  onRemoveAccount: (id: string) => void;
  onCreateDevAccount: (username: string) => void;
  onOpenLoginModal: () => void;
  onClose: () => void;
  theme?: ThemeMode;
  onToggleTheme?: () => void;
}

export const AccountDropdown: React.FC<AccountDropdownProps> = ({
  accounts,
  onSwitchAccount,
  onRemoveAccount,
  onCreateDevAccount,
  onOpenLoginModal,
  onClose,
  theme,
  onToggleTheme,
}) => {
  const [showDevInput, setShowDevInput] = useState(false);
  const [devUsername, setDevUsername] = useState("");
  const dropdownRef = useRef<HTMLDivElement>(null);
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        onCloseRef.current();
      }
    };
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") onCloseRef.current();
    };

    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, []);

  const handleCreateDev = (e: React.FormEvent) => {
    e.preventDefault();
    if (!devUsername.trim()) return;
    onCreateDevAccount(devUsername.trim());
    setDevUsername("");
    setShowDevInput(false);
  };

  return (
    <div
      ref={dropdownRef}
      className="absolute bottom-16 left-2 w-72 bg-szz-base border border-szz-border rounded-xl shadow-2xl p-2 z-50 text-szz-text animate-in fade-in zoom-in-95 duration-100 select-none"
    >
      <div className="px-2.5 py-1.5 text-[11px] font-bold text-szz-subtext-1 uppercase tracking-wider border-b border-szz-border mb-1 flex items-center justify-between font-pixel">
        <span>Accounts ({accounts.length})</span>
        {onToggleTheme && (
          <button
            onClick={onToggleTheme}
            className="flex items-center gap-1 text-[10px] text-szz-subtext-1 hover:text-szz-text px-1.5 py-0.5 rounded hover:bg-szz-surface-1 cursor-pointer transition-colors"
            title="Toggle Theme"
          >
            {theme === "dark" ? (
              <>
                <Sun className="w-3 h-3 text-szz-amber" />
                <span>Dark</span>
              </>
            ) : (
              <>
                <Moon className="w-3 h-3 text-szz-matcha" />
                <span>Latte</span>
              </>
            )}
          </button>
        )}
      </div>

      <div className="max-h-56 overflow-y-auto space-y-1 py-1 custom-scrollbar">
        {accounts.length === 0 ? (
          <div className="text-center py-4 text-xs text-szz-subtext-0">
            No accounts configured
          </div>
        ) : (
          accounts.map((acc) => {
            const isSelected = acc.is_active;
            const isMs = acc.account_type === "microsoft";

            return (
              <div
                key={acc.id}
                onClick={() => onSwitchAccount(acc.id)}
                className={`flex items-center justify-between px-2.5 py-2 rounded-lg cursor-pointer transition-colors group ${
                  isSelected
                    ? "bg-szz-matcha/15 border border-szz-matcha/40"
                    : "hover:bg-szz-surface-1 border border-transparent"
                }`}
              >
                <div className="flex items-center gap-2.5 min-w-0">
                  <PlayerAvatar
                    uuid={acc.uuid}
                    username={acc.username}
                    isMicrosoft={isMs}
                    className="w-6 h-6"
                    iconClassName="w-3.5 h-3.5"
                  />

                  <div className="flex flex-col min-w-0">
                    <span className="text-xs font-semibold truncate text-szz-text">
                      {acc.username}
                    </span>
                    <span className="text-[10px] text-szz-subtext-0 flex items-center gap-1">
                      {isMs ? "Microsoft Account" : "Dev Offline Profile"}
                    </span>
                  </div>
                </div>

                <div className="flex items-center gap-1 shrink-0">
                  {isSelected && (
                    <Check className="w-4 h-4 text-szz-matcha mr-1" />
                  )}
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      onRemoveAccount(acc.id);
                    }}
                    className="p-1 rounded opacity-0 group-hover:opacity-100 hover:bg-szz-berry/20 hover:text-szz-berry text-szz-subtext-0 transition-all"
                    title="Remove Account"
                  >
                    <Trash2 className="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>
            );
          })
        )}
      </div>

      <div className="border-t border-szz-border mt-1 pt-1.5 space-y-1">
        <button
          onClick={onOpenLoginModal}
          className="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-xs font-medium text-szz-matcha hover:bg-szz-matcha/10 transition-colors cursor-pointer"
        >
          <Plus className="w-3.5 h-3.5" />
          <span>Add Microsoft Account</span>
        </button>

        {!showDevInput ? (
          <button
            onClick={() => setShowDevInput(true)}
            className="w-full flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-xs font-medium text-szz-subtext-1 hover:text-szz-text hover:bg-szz-surface-1 transition-colors cursor-pointer"
          >
            <Terminal className="w-3.5 h-3.5" />
            <span>Add Offline Dev Profile</span>
          </button>
        ) : (
          <form onSubmit={handleCreateDev} className="p-1.5 space-y-1.5">
            <input
              type="text"
              placeholder="Dev Username..."
              value={devUsername}
              onChange={(e) => setDevUsername(e.target.value)}
              className="w-full px-2 py-1 text-xs bg-szz-surface-0 border border-szz-border rounded text-szz-text focus:outline-none focus:border-szz-matcha"
              autoFocus
            />
            <div className="flex justify-end gap-1">
              <button
                type="button"
                onClick={() => setShowDevInput(false)}
                className="px-2 py-0.5 text-[11px] text-szz-subtext-1 hover:text-szz-text"
              >
                Cancel
              </button>
              <button
                type="submit"
                className="px-2.5 py-0.5 text-[11px] bg-szz-matcha text-szz-action-play-text font-medium rounded hover:opacity-90 cursor-pointer"
              >
                Create
              </button>
            </div>
          </form>
        )}
      </div>
    </div>
  );
};
