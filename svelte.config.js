// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

// Production builds must emit component CSS as files: Tauri adds a nonce to the
// bundled CSP `style-src`, which makes browsers ignore 'unsafe-inline' and drop
// the <style> tags Svelte injects at runtime. The dev server keeps injection
// because Vite 8 + vite-plugin-svelte can race virtual `?svelte&type=style`
// CSS loads and log "failed to load virtual css module".
const isProductionBuild = process.argv.includes("build");

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  vitePlugin: {
    emitCss: isProductionBuild,
  },
  kit: {
    adapter: adapter({
      fallback: "index.html",
    }),
  },
};

export default config;
