// ./src/routes/__loading-lab/+page.ts
/*
Purpose: Keep the loading-lab route client-only and out of static prerender.
The page itself refuses to render outside Vite DEV so production Tauri
bundles never expose scripted loader fixtures as a product surface.
*/

export const ssr = false;
export const prerender = false;
