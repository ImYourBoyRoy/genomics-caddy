import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [sveltekit()],

  build: {
    // Desktop webview loads local assets; slightly higher than Vite's 500 kB default
    // keeps the warning useful without false alarms on the main shell chunk.
    chunkSizeWarningLimit: 900,
    rolldownOptions: {
      output: {
        // Split heavy vendor / feature surfaces so the first paint chunk stays leaner.
        codeSplitting: {
          groups: [
            {
              name: "vendor-svelte",
              test: /node_modules[\\/](svelte|@sveltejs)[\\/]/,
            },
            {
              name: "vendor-tauri",
              test: /node_modules[\\/]@tauri-apps[\\/]/,
            },
            {
              name: "feature-ai",
              test: /src[\\/]lib[\\/]components[\\/]ai[\\/]/,
            },
            {
              name: "feature-research",
              test: /src[\\/]lib[\\/]components[\\/]research[\\/]/,
            },
            {
              name: "feature-evidence",
              test: /src[\\/]lib[\\/]components[\\/]ai[\\/]evidence[\\/]/,
            },
          ],
        },
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
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
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
