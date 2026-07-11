// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  // Inject component <style> into JS instead of virtual `?svelte&type=style&lang.css`
  // modules. Vite 8 + vite-plugin-svelte can race those virtual CSS loads and log
  // "failed to load virtual css module" (often serving JS for the CSS URL).
  vitePlugin: {
    emitCss: false,
  },
  kit: {
    adapter: adapter({
      fallback: "index.html",
    }),
  },
};

export default config;
