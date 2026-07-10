<!-- ./src/lib/components/ai/ChatSettingsDrawer.svelte -->
<script lang="ts">
  import ConnectionModelSection from "./settings/ConnectionModelSection.svelte";
  import InferenceSettingsSection from "./settings/InferenceSettingsSection.svelte";
  import GenomicContextSection from "./settings/GenomicContextSection.svelte";
  import VectorResearchSettingsSection from "./settings/VectorResearchSettingsSection.svelte";
  import type { VectorResearchDiagnostics } from "../../types/research";
  import SystemPromptSection from "./settings/SystemPromptSection.svelte";
  import BiohackingProfileSection from "./BiohackingProfileSection.svelte";
  import type { AiContextMode, ConsultationMode } from "../../utils/aiPrompt";

  interface UserBiohackingProfile {
    goals: string;
    challenges: string;
    diet: string;
    supplements: string;
    medications: string;
    bloodwork: string;
    diagnoses: string;
    supportiveTests: string;
    injectProfile: boolean;
  }

  interface Props {
    ollamaUrl: string;
    ollamaToken: string;
    selectedModel: string;
    models: string[];
    isScanning: boolean;
    scanError: string;
    scanModels: () => void;
    modelDetails: any;
    isVisionCapable: boolean;
    contextWindow: number;
    temperature: number;
    maxTokens: number;
    extendedThinking: boolean;
    sessionPromptTokens: number;
    sessionResponseTokens: number;
    sessionTotalTokens: number;
    selectedPacks: Record<string, boolean>;
    onlyActiveFindings: boolean;
    contextStats: { included: number; total: number };
    userProfile: UserBiohackingProfile;
    systemInstructions: string;
    defaultInstructions: string;
    showThinkingProcess: boolean;
    autoCollapseThinking: boolean;
    includeTraceInExport: boolean;
    contextMode: AiContextMode;
    consultationMode: ConsultationMode;
    reviewModel: string;
    twoModelReview: boolean;
    useVectorResearch: boolean;
    vectorDiagnostics: VectorResearchDiagnostics | null;
    vectorDiagnosticsLoading?: boolean;
    refreshVectorDiagnostics: () => void;
  }

  let {
    ollamaUrl = $bindable(),
    ollamaToken = $bindable(),
    selectedModel = $bindable(),
    models,
    isScanning,
    scanError,
    scanModels,
    modelDetails,
    isVisionCapable,
    contextWindow,
    temperature = $bindable(),
    maxTokens = $bindable(),
    extendedThinking = $bindable(),
    sessionPromptTokens,
    sessionResponseTokens,
    sessionTotalTokens,
    selectedPacks = $bindable(),
    onlyActiveFindings = $bindable(),
    contextStats,
    userProfile = $bindable(),
    systemInstructions = $bindable(),
    defaultInstructions,
    showThinkingProcess = $bindable(),
    autoCollapseThinking = $bindable(),
    includeTraceInExport = $bindable(),
    contextMode = $bindable(),
    consultationMode = $bindable(),
    reviewModel = $bindable(),
    twoModelReview = $bindable(),
    useVectorResearch = $bindable(true),
    vectorDiagnostics = null,
    vectorDiagnosticsLoading = false,
    refreshVectorDiagnostics,
  }: Props = $props();
</script>

<aside class="settings-drawer">
  <div class="drawer-inner">
    
    <!-- SECTION 1: Connection & Model -->
    <ConnectionModelSection
      bind:ollamaUrl
      bind:ollamaToken
      bind:selectedModel
      {models}
      {isScanning}
      {scanError}
      {scanModels}
      {modelDetails}
      {isVisionCapable}
      {contextWindow}
      bind:reviewModel
      bind:twoModelReview
    />

    {#if models.length > 0}
      <!-- SECTION 2: Inference Settings -->
      <InferenceSettingsSection
        bind:temperature
        bind:maxTokens
        bind:extendedThinking
        bind:showThinkingProcess
        bind:autoCollapseThinking
        bind:includeTraceInExport
        bind:contextMode
        bind:consultationMode
        {sessionPromptTokens}
        {sessionResponseTokens}
        {sessionTotalTokens}
        {contextWindow}
        {selectedModel}
      />

      <!-- SECTION 3: Genomic Data Context -->
      <GenomicContextSection
        bind:selectedPacks
        bind:onlyActiveFindings
        {contextStats}
      />

      <VectorResearchSettingsSection
        bind:useVectorResearch
        {vectorDiagnostics}
        diagnosticsLoading={vectorDiagnosticsLoading}
        refreshDiagnostics={refreshVectorDiagnostics}
      />

      <!-- SECTION 4: Biohacking & Health Profile -->
      <details class="settings-details-group">
        <summary class="settings-details-summary">👤 Biohacking &amp; Health Profile</summary>
        <div class="settings-details-content">
          <BiohackingProfileSection bind:userProfile={userProfile} />
        </div>
      </details>

      <!-- SECTION 5: Custom System Prompt -->
      <SystemPromptSection
        bind:systemInstructions
        {defaultInstructions}
      />
    {/if}
  </div>
</aside>

<style>
  .settings-drawer {
    flex: 0 0 288px;
    display: flex;
    flex-direction: column;
    background: rgba(10, 11, 20, 0.45);
    border-left: 1px solid var(--border-color);
    box-sizing: border-box;
    height: 100%;
    overflow-y: auto;
    padding: 12px;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  }

  .drawer-inner {
    width: 264px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  /* Shared accordion styles — applied to child sections via :global */
  :global(.settings-details-group) {
    border: 1px solid var(--border-color);
    border-radius: 8px;
    background: rgba(255, 255, 255, 0.01);
    margin-bottom: 8px;
    overflow: hidden;
  }

  :global(.settings-details-group[open]) {
    background: rgba(255, 255, 255, 0.02);
  }

  :global(.settings-details-summary) {
    padding: 10px 12px;
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--text-primary);
    cursor: pointer;
    user-select: none;
    background: rgba(0, 0, 0, 0.15);
    list-style: none;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  :global(.settings-details-summary::-webkit-details-marker) {
    display: none;
  }

  :global(.settings-details-summary::after) {
    content: "▶";
    font-size: 0.65rem;
    transition: transform 0.2s;
    opacity: 0.7;
  }

  :global(.settings-details-group[open] > .settings-details-summary::after) {
    transform: rotate(90deg);
  }

  :global(.settings-details-content) {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
</style>
