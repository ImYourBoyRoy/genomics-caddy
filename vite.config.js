import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";
import {
  LOOPBACK_HOST,
  VITE_DEV_ENDPOINT_NAME,
  VITE_DEV_SERVICE,
  loopbackUrl,
  writeLoopbackEndpoint,
} from "./scripts/lib/agentUiEndpoint.mjs";

const host = process.env.TAURI_DEV_HOST || LOOPBACK_HOST;
const vitePort = Number.parseInt(process.env.GENOMICS_VITE_PORT || "1420", 10);
const isProductionBuild = process.argv.includes("build");

const sveltekitOptions = {
  adapter: adapter({ fallback: "index.html" }),
  preprocess: vitePreprocess(),
  // Production emits component CSS as files so Tauri's CSP nonce can style
  // components. In dev, keep injection to avoid Vite 8 virtual-CSS races.
  emitCss: isProductionBuild,
};

function genomicsLoopbackEndpointPlugin() {
  return {
    name: "genomics-loopback-endpoint",
    /** @param {import('vite').ViteDevServer} server */
    configureServer(server) {
      let attached = false;
      const publish = () => {
        const address = server.httpServer?.address();
        if (!address || typeof address === "string") return;
        void writeLoopbackEndpoint(VITE_DEV_ENDPOINT_NAME, {
          service: VITE_DEV_SERVICE,
          bind: LOOPBACK_HOST,
          port: address.port,
          url: loopbackUrl(address.port),
          pid: process.pid,
        });
      };
      const attach = () => {
        const http = server.httpServer;
        if (!http) return;
        if (attached) {
          if (http.listening) publish();
          return;
        }
        attached = true;
        if (http.listening) publish();
        else http.once("listening", publish);
      };
      attach();
      return attach;
    },
  };
}

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [sveltekit(sveltekitOptions), genomicsLoopbackEndpointPlugin()],

  build: {
    // Desktop webview loads local assets; slightly higher than Vite's 500 kB default
    // keeps the warning useful without false alarms on the main shell chunk.
    chunkSizeWarningLimit: 900,
    rolldownOptions: {
      output: {
        // SvelteKit owns its client runtime split. Keep only the independent
        // Tauri and research groups so its dynamic runtime entry stays intact.
        codeSplitting: {
          groups: [
            {
              name: "vendor-tauri",
              test: /node_modules[\\/]@tauri-apps[\\/]/,
            },
            {
              name: "feature-research",
              test: /src[\\/]lib[\\/]components[\\/]research[\\/]/,
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
    port: Number.isInteger(vitePort) && vitePort > 0 ? vitePort : 1420,
    strictPort: true,
    host,
    hmr: {
      protocol: "ws",
      host,
    },
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
