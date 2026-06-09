<!-- ./src/lib/components/ai/WelcomeChat.svelte -->
<div class="welcome-chat">
  <div class="welcome-icon">💬</div>
  <h3>Local Genomics Assistant</h3>
  <p>This session uses a local, private Large Language Model to interpret genetic data safely on your machine. None of your genomic variants ever leave your computer.</p>
  <p class="hint">Select a curated prompt below or ask a custom question to begin.</p>
</div>

<style>
  .welcome-chat {
    margin: auto;
    max-width: 460px;
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    color: var(--text-secondary);
    padding: 20px;
  }

  .welcome-icon {
    font-size: 3rem;
  }

  .welcome-chat h3 {
    color: var(--text-primary);
    font-size: 1.25rem;
    margin: 0;
  }

  .welcome-chat p {
    font-size: 0.88rem;
    line-height: 1.5;
    margin: 0;
  }

  .welcome-chat .hint {
    font-size: 0.8rem;
    opacity: 0.7;
    margin-top: 10px;
  }
</style>
