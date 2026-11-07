<!-- ./src/routes/__helix-preview/+page.svelte -->
<script lang="ts">
  /*
    Dev-only visual comparison for the Nov 2 and Nov 4 sequencing helix motion.
    Open with: pnpm run dev → http://127.0.0.1:5173/__helix-preview
  */
  import SequencingHelix from "#lib/components/common/loading/SequencingHelix.svelte";
</script>

<svelte:head>
  <title>Helix motion preview · Genomics Caddy</title>
  <meta name="robots" content="noindex, nofollow" />
</svelte:head>

{#if !import.meta.env.DEV}
  <main class="preview-gate">
    <h1>Helix preview unavailable</h1>
    <p>This visual comparison is available in the Vite development server.</p>
    <a href="/">Return home</a>
  </main>
{:else}
  <main class="preview-page">
    <header>
      <p class="eyebrow">Animation comparison</p>
      <h1>Sequencing helix</h1>
      <p>Same nodes, rungs, size, and colors. The left panel restores the Nov 2 motion; the right panel uses the Nov 4 motion.</p>
    </header>

    <section class="comparison" aria-label="Helix animation comparison">
      <article class="sample sample--nov2">
        <h2>Nov 2 · smooth alternating motion</h2>
        <p>Independent, sine-eased sway and depth animations.</p>
        <SequencingHelix percentage={52} />
      </article>

      <article class="sample sample--nov4">
        <h2>Nov 4 · single path animation</h2>
        <p>Today’s four-stage transform path and rung motion.</p>
        <SequencingHelix percentage={52} />
      </article>

    </section>

    <footer>Both previews loop continuously. Reduce-motion settings are respected by the shared helix stylesheet.</footer>
  </main>
{/if}

<style>
  :global(html),
  :global(body) {
    min-width: 320px;
    min-height: 100%;
    margin: 0;
    background: #080b14;
  }

  .preview-page {
    min-height: 100dvh;
    box-sizing: border-box;
    padding: clamp(1rem, 4vw, 3rem);
    color: #eef2ff;
    background:
      radial-gradient(ellipse at 50% 0%, rgba(45, 212, 191, 0.12), transparent 55%),
      #080b14;
    font-family: Inter, ui-sans-serif, system-ui, sans-serif;
  }

  header {
    max-width: 48rem;
    margin: 0 auto 1.5rem;
    text-align: center;
  }

  .eyebrow {
    margin: 0 0 0.4rem;
    color: #5eead4;
    font-size: 0.75rem;
    font-weight: 750;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  h1 {
    margin: 0;
    font-size: clamp(1.7rem, 4vw, 2.6rem);
  }

  header > p:last-child {
    margin: 0.7rem auto 0;
    color: #b7c4d8;
    line-height: 1.55;
  }

  .comparison {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 1rem;
    width: min(100%, 76rem);
    margin: 0 auto;
  }

  .sample {
    display: flex;
    min-height: min(70vh, 38rem);
    flex-direction: column;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    padding: 1.25rem;
    border: 1px solid rgba(148, 163, 184, 0.25);
    border-radius: 1rem;
    background: linear-gradient(155deg, rgba(19, 26, 43, 0.96), rgba(10, 19, 30, 0.96));
  }

  .sample h2 {
    margin: 0;
    color: #eef2ff;
    font-size: clamp(0.95rem, 2vw, 1.25rem);
    text-align: center;
  }

  .sample > p {
    min-height: 2.8em;
    margin: 0.45rem 0 1rem;
    color: #aebbd0;
    font-size: 0.85rem;
    line-height: 1.4;
    text-align: center;
  }

  .sample :global(.sequencing-helix) {
    --helix-width: clamp(9rem, 22vw, 15rem);
    --helix-amp: calc(var(--helix-width) * 0.42);
    --helix-node: clamp(0.6rem, 1.3vw, 0.95rem);
    --helix-period: 6s;
    --helix-row-gap: 0.17;
    --accent: #2dd4bf;
    --success: #34d399;
    --text-muted: #7dd3fc;
    height: min(58vh, 31rem);
    min-height: 18rem;
  }

  /* Nov 2 motion: the pre-Nov 4 smooth alternating translate/scale tracks. */
  .sample--nov2 :global(.helix-node) {
    --helix-sine: cubic-bezier(0.37, 0, 0.63, 1);
    animation:
      helix-preview-sway calc(var(--helix-period) / 2) var(--helix-sine) infinite alternate,
      helix-preview-depth calc(var(--helix-period) / 2) var(--helix-sine) infinite alternate;
    animation-delay:
      var(--helix-row-phase),
      calc(var(--helix-row-phase) - var(--helix-period) / 4);
  }

  .sample--nov2 :global(.helix-node-b) {
    animation-direction: alternate-reverse;
  }

  .sample--nov2 :global(.helix-rung) {
    animation-name: helix-preview-rung;
  }

  /* Keep the Nov 4 comparison intact after the production helix is restored. */
  .sample--nov4 :global(.helix-node) {
    animation: helix-preview-path var(--helix-period) linear infinite;
    animation-delay: var(--helix-row-phase);
  }

  .sample--nov4 :global(.helix-node-b) {
    animation-direction: normal;
    animation-delay: calc(var(--helix-row-phase) - var(--helix-period) / 2);
  }

  .sample--nov4 :global(.helix-rung) {
    animation-name: helix-preview-rung-path;
  }

  @keyframes helix-preview-sway {
    from { translate: calc(var(--helix-amp) * -1) 0; }
    to { translate: var(--helix-amp) 0; }
  }

  @keyframes helix-preview-depth {
    from { scale: 0.65; opacity: 0.35; }
    to { scale: 1.35; opacity: 1; }
  }

  @keyframes helix-preview-rung {
    from { scale: 1 1; }
    to { scale: 0.04 1; }
  }

  @keyframes helix-preview-path {
    0% {
      transform: translateX(calc(var(--helix-amp) * -1)) scale(0.95);
      opacity: 0.72;
      animation-timing-function: var(--helix-sine);
    }
    25% {
      transform: translateX(0) scale(1.35);
      opacity: 1;
      animation-timing-function: var(--helix-sine);
    }
    50% {
      transform: translateX(var(--helix-amp)) scale(0.95);
      opacity: 0.72;
      animation-timing-function: var(--helix-sine);
    }
    75% {
      transform: translateX(0) scale(0.65);
      opacity: 0.35;
      animation-timing-function: var(--helix-sine);
    }
    100% {
      transform: translateX(calc(var(--helix-amp) * -1)) scale(0.95);
      opacity: 0.72;
    }
  }

  @keyframes helix-preview-rung-path {
    from { transform: scaleX(1); }
    to { transform: scaleX(0.04); }
  }

  footer {
    width: min(100%, 76rem);
    margin: 1rem auto 0;
    color: #8796ad;
    font-size: 0.75rem;
    text-align: center;
  }

  .preview-gate {
    min-height: 100dvh;
    display: grid;
    place-content: center;
    gap: 0.75rem;
    padding: 2rem;
    color: #e2e8f0;
    background: #080b14;
    text-align: center;
  }

  @media (max-width: 720px) {
    .comparison { grid-template-columns: minmax(0, 1fr); }
    .sample { min-height: 27rem; }
    .sample :global(.sequencing-helix) { height: 20rem; }
  }
</style>
