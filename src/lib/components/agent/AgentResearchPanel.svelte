<!-- ./src/lib/components/agent/AgentResearchPanel.svelte -->
<script lang="ts">
  import { onMount } from "svelte";
  import { scanOllamaModels, streamOllamaChat, saveReportJson, chatOllama } from "../../api/tauri";
  import type { ChatMessage } from "../../types/agent";
  import type { GenomeSample, GeneratedReport } from "../../types/genomics";
  import { dialogStore } from "../../utils/dialogState.svelte";
  import { isReasoningModel, filterChatModels, autoSelectModel } from "../../utils/aiPrompt";
  import { newOllamaStreamId, subscribeOllamaStream } from "../../utils/ollamaStream";
  import {
    type AgentStep, type LocalGenotypeResult, type NcbiData, type PubMedArticle,
    type ClinicalTrial, type ChemblDrug, fetchLocalGenotype, resolveGeneToRsids,
    resolveTopicToRsids, fetchNcbiDbsnpAndClinvar, fetchPubMedArticles,
    fetchClinicalTrials, fetchChemblDrugs, buildCritiquePrompt, buildSynthesisPrompt,
    buildValidationPrompt
  } from "../../utils/agentApis";
  import AgentRunner from "./AgentRunner.svelte";
  import AgentReportView from "./AgentReportView.svelte";

  interface Props {
    selectedSample: GenomeSample | null;
    generatedReport: GeneratedReport | null;
    ollamaUrl: string;
    ollamaToken: string;
    selectedModel: string;
  }

  let {
    selectedSample,
    generatedReport,
    ollamaUrl = $bindable(),
    ollamaToken = $bindable(),
    selectedModel = $bindable()
  }: Props = $props();

  // State Machine
  let runState = $state<"setup" | "running" | "result">("setup");
  let queryText = $state("");
  let queryType = $state<"rsid" | "gene" | "topic">("rsid");
  let isScanning = $state(false);
  let scanError = $state("");
  let models = $state<string[]>([]);
  let showThinking = $state(true);

  // Agent Loop Variables
  let steps = $state<AgentStep[]>([]);
  let currentStepIndex = $state(0);
  let progressPercent = $state(0);
  let reportText = $state("");
  let activeRsid = $state("");
  let collectedStats = $state({
    pubmedCount: 0,
    trialsCount: 0,
    drugsCount: 0,
    clinvarMatch: false,
    localMatch: false
  });
  let validationResult = $state<any>(null);

  // Quick launch findings
  let quickFindings = $derived(
    generatedReport?.sections
      ? generatedReport.sections
          .flatMap(s => s.markers || [])
          .filter(m => m.user_genotype !== "--" && !m.user_genotype.includes("-"))
          .sort((a, b) => {
            const getRank = (sc: string) =>
              sc === "high_risk" ? 0 : sc === "moderate_risk" ? 1 : 2;
            return getRank(a.severity_class) - getRank(b.severity_class);
          })
          .slice(0, 12)
      : []
  );

  onMount(async () => {
    await scanModels();
  });

  async function scanModels() {
    isScanning = true;
    scanError = "";
    try {
      models = filterChatModels(await scanOllamaModels(ollamaUrl, ollamaToken || undefined));
      selectedModel = autoSelectModel(models, selectedModel);
    } catch (e: any) {
      scanError = `Scan failed: ${e.message || e}`;
    } finally {
      isScanning = false;
    }
  }

  function parseJsonSafely(text: string): any {
    try {
      const match = text.match(/\{[\s\S]*\}/);
      if (match) return JSON.parse(match[0]);
      return JSON.parse(text);
    } catch (e) {
      console.error("Agent JSON parse failed", text, e);
      return null;
    }
  }

  async function callOllamaNonStreamed(prompt: string): Promise<string> {
    return chatOllama(
      ollamaUrl,
      ollamaToken || undefined,
      selectedModel,
      [{ role: "user", content: prompt }],
      0.0
    );
  }

  async function runAgent() {
    if (!queryText.trim() || !selectedSample) return;
    if (!selectedModel) {
      dialogStore.alert("Please select an Ollama model first.");
      return;
    }

    validationResult = null;
    runState = "running";
    currentStepIndex = 0;
    progressPercent = 0;

    steps = [
      { id: "resolve", label: "Resolve Query & Fetch Genotypes", status: "running" },
      { id: "ncbi", label: "Query NCBI dbSNP & ClinVar", status: "idle" },
      { id: "pubmed", label: "Search PubMed Articles", status: "idle" },
      { id: "trials", label: "Query ClinicalTrials.gov", status: "idle" },
      { id: "critique", label: "Perform Self-Critique & Gap Analysis (LLM)", status: "idle" },
      { id: "healing", label: "Execute Self-Healing Queries", status: "idle" },
      { id: "synthesis", label: "Synthesize Personal Report (LLM Stream)", status: "idle" },
      { id: "validation", label: "Execute Quality Audit Validation (LLM)", status: "idle" }
    ];

    const inputQuery = queryText.trim();
    let localResult: LocalGenotypeResult | null = null;
    let ncbiData: NcbiData | null = null;
    let pubmedArticles: PubMedArticle[] = [];
    let trials: ClinicalTrial[] = [];
    let drugs: ChemblDrug[] = [];
    let critiqueText = "";
    let healedTrials: ClinicalTrial[] = [];
    let healedDrugs: ChemblDrug[] = [];

    const advanceStep = (index: number, status: AgentStep["status"], msg: string | undefined = undefined) => {
      steps[index].status = status;
      if (msg) steps[index].message = msg;
      currentStepIndex = index + 1;
      if (currentStepIndex < steps.length) {
        steps[currentStepIndex].status = "running";
      }
      progressPercent = Math.round((currentStepIndex / steps.length) * 100);
      steps = [...steps];
    };

    try {
      // Step 1: Resolve Query
      let rsidsToQuery: string[] = [];
      if (queryType === "rsid") {
        rsidsToQuery = [inputQuery];
      } else if (queryType === "gene") {
        rsidsToQuery = await resolveGeneToRsids(inputQuery, generatedReport);
        if (rsidsToQuery.length === 0) {
          rsidsToQuery = [inputQuery]; // fallback
        }
      } else {
        rsidsToQuery = await resolveTopicToRsids(inputQuery, ollamaUrl, ollamaToken);
      }

      const mainRsid = rsidsToQuery[0] || inputQuery;
      activeRsid = mainRsid;
      localResult = await fetchLocalGenotype(selectedSample.id, mainRsid, generatedReport);
      advanceStep(0, "success", `Resolved target ${mainRsid}. Genotype: ${localResult.genotype}. Gene: ${localResult.gene || "Unknown"}`);

      // Step 2: NCBI ClinVar/dbSNP
      ncbiData = await fetchNcbiDbsnpAndClinvar(mainRsid);
      advanceStep(1, "success", `ClinVar significance: ${ncbiData.clinicalSignificance}. Position: Chr ${ncbiData.chromosome}:${ncbiData.position}`);

      // Step 3: PubMed
      pubmedArticles = await fetchPubMedArticles(mainRsid);
      advanceStep(2, "success", `Retrieved ${pubmedArticles.length} recent PubMed publications`);

      // Step 4: ClinicalTrials.gov
      trials = await fetchClinicalTrials(mainRsid, localResult.gene);
      advanceStep(3, "success", `Found ${trials.length} active/recruiting trials`);

      // Step 5: Self-Critique & Gap Analysis (LLM)
      drugs = await fetchChemblDrugs(localResult.gene);
      const critiquePrompt = buildCritiquePrompt(mainRsid, localResult, ncbiData, pubmedArticles, trials, drugs);
      critiqueText = await callOllamaNonStreamed(critiquePrompt);
      const critiqueObj = parseJsonSafely(critiqueText);
      const critiqueMsg = critiqueObj 
        ? `Audited strand warning: ${critiqueObj.strandWarning}. Identified gaps: ${critiqueObj.missingGaps?.join(", ") || "none"}`
        : "Completed validation critique audit";
      advanceStep(4, "success", critiqueMsg);

      // Step 6: Self-Healing & Refinement
      if (critiqueObj?.recommendedGeneHealing && critiqueObj.recommendedGeneHealing !== "none") {
        const geneTarget = critiqueObj.recommendedGeneHealing;
        if (trials.length === 0) {
          healedTrials = await fetchClinicalTrials(mainRsid, geneTarget);
        }
        if (drugs.length === 0) {
          healedDrugs = await fetchChemblDrugs(geneTarget);
        }
        advanceStep(5, "success", `Self-healed gaps. Refined queries completed for gene: ${geneTarget}`);
      } else {
        advanceStep(5, "success", "No gaps identified for healing. Query scope verified.");
      }

      // Step 7: Synthesis (LLM Stream)
      const synthesisPrompt = buildSynthesisPrompt(
        mainRsid, localResult, ncbiData, pubmedArticles, trials, drugs, critiqueText, healedTrials, healedDrugs
      );

      collectedStats = {
        pubmedCount: pubmedArticles.length,
        trialsCount: Math.max(trials.length, healedTrials.length),
        drugsCount: Math.max(drugs.length, healedDrugs.length),
        clinvarMatch: ncbiData.clinicalSignificance !== "No ClinVar annotation",
        localMatch: localResult.genotype !== "Not found in raw DNA file"
      };

      let stopStream: (() => void) | null = null;
      const synthesisStreamId = newOllamaStreamId();

      try {
        stopStream = await subscribeOllamaStream(synthesisStreamId, {
          onChunk: (chunk) => {
            reportText += chunk;
          },
          onDone: () => {},
        });

        const payload: ChatMessage[] = [
          { role: "system", content: "You are an AI research scientist." },
          { role: "user", content: synthesisPrompt },
        ];
        await streamOllamaChat(
          synthesisStreamId,
          ollamaUrl,
          ollamaToken || undefined,
          selectedModel,
          payload,
          0.2,
          4096,
        );
      } finally {
        stopStream?.();
      }
      advanceStep(6, "success", "Draft report synthesis streamed and compiled successfully.");

      // Step 8: Quality Validation (LLM)
      const valPrompt = buildValidationPrompt(reportText);
      const valText = await callOllamaNonStreamed(valPrompt);
      const valObj = parseJsonSafely(valText);
      validationResult = valObj ?? {
        medicalClaimingFree: false,
        disclaimerPresent: false,
        structureOk: false,
        auditComments: "Quality audit returned invalid JSON — report blocked pending manual review.",
        approved: false
      };
      advanceStep(7, "success", `Quality check results: Approved=${validationResult.approved}. Notes: ${validationResult.auditComments}`);

      // All finished!
      progressPercent = 100;
      setTimeout(() => {
        runState = "result";
      }, 1500);

    } catch (err: any) {
      console.error(err);
      if (currentStepIndex < steps.length) {
        steps[currentStepIndex].status = "error";
        steps[currentStepIndex].message = err.message || String(err);
      }
      dialogStore.alert("Agent loop failed: " + (err.message || String(err)));
    }
  }

  function launchQuickFind(rsid: string, gene: string) {
    queryText = rsid;
    queryType = "rsid";
    runAgent();
  }

  function handleExport() {
    const cleanName = (selectedSample ? selectedSample.name : "genome").replace(/[^a-zA-Z0-9]/g, "_");
    saveReportJson(reportText, `${cleanName}_agent_report_${activeRsid}_${Date.now()}.md`)
      .then(ok => {
        if (ok) dialogStore.alert("Agent report saved successfully!");
      })
      .catch(e => dialogStore.alert("Failed to export: " + e.message));
  }
</script>

<div class="agent-panel-wrapper">
  {#if runState === "setup"}
    <div class="setup-grid">
      <!-- Left Config Card -->
      <div class="setup-config card">
        <h3>🕵️ Configure Genomics Research Agent</h3>
        <p class="card-hint">
          Research variants, genes, or health topics dynamically. The agent will check SQLite, search ClinVar, PubMed, and ClinicalTrials before synthesizing a Personal Research Report.
        </p>

        <form onsubmit={(e) => { e.preventDefault(); runAgent(); }} style="display: flex; flex-direction: column; gap: 16px;">
          <!-- Target selector -->
          <div class="input-row">
            <label for="query-type-selector">Query Target Type</label>
            <select id="query-type-selector" bind:value={queryType}>
              <option value="rsid">rsID (Single Variant)</option>
              <option value="gene">Gene Symbol (e.g., MTHFR)</option>
              <option value="topic">Health Topic (RAG Semantic Search)</option>
            </select>
          </div>

          <!-- Query Input -->
          <div class="input-row">
            <label for="query-input">Search Term</label>
            <input
              id="query-input"
              type="text"
              bind:value={queryText}
              placeholder={queryType === "rsid" ? "e.g. rs1801133" : queryType === "gene" ? "e.g. MTHFR" : "e.g. caffeine metabolism"}
              required
            />
          </div>

          <!-- Ollama Model selector -->
          <div class="input-row">
            <label for="agent-model-selector">Agent LLM Model</label>
            <select id="agent-model-selector" bind:value={selectedModel}>
              {#each models as m}
                <option value={m}>{m}</option>
              {/each}
            </select>
            {#if isScanning}
              <span class="scanning-text font-mono">Scanning Ollama...</span>
            {/if}
          </div>

          <p style="font-size: 0.65rem; color: var(--text-secondary); margin: 0 0 12px; line-height: 1.35;">
            NCBI API key (optional) is configured in AI Settings → Advanced → Vector Research.
          </p>

          <button class="btn btn-primary w-full" type="submit" disabled={isScanning || !queryText.trim()}>
            🕵️ Start Agentic Research
          </button>
        </form>
      </div>

      <!-- Right Quick Launch Card -->
      <div class="setup-quick-launch card">
        <h3>🧬 Quick Launch Report Findings</h3>
        <p class="card-hint">
          Click any active marker found in your Trait Report to launch a complete agentic research investigation.
        </p>

        {#if quickFindings.length === 0}
          <div class="empty-quick-list">
            No active findings detected. Please import a genome first.
          </div>
        {:else}
          <div class="findings-grid">
            {#each quickFindings as f}
              <button class="finding-shortcut-btn" onclick={() => launchQuickFind(f.rsid, f.gene)}>
                <div class="shortcut-header">
                  <span class="rsid font-mono">{f.rsid}</span>
                  <span class="gene font-bold">{f.gene}</span>
                </div>
                <div class="shortcut-body">
                  <span class="genotype font-mono">Genotype: {f.user_genotype}</span>
                  <span class="severity-tag {f.severity_class}">{f.severity_class.replace("_", " ")}</span>
                </div>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {:else if runState === "running"}
    <AgentRunner {steps} {currentStepIndex} {progressPercent} />
  {:else if runState === "result"}
    <AgentReportView
      {reportText}
      modelName={selectedModel}
      rsid={activeRsid}
      stats={collectedStats}
      validation={validationResult}
      onReset={() => runState = "setup"}
      onExport={handleExport}
    />
  {/if}
</div>

<style src="../../styles/components/agent-research-panel.css"></style>
