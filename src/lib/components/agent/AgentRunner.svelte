<!-- ./src/lib/components/agent/AgentRunner.svelte -->
<script lang="ts">
  import { type AgentStep } from "../../utils/agentApis";

  interface Props {
    steps: AgentStep[];
    currentStepIndex: number;
    progressPercent: number;
  }

  let { steps, currentStepIndex, progressPercent }: Props = $props();

  function getStepIcon(status: AgentStep["status"]): string {
    switch (status) {
      case "success": return "🟢";
      case "error": return "🔴";
      case "running": return "🔄";
      default: return "⚪";
    }
  }
</script>

<div class="agent-runner-container">
  <div class="runner-card">
    <div class="runner-header">
      <div class="title-area">
        <span class="pulse-beacon"></span>
        <h3>Genomics Agent Research Loop Active</h3>
      </div>
      <span class="percentage font-mono">{progressPercent}%</span>
    </div>

    <!-- Progress bar -->
    <div class="progress-bar-bg">
      <div class="progress-bar-fill" style="width: {progressPercent}%"></div>
    </div>

    <!-- Terminal Output -->
    <div class="agent-terminal">
      <div class="terminal-bar">
        <span class="dot red"></span>
        <span class="dot yellow"></span>
        <span class="dot green"></span>
        <span class="terminal-title font-mono">agent-terminal@genomics-caddy</span>
      </div>
      <div class="terminal-body font-mono">
        {#each steps as step, idx}
          <div class="terminal-line" class:running={step.status === "running"} class:success={step.status === "success"} class:error={step.status === "error"}>
            <span class="step-icon">{getStepIcon(step.status)}</span>
            <span class="timestamp">[{new Date().toLocaleTimeString()}]</span>
            <span class="label">{step.label}</span>
            {#if step.message}
              <div class="terminal-message font-mono">└─ {step.message}</div>
            {/if}
          </div>
        {/each}
        {#if currentStepIndex < steps.length && steps[currentStepIndex]?.status === "running"}
          <div class="terminal-prompt">
            <span class="cursor-block">_</span>
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .agent-runner-container {
    max-width: 800px;
    margin: 40px auto;
  }
  .runner-card {
    background: rgba(255, 255, 255, 0.02);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 24px;
    backdrop-filter: blur(20px);
  }
  .runner-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
  }
  .title-area {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .pulse-beacon {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #06b6d4;
    box-shadow: 0 0 8px #06b6d4;
    animation: beacon-glow 1.5s infinite ease-in-out;
  }
  @keyframes beacon-glow {
    0%, 100% { opacity: 0.5; transform: scale(0.9); }
    50% { opacity: 1; transform: scale(1.15); box-shadow: 0 0 14px #06b6d4; }
  }
  .percentage {
    color: #06b6d4;
    font-weight: 700;
  }
  .progress-bar-bg {
    background: rgba(255, 255, 255, 0.05);
    height: 6px;
    border-radius: 3px;
    overflow: hidden;
    margin-bottom: 24px;
  }
  .progress-bar-fill {
    background: linear-gradient(90deg, #06b6d4 0%, var(--accent) 100%);
    height: 100%;
    border-radius: 3px;
    transition: width 0.4s ease;
  }
  .agent-terminal {
    background: #08090d;
    border: 1px solid rgba(6, 182, 212, 0.2);
    border-radius: 8px;
    overflow: hidden;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
  }
  .terminal-bar {
    background: #11131c;
    padding: 8px 12px;
    display: flex;
    align-items: center;
    gap: 6px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }
  .dot.red { background: #ef4444; }
  .dot.yellow { background: #eab308; }
  .dot.green { background: #22c55e; }
  .terminal-title {
    font-size: 0.65rem;
    color: var(--text-secondary);
    margin-left: 8px;
  }
  .terminal-body {
    padding: 16px;
    min-height: 240px;
    max-height: 380px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .terminal-line {
    font-size: 0.8rem;
    color: var(--text-secondary);
    line-height: 1.4;
  }
  .terminal-line.running {
    color: #06b6d4;
    animation: pulse-line 1.5s infinite ease-in-out;
  }
  @keyframes pulse-line {
    0%, 100% { opacity: 0.8; }
    50% { opacity: 1; }
  }
  .terminal-line.success { color: var(--text-primary); }
  .terminal-line.error { color: var(--danger); }
  .timestamp {
    color: rgba(255, 255, 255, 0.2);
    margin-right: 6px;
  }
  .terminal-message {
    padding-left: 24px;
    font-size: 0.75rem;
    color: #67e8f9;
    margin-top: 4px;
    opacity: 0.85;
  }
  .terminal-prompt {
    display: flex;
    align-items: center;
  }
  .cursor-block {
    width: 6px;
    height: 12px;
    background: #06b6d4;
    display: inline-block;
    animation: cursor-blink 1s infinite steps(2, start);
  }
  @keyframes cursor-blink {
    0%, 100% { opacity: 0; }
    50% { opacity: 1; }
  }
  .font-mono { font-family: monospace; }
</style>
