import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const overlaySource = readFileSync(resolve(process.cwd(), 'src/lib/components/common/bootstrap/BootstrapOverlay.svelte'), 'utf8');
const screenSource = readFileSync(resolve(process.cwd(), 'src/lib/components/common/AppBootstrapScreen.svelte'), 'utf8');
const bootstrapSource = readFileSync(resolve(process.cwd(), 'src/lib/utils/pageBootstrap.ts'), 'utf8');
const appHtmlSource = readFileSync(resolve(process.cwd(), 'src/app.html'), 'utf8');
const themeSource = readFileSync(resolve(process.cwd(), 'src/lib/styles/theme.css'), 'utf8');

describe('desktop bootstrap screen', () => {
  it('keeps startup intentionally dark and independent from the selected report theme', () => {
    expect(overlaySource).toContain('color-scheme: dark;');
    expect(overlaySource).toContain('--bg-primary: #080b14;');
    expect(overlaySource).toContain('linear-gradient(135deg, #070a12 0%, #0d1424 54%, #0a1718 100%)');
    expect(overlaySource).not.toContain('var(--bg-primary));');
  });

  it('gives the startup details a focused panel instead of spreading them across the screen', () => {
    expect(screenSource).toContain('max-width: 38rem;');
    expect(screenSource).toContain('border-radius: 20px;');
    expect(screenSource).toContain('background: #0c1220;');
    expect(screenSource).toContain('animation: none;');
    expect(screenSource).toContain('style="opacity:1;visibility:visible;color:#eef2ff;background:#0c1220;"');
    expect(screenSource).toContain('<BootstrapActivityPulse');
    expect(screenSource).not.toContain('class="live-status"');
    expect(screenSource).not.toContain('backdrop-filter:');
    expect(screenSource).not.toContain('animation: bootstrap-fade-up');
  });

  it('keeps the DNA helix to the right behind the status panel', () => {
    const helixSource = readFileSync(
      resolve(process.cwd(), 'src/lib/components/common/bootstrap/BootstrapHelixBackdrop.svelte'),
      'utf8',
    );
    expect(screenSource).toContain('class="bootstrap-helix-slot"');
    expect(screenSource).toContain('place-items: center;');
    expect(helixSource).toContain('class="helix-anchor"');
    expect(helixSource).toContain('class="helix-stage"');
    expect(helixSource).toContain('right: clamp(1.5rem, 7vw, 8rem);');
    expect(helixSource).toContain('width: min(34vw, 28rem);');
    expect(helixSource).toContain('animation: bootstrap-helix-drift 9s ease-in-out infinite;');
  });

  it('keeps startup status text visible with inline styles instead of a second overlay card', () => {
    expect(overlaySource).not.toContain('bootstrap-status-strip');
    expect(overlaySource).not.toContain('hideHeadline={true}');
    expect(screenSource).toContain('Starting Genomics Caddy');
    expect(screenSource).toContain('message={activityMessage}');
  });

  it('ships a static first-paint loader layout for native WebKit', () => {
    expect(themeSource).toContain('.bootstrap-overlay .bootstrap-screen');
    expect(themeSource).toContain('place-items: center;');
    expect(themeSource).toContain('text-align: center;');
    expect(themeSource).toContain('.bootstrap-overlay .phase-orb .logo');
    expect(themeSource).toContain('width: 44px;');
    expect(themeSource).toContain('.bootstrap-overlay .helix-anchor');
    expect(themeSource).toContain('right: clamp(1.5rem, 7vw, 8rem);');
    expect(themeSource).toContain('width: min(34vw, 28rem);');
    expect(themeSource).toContain('animation: bootstrap-helix-drift 9s ease-in-out infinite;');
  });

  it('keeps the loader reachable when display scaling reduces the CSS viewport height', () => {
    expect(themeSource).toContain('overflow-y: auto;');
    expect(themeSource).toContain('min-height: 100dvh;');
    expect(themeSource).toContain('@media screen and (max-height: 800px)');
    expect(themeSource).toContain('place-items: start center;');
    expect(screenSource).toContain('overflow: visible;');
    expect(screenSource).toContain('flex-wrap: wrap;');
  });

  it('keeps DNA import confirmation inside the waterfall until cancel or confirm', () => {
    const pageSource = readFileSync(resolve(process.cwd(), 'src/routes/+page.svelte'), 'utf8');
    expect(pageSource).toContain('importPhase === "error" && !importOverlayDismissed');
    expect(pageSource).toContain('importPhase = "idle"');
    expect(pageSource).toContain('isImportPreparing || isImporting || importPhase === "awaiting-confirmation"');
    expect(pageSource).toContain('<ImportWorkspaceState');
  });

  it('keeps the native first paint dark without replacing the detailed bootstrap screen', () => {
    expect(appHtmlSource).toContain('background: #080b14;');
    expect(appHtmlSource).not.toContain('startup-shell');
    expect(appHtmlSource).not.toContain('animation:');
  });

  it('keeps startup checkpoints visible while the native database status is loading', () => {
    expect(bootstrapSource).toContain('const ticker = setInterval');
    expect(bootstrapSource).toContain('callbacks.onPhase("db", DB_TICKER_MESSAGES[tickerIdx]);');
    expect(bootstrapSource).toContain('await bootstrapSleep(450);');
    expect(bootstrapSource).toContain('await bootstrapSleep(700);');
    expect(screenSource).toContain('let elapsedSeconds = $state(0);');
    expect(screenSource).toContain('setInterval(() =>');
    expect(screenSource).toContain('formatEta(progressInfo.etaSeconds)');
    expect(screenSource).toContain('import a genome whenever you’re ready.');
    expect(bootstrapSource).toContain('Ready — import a genome to begin.');
  });

  it('keeps the existing detailed loader available for catalog synchronization', () => {
    const pageSource = readFileSync(resolve(process.cwd(), 'src/routes/+page.svelte'), 'utf8');
    const sidebarSource = readFileSync(resolve(process.cwd(), 'src/lib/components/sidebar/Sidebar.svelte'), 'utf8');

    expect(pageSource).toContain('isBootstrapping ||');
    expect(pageSource).toContain('(isResourceSyncing && !resourceSyncOverlayDismissed)');
    expect(pageSource).toContain('onResourceSyncStateChange');
    expect(pageSource).toContain('resourceSyncOverlayDismissed');
    expect(sidebarSource).toContain('runSyncAllMissing({ showOverlay: true })');
    expect(sidebarSource).toContain("message: 'Syncing missing reference resources…'");
    expect(screenSource).toContain('Continue in workspace');
    expect(overlaySource).toContain('mode === "resources"');
    expect(overlaySource).toContain('Reference resource synchronization');
  });

  it('keeps DNA import progress in the same detailed waterfall', () => {
    const pageSource = readFileSync(resolve(process.cwd(), 'src/routes/+page.svelte'), 'utf8');
    const importSource = readFileSync(resolve(process.cwd(), 'src/lib/utils/pageSampleHandlers.ts'), 'utf8');

    expect(pageSource).toContain('showImportOverlay');
    expect(pageSource).toContain('importProgress={importOverlayProgress}');
    expect(importSource).toContain('await selectSample(newSample)');
    expect(importSource).toContain('importPhase: "report"');
    expect(importSource).toContain('importPhase: "ready"');
    expect(screenSource).toContain('mode === "import"');
    expect(screenSource).toContain('<ImportStepTimeline');
    expect(overlaySource).toContain('mode === "import"');
  });

  it('keeps reference disclosure labels inside narrow finding cards', () => {
    const sourcesSource = readFileSync(resolve(process.cwd(), 'src/lib/components/report/SourcesList.svelte'), 'utf8');
    expect(sourcesSource).toContain('display: flex;');
    expect(sourcesSource).toContain('width: 100%;');
    expect(sourcesSource).toContain('max-width: 100%;');
    expect(sourcesSource).toContain('.marker-sources-details summary::-webkit-details-marker');
    expect(sourcesSource).toContain('class="marker-sources-summary-label"');
    expect(sourcesSource).toContain('min-width: 0;');
    expect(sourcesSource).toContain('overflow: hidden;');
  });
});
