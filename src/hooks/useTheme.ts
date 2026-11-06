import { useState, useEffect, useCallback } from "react";
import { ThemeMode } from "../types/theme";

const STORAGE_KEY = "szz_theme";
const DEFAULT_THEME: ThemeMode = "latte";

export function useTheme() {
  const [theme, setThemeState] = useState<ThemeMode>(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved === "latte" || saved === "dark") {
        return saved;
      }
    } catch {
      // ignore localStorage security / access error
    }
    return DEFAULT_THEME;
  });

  const setTheme = useCallback((newTheme: ThemeMode) => {
    setThemeState(newTheme);
    try {
      localStorage.setItem(STORAGE_KEY, newTheme);
    } catch {
      // ignore localStorage security / access error
    }
    document.documentElement.setAttribute("data-theme", newTheme);
  }, []);

  const toggleTheme = useCallback(() => {
    setTheme(theme === "latte" ? "dark" : "latte");
  }, [theme, setTheme]);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
  }, [theme]);

  return { theme, setTheme, toggleTheme };
}
