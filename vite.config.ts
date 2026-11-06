import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => {
  const isWeb = mode === "web";

  return {
    plugins: [react()],
    build: {
      outDir: isWeb ? "dist-web" : "dist",
      emptyOutDir: true,
    },
    clearScreen: false,
    server: isWeb
      ? {
          port: 3000,
        }
      : {
          port: 1420,
          strictPort: true,
          host: host || false,
          hmr: host
            ? {
                protocol: "ws",
                host,
                port: 1421,
              }
            : undefined,
          watch: {
            ignored: ["**/src-tauri/**"],
          },
        },
  };
});
