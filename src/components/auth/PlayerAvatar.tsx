import React from "react";
import { User } from "lucide-react";

interface PlayerAvatarProps {
  uuid?: string;
  username?: string;
  isMicrosoft: boolean;
  className?: string;
  iconClassName?: string;
}

export const PlayerAvatar: React.FC<PlayerAvatarProps> = ({
  uuid,
  username,
  isMicrosoft,
  className = "w-6 h-6 rounded",
  iconClassName = "w-5 h-5",
}) => {
  const finalIconClass = iconClassName.includes("text-")
    ? iconClassName
    : `${iconClassName} text-szz-subtext-0`;

  return (
    <div className="flex items-center justify-center shrink-0">
      {isMicrosoft && uuid ? (
        <img
          src={`https://minotar.net/helm/${uuid}/28.png`}
          alt={username || "Player avatar"}
          className={`${className} object-cover`}
          onError={(e) => {
            const target = e.currentTarget;
            if (!target.src.includes("crafthead.net")) {
              target.src = `https://crafthead.net/helm/${uuid}/28`;
            } else {
              target.style.display = "none";
            }
          }}
        />
      ) : (
        <User className={finalIconClass} />
      )}
    </div>
  );
};
