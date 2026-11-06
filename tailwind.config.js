/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        szz: {
          // Tier 1: Foundation tokens (Catppuccin-Inspired Matcha Palette)
          crust: "var(--szz-crust)",
          mantle: "var(--szz-mantle)",
          base: "var(--szz-base)",
          surface: {
            DEFAULT: "var(--szz-surface-0)",
            0: "var(--szz-surface-0)",
            1: "var(--szz-surface-1)",
            2: "var(--szz-surface-2)",
          },
          overlay: {
            DEFAULT: "var(--szz-overlay-0)",
            0: "var(--szz-overlay-0)",
            1: "var(--szz-overlay-1)",
            2: "var(--szz-overlay-2)",
          },
          text: "var(--szz-text)",
          subtext: {
            DEFAULT: "var(--szz-subtext-1)",
            1: "var(--szz-subtext-1)",
            0: "var(--szz-subtext-0)",
          },
          matcha: "var(--szz-matcha)",
          teaCream: "var(--szz-tea-cream)",
          bamboo: "var(--szz-bamboo)",
          amber: "var(--szz-amber)",
          berry: "var(--szz-berry)",
          mauve: "var(--szz-mauve)",

          // Tier 2: szz Component Semantic Aliases
          chassis: "var(--szz-base)",
          sidebar: "var(--szz-crust)",
          sidebarIcon: "var(--szz-sidebar-icon-bg)",
          "sidebar-icon": "var(--szz-sidebar-icon-bg)",
          border: "var(--szz-overlay-0)",
          borderSubtle: "var(--szz-overlay-0)",
          "border-subtle": "var(--szz-overlay-0)",
          borderActive: "var(--szz-overlay-2)",
          "border-active": "var(--szz-overlay-2)",
          actionPlay: "var(--szz-matcha)",
          "action-play": "var(--szz-matcha)",
          actionPlayText: "var(--szz-action-play-text)",
          "action-play-text": "var(--szz-action-play-text)",
          actionInstall: "var(--szz-surface-2)",
          "action-install": "var(--szz-surface-2)",
          actionInstallText: "var(--szz-matcha)",
          "action-install-text": "var(--szz-matcha)",
          statusOnline: "var(--szz-matcha)",
          "status-online": "var(--szz-matcha)",
          statusMaintenance: "var(--szz-amber)",
          "status-maintenance": "var(--szz-amber)",
          statusOffline: "var(--szz-berry)",
          "status-offline": "var(--szz-berry)",
          error: "var(--szz-berry)",
        },
      },
      fontFamily: {
        pixel: ["'MonaS12'", "monospace", "sans-serif"],
        "pixel-sm": ["'MonaS10x12'", "monospace", "sans-serif"],
        mono: ["'D2Coding'", "Consolas", "monospace"],
      },
      spacing: {
        "sidebar": "72px",
        "sidebar-expanded": "260px",
      },
      borderRadius: {
        xs: "2px",
        sm: "4px",
        md: "6px",
        lg: "10px",
      }
    },
  },
  plugins: [],
}
