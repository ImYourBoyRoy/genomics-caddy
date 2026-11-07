<!-- ./src/lib/components/ai/ChatSettingsDrawer.svelte -->
<script lang="ts">
  import "#lib/styles/components/chat-settings-drawer.css";
  import type { ChatProvider } from "../../utils/chatProvider";
  import ConnectionModelSection from "./settings/ConnectionModelSection.svelte";
  import InferenceSettingsSection from "./settings/InferenceSettingsSection.svelte";
  import GenomicContextSection from "./settings/GenomicContextSection.svelte";
  import VectorResearchSettingsSection from "./settings/VectorResearchSettingsSection.svelte";
  import type { VectorResearchDiagnostics } from "../../types/research";
  import SystemPromptSection from "./settings/SystemPromptSection.svelte";
  import BiohackingProfileSection from "./BiohackingProfileSection.svelte";
  import type { AiContextMode, ConsultationMode } from "../../utils/aiPrompt";
  import type { ProfileContext } from "../../utils/profileContext";

  interface UserBiohackingProfile {
    goals: string;
    challenges: string;
    relevantBodySystems: string;
    reproductiveHormoneContext: string;
    diet: string;
    supplements: string;
    medications: string;
    bloodwork: string;
    diagnoses: string;
    supportiveTests: string;
    injectProfile: boolean;
  }

  interface Props {
    chatProvider?: ChatProvider;
    onChatProviderChange?: (provider: ChatProvider) => void;
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
    contextStats: {
      included: number;
      total: number;
      estimatedTokens?: number;
      notices?: string[];
      dropCounts?: Partial<Record<string, number>>;
      budgetNotice?: string;
    };
    userProfile: UserBiohackingProfile;
    profileContext: ProfileContext;
    onOpenContext?: () => void;
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
    chatProvider = "ollama",
    onChatProviderChange,
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
    profileContext,
    onOpenContext,
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
      {chatProvider}
      {onChatProviderChange}
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
          <BiohackingProfileSection
            bind:userProfile={userProfile}
            {profileContext}
            {onOpenContext}
          />
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
