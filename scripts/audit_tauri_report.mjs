#!/usr/bin/env node
/*
Purpose: Exercise the running Tauri desktop report through its local UI bridge.
How to run: `npm run audit:tauri-ui` while `npm run tauri:dev` is running.
Outputs: Aggregate-only desktop mode, geometry, sex-label, medication-surface, public-copy, and rendered-theme contrast checks.
Privacy: Never prints sample names, genotype calls, technical disclosures, or raw UI text.
*/

import { setTimeout as sleep } from "node:timers/promises";

const baseUrl = (process.env.GENOMICS_AGENT_UI_URL || "http://127.0.0.1:17321").replace(/\/$/, "");
const timeoutMs = parsePositiveInteger(process.env.GENOMICS_TAURI_AUDIT_TIMEOUT_MS, 120_000);
const pollMs = 500;
const requestedViewportWidth = parsePositiveInteger(process.env.GENOMICS_TAURI_AUDIT_WIDTH, 0);
const requestedViewportHeight = parsePositiveInteger(
  process.env.GENOMICS_TAURI_AUDIT_HEIGHT,
  requestedViewportWidth > 0 ? Math.max(720, Math.round(requestedViewportWidth * 0.5625)) : 0,
);

function parsePositiveInteger(value, fallback) {
  const parsed = Number.parseInt(value || "", 10);
  return Number.isInteger(parsed) && parsed > 0 ? parsed : fallback;
}

async function request(path, options = {}) {
  const response = await fetch(`${baseUrl}${path}`, {
    ...options,
    headers: {
      Accept: "application/json",
      ...(options.body ? { "Content-Type": "application/json" } : {}),
      ...(options.headers || {}),
    },
  });
  const body = await response.text();
  let payload;
  try {
    payload = body ? JSON.parse(body) : null;
  } catch {
    throw new Error(`Bridge returned non-JSON data for ${path}`);
  }
  if (!response.ok) {
    throw new Error(`Bridge request failed for ${path} (HTTP ${response.status})`);
  }
  return payload;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function layoutOf(snapshot) {
  assert(snapshot && typeof snapshot === "object", "Bridge snapshot is not an object");
  assert(snapshot.layout && typeof snapshot.layout === "object", "Bridge snapshot has no layout metrics");
  return snapshot.layout;
}

function validateDesktopSnapshot(snapshot, expectedMode) {
  assert(snapshot.hasReport === true, "Report is not ready");
  const sex = snapshot.sample?.genetic_sex;
  assert(sex === "Male" || sex === "Female", "Report did not expose a concise Male/Female sex label");

  const layout = layoutOf(snapshot);
  assert(layout.activePresentationMode === expectedMode, `Expected ${expectedMode} mode`);
  assert(layout.viewportWidth > 0 && layout.viewportHeight > 0, "Desktop viewport is unavailable");
  assert(layout.documentScrollWidth <= layout.documentClientWidth + 1, "Desktop document overflows horizontally");
  if (layout.mainContentWidth !== null && layout.mainContentScrollWidth !== null) {
    assert(
      layout.mainContentScrollWidth <= layout.mainContentWidth + 1,
      "Desktop report content overflows horizontally"
    );
  }
  if (layout.profileNameCount > 0) {
    assert(
      layout.profileNameMinWidth !== null && layout.profileNameMinWidth >= 80,
      "Active profile names do not have a readable sidebar text region",
    );
  }
  assert(layout.overflowingElements.length === 0, "Desktop bridge reported overflowing elements");
  assert(layout.focusControlOverlapsContent === false, "Focus Report control overlaps the first content block");
  if (layout.focusControlBottom !== null && layout.firstContentTop !== null) {
    assert(layout.focusControlBottom <= layout.firstContentTop, "Focus Report control is not above the first content block");
    assert(
      layout.firstContentTop - layout.focusControlBottom >= 4,
      "Focus Report control has no visual separation from the first content block",
    );
  }
  assert(layout.themeControlInSidebarFooter === true, "Theme control is not in the sidebar footer");
  assert(layout.themeModeOptionCount === 3, "Sidebar theme control does not expose Auto, Light, and Dark choices");
  assert(layout.topToolbarPresent === false, "Redundant top report toolbar is still present");
  assert(layout.themeMenuOpen === false, "Theme control still exposes a popover menu");
  assert(layout.appContextMenuInstalled === true, "Application context menu is not installed in the desktop app");
  assert(layout.sidebarFocusControlInsideSidebar === true, "Sidebar hide control is not contained by the sidebar");
  assert(layout.sidebarFocusControlOverlapsBrand === false, "Sidebar hide control overlaps the sidebar brand");
  if (layout.sidebarFocusControlTop !== null) {
    assert(layout.sidebarFocusControlTop <= 24, "Sidebar hide control is not anchored near the sidebar top");
  }
  if (layout.sidebarFocusControlRightGap !== null) {
    assert(layout.sidebarFocusControlRightGap >= 4 && layout.sidebarFocusControlRightGap <= 20, "Sidebar hide control is not aligned to the sidebar edge");
  }
  assert(layout.actionQueueItemCount <= 9, "Action queue exceeds the nine-item desktop contract");
  if (expectedMode === "simple" && layout.reportOverviewWidth !== null && layout.mainContentWidth !== null) {
    assert(
      layout.reportOverviewWidth >= Math.min(Math.round(layout.mainContentWidth * 0.9), 1100),
      "Simple report overview is not using the available desktop banner width",
    );
  }
  if (expectedMode === "simple" && layout.viewportWidth > 1400 && layout.actionQueueColumnCount !== null) {
    assert(layout.actionQueueColumnCount === 3, "Simple action queue is not using the three-column desktop layout");
  } else if (expectedMode === "simple" && layout.viewportWidth > 1100 && layout.actionQueueColumnCount !== null) {
    assert(layout.actionQueueColumnCount === 2, "Simple action queue is not using the two-column desktop layout");
  }
  if (layout.actionQueueWidth !== null && layout.mainContentWidth !== null) {
    assert(layout.actionQueueWidth < layout.mainContentWidth, "Action queue still stretches across the full report pane");
  }
  if (layout.actionQueueShellWidth !== null && layout.mainContentWidth !== null) {
    assert(layout.actionQueueShellWidth < layout.mainContentWidth, "Action queue shell still stretches across the full report pane");
  }
  if (layout.actionQueueWidth !== null && layout.actionQueueItemMaxWidth !== null) {
    assert(layout.actionQueueItemMaxWidth <= layout.actionQueueWidth + 1, "Action queue item exceeds its reading surface");
  }
  if (layout.dashboardGuidanceWidth !== null && layout.mainContentWidth !== null) {
    assert(layout.dashboardGuidanceWidth < layout.mainContentWidth, "Dashboard guidance still stretches across the full report pane");
  }
  if (layout.dashboardGuidanceWidth !== null && layout.dashboardGuidanceCardMaxWidth !== null) {
    assert(
      layout.dashboardGuidanceCardMaxWidth <= layout.dashboardGuidanceWidth + 1,
      "Dashboard guidance card exceeds its measured guidance surface",
    );
  }
  if (layout.connectionsLaunchHeight !== null) {
    assert(layout.connectionsLaunchHeight <= 72, "Advanced Connections launch surface is too tall for the default sidebar");
  }
  if (layout.liftoverStatusHeight !== null) {
    assert(layout.liftoverStatusHeight <= 72, "Healthy Liftover status surface is too tall for the default sidebar");
  }
  if (layout.markerGridWidth !== null && layout.mainContentWidth !== null) {
    assert(layout.markerGridWidth < layout.mainContentWidth, "Finding grid still stretches across the full report pane");
  }
  if (layout.markerGridColumnCount !== null) {
    assert(layout.markerGridColumnCount <= 2, "Report finding grid exceeds the two-column desktop contract");
  }
  if (
    layout.markerGridWidth !== null &&
    layout.markerGridColumnCount !== null &&
    layout.markerGridColumnCount > 0 &&
    layout.markerCardMaxWidth !== null
  ) {
    const gap = layout.markerGridColumnGap ?? 0;
    const expectedColumnWidth = (layout.markerGridWidth - gap * (layout.markerGridColumnCount - 1)) / layout.markerGridColumnCount;
    assert(
      layout.markerCardMaxWidth <= Math.ceil(expectedColumnWidth + 1),
      "Finding card exceeds its grid column and may be spanning or stretching unexpectedly",
    );
  }

  if (expectedMode === "clinical") {
    assert(layout.clinicalTableCount === 1, "Clinical mode did not render its structured findings table");
    assert(layout.clinicalProvenanceCount === 1, "Clinical mode did not render its single provenance disclosure");
    assert(layout.markerCardCount === 0, "Clinical mode rendered duplicate marker cards");
  } else {
    assert(layout.clinicalTableCount === 0, `${expectedMode} mode rendered a clinical table`);
    assert(layout.clinicalProvenanceCount === 0, `${expectedMode} mode rendered Clinical provenance content`);
  }

  const warnings = snapshot.warningMetrics;
  assert(warnings && typeof warnings === "object", "Desktop bridge returned no warning-routing metrics");
  assert(warnings.duplicateGenericWarningPhraseCount === 0, "Visible report repeats a generic warning phrase");
  assert(warnings.reportGuideCount <= 1, "Report contains more than one interpretation guide");
  assert(warnings.footerReminderCount <= 1, "Report contains more than one shared footer reminder");
  assert(warnings.legalPrivacyPageCount === 0, "Legal & Privacy page is visible inside the Trait Report");
  if (expectedMode === "simple") {
    assert(warnings.genericWarningPhraseCount <= 2, "Simple mode exposes a generic warning wall");
  }

  if (expectedMode === "simple" && layout.markerCardCount > 0) {
    const contract = snapshot.simpleCardContract;
    assert(contract && typeof contract === "object", "Simple card contract metrics are missing");
    for (const [key, label] of [
      ["titleCount", "title"],
      ["signalCount", "signal"],
      ["whyItMattersCount", "why-it-matters"],
      ["reviewActionCount", "review-action"],
      ["evidenceCount", "evidence"],
      ["detailsCount", "Details"],
      ["technicalDataCount", "Technical data"],
    ]) {
      assert(contract[key] === contract.cardCount, `Simple cards are missing a ${label} contract surface`);
    }
    // Follow-up is intentionally a section-level surface, not a repeated
    // per-card block. One section can cover many cards, and sections with no
    // concrete confirm_with route correctly have no follow-up line.
    assert(
      Number.isInteger(contract.sectionFollowUpCount) &&
        contract.sectionFollowUpCount >= 0 &&
        contract.sectionFollowUpCount <= contract.cardCount,
      "Simple section follow-up metrics are not aggregate and bounded",
    );
  }

  return {
    sex,
    markerCards: layout.markerCardCount,
    clinicalTables: layout.clinicalTableCount,
    clinicalProvenance: layout.clinicalProvenanceCount,
    columns: layout.markerGridColumnCount,
    gridWidth: layout.markerGridWidth,
    cardWidth: layout.markerCardMaxWidth,
    simpleCardContract: snapshot.simpleCardContract,
    warningMetrics: snapshot.warningMetrics,
    connectionsHeight: layout.connectionsLaunchHeight,
    liftoverHeight: layout.liftoverStatusHeight,
  };
}

function validateResourceStatus(snapshot) {
  assert(
    snapshot.resourceStatus === "checking" ||
      snapshot.resourceStatus === "current" ||
      snapshot.resourceStatus === "attention" ||
      snapshot.resourceStatus === "error",
    "Collapsed Data & updates row did not expose a settled or in-progress status category",
  );
  return snapshot.resourceStatus;
}

function validateResourceUpdatePhase(snapshot) {
  const phase = snapshot.resourceUpdatePhase;
  assert(
    phase === null || [
      "checking",
      "available",
      "downloading",
      "validating",
      "installed",
      "reloading",
      "ready",
      "attention",
      "error",
    ].includes(phase),
    "Desktop bridge returned an unknown resource update phase",
  );
  assert(phase !== "checking", "Resource update status remained in checking without reaching a terminal phase");
  if (snapshot.resourceStatus === "error") {
    assert(phase === "error", "Resource status error was not reflected by the update state machine");
  }
  return phase;
}

async function assertNoUnverifiedUpdateCopy(snapshot) {
  if (snapshot.resourceStatus !== "error" && snapshot.resourceStatus !== "attention") return;

  for (const text of ["newer remote", "Update available", "updates available"]) {
    const result = await request("/ui/queryText", {
      method: "POST",
      body: JSON.stringify({ text }),
    });
    assert(
      result?.ok === true && result.count === 0,
      `Incomplete resource verification exposed actionable update copy: ${text}`,
    );
  }
}

async function selectTheme(label, expectedMode) {
  await clickText(label);
  const snapshot = await waitForSnapshot(
    (candidate) => candidate.themeMode === expectedMode && candidate.layout?.themeMenuOpen === false,
    `${expectedMode} theme mode`,
  );
  assert(snapshot.themeMode === expectedMode, `Expected ${expectedMode} theme mode`);
  assert(snapshot.layout?.themeMenuOpen === false, `Expected ${expectedMode} theme selection to close`);
  return snapshot;
}

function validateContrastSnapshot(contrast, theme) {
  assert(contrast && typeof contrast === "object", `${theme} contrast probe returned no metrics`);
  assert(contrast.checkedPairCount > 0, `${theme} contrast probe checked no rendered pairs`);
  assert(
    contrast.passingPairCount === contrast.checkedPairCount && contrast.failingPairCount === 0,
    `${theme} contrast probe found a low-contrast rendered pair`,
  );
  assert(contrast.minimumRatio >= 4.5, `${theme} contrast minimum is below normal-text AA`);
  return contrast;
}

async function probeContrast(theme) {
  const contrast = await request("/ui/probeContrast", {
    method: "POST",
    body: JSON.stringify({}),
  });
  return validateContrastSnapshot(contrast, theme);
}

function validateTooltipSnapshot(snapshot) {
  const tooltip = snapshot.tooltip;
  assert(tooltip && typeof tooltip === "object", "Bridge snapshot has no tooltip metrics");
  assert(tooltip.openPanelCount === 1, "Expected one open tooltip panel");
  assert(tooltip.withinViewportCount === tooltip.openPanelCount, "Tooltip panel is outside the desktop viewport");
  assert(tooltip.accessiblePanelCount === tooltip.openPanelCount, "Open tooltip lacks accessible name/description metadata");
  assert(
    tooltip.maxLeftOverflow === 0 &&
      tooltip.maxTopOverflow === 0 &&
      tooltip.maxRightOverflow === 0 &&
      tooltip.maxBottomOverflow === 0,
    "Tooltip panel exceeds the desktop viewport",
  );
}

function validateAccessibilitySnapshot(snapshot) {
  const accessibility = snapshot.accessibility;
  assert(accessibility && typeof accessibility === "object", "Bridge snapshot has no accessibility metrics");
  assert(accessibility.mainLandmarkCount === 1, "Report does not expose one main landmark");
  assert(accessibility.sidebarLandmarkCount === 1, "Data sidebar landmark is missing or unlabeled");
  assert(accessibility.labelledToolbarCount === 1, "Report toolbar is missing an accessible label");
  assert(accessibility.labelledTablistCount >= 1, "Report has no labelled tab list");
  assert(accessibility.activeTabCount === 1, "Report does not expose one selected tab");
  assert(accessibility.labelledTabpanelCount === 1, "Active report panel is missing its accessible relationship");
  assert(accessibility.labelledActionQueueCount === 1, "Action queue is missing its accessible heading relationship");
  assert(
    accessibility.sectionToggleCount === accessibility.boundSectionToggleCount,
    "At least one report section toggle points to a missing panel",
  );
  assert(
    accessibility.guidanceToggleCount === accessibility.boundGuidanceToggleCount,
    "At least one dashboard guidance toggle points to a missing panel",
  );
  assert(
    accessibility.expandedControlCount === accessibility.boundExpandedControlCount,
    "At least one expanded-state control points to a missing panel",
  );
  assert(accessibility.focusControlTargetsSidebar === true, "Focus Report control does not target the data sidebar");
}

function validateFocusModeSnapshot(snapshot) {
  const layout = layoutOf(snapshot);
  assert(layout.focusMode === true, "Focus Report did not enter focus mode");
  assert(layout.sidebarWidth === 0, "Focus Report did not hide the data sidebar");
  assert(layout.mainContentWidth === layout.viewportWidth, "Focus Report did not use the available desktop width");
  assert(layout.focusControlOverlapsContent === false, "Focus Report overlaps report content in focus mode");
  assert(layout.overflowingElements.length === 0, "Focus mode introduced desktop overflow");
}

async function waitForSnapshot(predicate, label, waitMs = timeoutMs) {
  const started = Date.now();
  let lastError = null;
  while (Date.now() - started < waitMs) {
    try {
      const snapshot = await request("/ui/snapshot");
      if (predicate(snapshot)) return snapshot;
    } catch (error) {
      lastError = error;
    }
    await sleep(pollMs);
  }
  const detail = lastError ? ` (${lastError.message})` : "";
  throw new Error(`Timed out waiting for ${label}${detail}`);
}

async function prepareRequestedViewport() {
  if (!requestedViewportWidth) return;
  const result = await request("/ui/resize", {
    method: "POST",
    body: JSON.stringify({ width: requestedViewportWidth, height: requestedViewportHeight }),
  });
  assert(result?.ok === true, "Could not resize the Tauri window for the requested desktop audit width");
  await waitForSnapshot(
    (snapshot) => {
      const width = snapshot.layout?.viewportWidth;
      return Number.isFinite(width) && Math.abs(width - requestedViewportWidth) <= 80;
    },
    `${requestedViewportWidth}px desktop viewport`,
  );
}

async function clickText(text) {
  const result = await request("/ui/clickText", {
    method: "POST",
    body: JSON.stringify({ text }),
  });
  assert(result?.ok === true, `Could not activate ${text}`);
}

async function focusText(text) {
  const result = await request("/ui/focusText", {
    method: "POST",
    body: JSON.stringify({ text }),
  });
  assert(result?.ok === true, `Could not focus ${text}`);
}

async function pressKey(key) {
  const result = await request("/ui/pressKey", {
    method: "POST",
    body: JSON.stringify({ key }),
  });
  assert(result?.ok === true, `Could not press ${key}`);
}

async function waitForMode(mode) {
  return waitForSnapshot(
    (snapshot) => {
      const layout = snapshot.layout;
      if (layout?.activePresentationMode !== mode) return false;
      return mode === "clinical" ? layout.clinicalTableCount === 1 : layout.clinicalTableCount === 0;
    },
    `${mode} mode and settled report structure`,
  );
}

async function assertNoRedundantPublicCopy() {
  const result = await request("/ui/queryText", {
    method: "POST",
    body: JSON.stringify({ text: "Evidence sources available" }),
  });
  assert(result?.ok === true && result.count === 0, "Redundant generic evidence warning is visible in the report");

  const genericFollowUp = await request("/ui/queryText", {
    method: "POST",
    body: JSON.stringify({ text: "Review the suggested follow-up." }),
  });
  assert(
    genericFollowUp?.ok === true && genericFollowUp.count === 0,
    "Generic follow-up copy is visible instead of an authored next step",
  );

  const genericConfirmation = await request("/ui/queryText", {
    method: "POST",
    body: JSON.stringify({ text: "Consider clinical confirmation." }),
  });
  assert(
    genericConfirmation?.ok === true && genericConfirmation.count === 0,
    "Generic confirmation copy is visible instead of an authored confirmation step",
  );
}

async function assertAllergySurface() {
  const dnaContext = await request("/ui/queryText", {
    method: "POST",
    body: JSON.stringify({ text: "Matched pathways" }),
  });
  if (dnaContext?.ok === true && dnaContext.count === 0) {
    const toggle = await request("/ui/clickText", {
      method: "POST",
      body: JSON.stringify({ text: "Allergy & sensitivity map" }),
    });
    assert(toggle?.ok === true, "Could not expand the allergy and sensitivity map for verification");
    await sleep(100);
  }

  for (const text of [
    "Allergy & sensitivity map",
    "Matched pathways",
  ]) {
    const result = await request("/ui/queryText", {
      method: "POST",
      body: JSON.stringify({ text }),
    });
    assert(result?.ok === true && result.count > 0, `Allergy surface is missing ${text}`);
  }

  for (const text of [
    "Allergy & exposure",
    "Exposure history checklist",
    "almond, walnut, pecan, cashew, pistachio",
    "honeybee or bumblebee sting",
    "cold air, water, or cold objects",
    "sunlight or photosensitive rash",
  ]) {
    const result = await request("/ui/queryText", {
      method: "POST",
      body: JSON.stringify({ text }),
    });
    assert(result?.ok === true && result.count === 0, `Non-DNA allergy checklist content is visible: ${text}`);
  }

  const showMore = await request("/ui/queryText", {
    method: "POST",
    body: JSON.stringify({ text: "Show more" }),
  });
  assert(showMore?.ok === true && showMore.count === 0, "Allergy and dietary surfaces still use Show more truncation");
}

async function assertReportOrganization() {
  for (const text of [
    "Use your results",
    "Allergy & sensitivity",
    "Food & supplements",
    "Training & recovery",
    "Medication & clinical follow-up",
    "Explore all health areas",
  ]) {
    const result = await request("/ui/queryText", {
      method: "POST",
      body: JSON.stringify({ text }),
    });
    assert(result?.ok === true && result.count > 0, `Report organization is missing ${text}`);
  }
}

async function assertMedicationSurface() {
  const pathwayHeader = await request("/ui/queryText", {
    method: "POST",
    body: JSON.stringify({ text: "Medication pathways" }),
  });
  assert(pathwayHeader?.ok === true && pathwayHeader.count > 0, "Medication pathway surface is missing");

  let pathwayIntro = await request("/ui/queryText", {
    method: "POST",
    body: JSON.stringify({ text: "DNA-linked medication and treatment topics found in this report." }),
  });
  if (pathwayIntro?.ok === true && pathwayIntro.count === 0) {
    const toggle = await request("/ui/clickText", {
      method: "POST",
      body: JSON.stringify({ text: "Medication pathways" }),
    });
    assert(toggle?.ok === true, "Could not expand the medication pathway surface for verification");
    await sleep(100);
    pathwayIntro = await request("/ui/queryText", {
      method: "POST",
      body: JSON.stringify({ text: "DNA-linked medication and treatment topics found in this report." }),
    });
  }
  assert(pathwayIntro?.ok === true && pathwayIntro.count > 0, "Medication pathway surface has no DNA-linked intro");

  for (const text of ["Ask for / record", "Do not do from raw DNA", "PGx completeness check"]) {
    const result = await request("/ui/queryText", {
      method: "POST",
      body: JSON.stringify({ text }),
    });
    assert(result?.ok === true && result.count === 0, `Generic medication boilerplate is visible: ${text}`);
  }
}

async function assertProfileSexSymbols() {
  for (const symbol of ["♀", "♂"]) {
    const result = await request("/ui/queryText", {
      method: "POST",
      body: JSON.stringify({ text: symbol }),
    });
    assert(result?.ok === true && result.count > 0, `Active Profiles is missing the ${symbol} sex symbol`);
  }

  const legacyIcon = await request("/ui/queryText", {
    method: "POST",
    body: JSON.stringify({ text: "👤" }),
  });
  assert(legacyIcon?.ok === true && legacyIcon.count === 0, "Active Profiles still uses the generic person icon");

  const profileHelp = await request("/ui/queryText", {
    method: "POST",
    body: JSON.stringify({ text: "Sex estimate from DNA" }),
  });
  assert(profileHelp?.ok === true && profileHelp.count === 0, "Active Profiles still exposes the delayed sex-estimate tooltip");
}

async function ensurePopulatedSection() {
  let snapshot = await request("/ui/snapshot");
  if (
    snapshot.layout?.markerCardCount > 0 &&
    snapshot.layout?.expandedSectionNames?.includes("Pharmacogenomics (PGx)")
  ) return snapshot;

  const result = await request("/ui/clickSection", {
    method: "POST",
    body: JSON.stringify({ section: "Pharmacogenomics (PGx)" }),
  });
  assert(result?.ok === true, "Could not activate the PGx report section");
  snapshot = await waitForSnapshot(
    (candidate) =>
      candidate.layout?.activePresentationMode === "simple" &&
      candidate.layout.markerCardCount > 0 &&
      candidate.layout.expandedSectionNames?.includes("Pharmacogenomics (PGx)"),
    "a populated Simple report section",
    5_000,
  );
  assert(snapshot.layout.markerGridColumnCount !== null, "Populated Simple section did not render a finding grid");
  return snapshot;
}

async function ensureCollapsedSections() {
  let snapshot = await request("/ui/snapshot");
  let expandedSections = snapshot.layout?.expandedSectionNames;
  if (!Array.isArray(expandedSections)) {
    throw new Error("Desktop bridge did not expose expanded report sections");
  }

  while (expandedSections.length > 0) {
    const sectionName = expandedSections[0];
    const result = await request("/ui/clickSection", {
      method: "POST",
      body: JSON.stringify({ section: sectionName }),
    });
    assert(result?.ok === true, `Could not collapse report section ${sectionName}`);
    assert(
      result.detail?.startsWith("collapsed:"),
      `Expected report section ${sectionName} to collapse`,
    );
    snapshot = await waitForSnapshot(
      (candidate) => !candidate.layout?.expandedSectionNames?.includes(sectionName),
      `report section ${sectionName} to collapse`,
      5_000,
    );
    expandedSections = snapshot.layout?.expandedSectionNames;
    if (!Array.isArray(expandedSections)) {
      throw new Error("Desktop bridge lost expanded report-section state");
    }
  }

  await waitForSnapshot(
    (candidate) => candidate.layout?.activePresentationMode === "simple" && candidate.layout.markerCardCount === 0,
    "all populated report sections to collapse",
    5_000,
  );
}

async function assertClinicalCollapsedHint() {
  await clickText("Clinical");
  await waitForSnapshot(
    (snapshot) => snapshot.layout?.activePresentationMode === "clinical" && snapshot.layout.clinicalTableCount === 0,
    "Clinical mode with collapsed sections",
  );
  const result = await request("/ui/queryText", {
    method: "POST",
    body: JSON.stringify({ text: "Clinical view is ready." }),
  });
  assert(result?.ok === true && result.count > 0, "Collapsed Clinical mode does not explain how to reveal findings");
  await clickText("Simple");
  await waitForMode("simple");
}

async function main() {
  await prepareRequestedViewport();
  let ready = await waitForSnapshot(
    (snapshot) => snapshot.hasReport === true && snapshot.sample && snapshot.layout?.activePresentationMode && snapshot.resourceStatus !== null && snapshot.resourceUpdatePhase !== "checking",
    "a loaded Tauri report"
  );

  if (ready.layout.activePresentationMode !== "simple") {
    await clickText("Simple");
    ready = await waitForMode("simple");
  }

  const modes = {};
  const resourceStatus = validateResourceStatus(ready);
  const resourceUpdatePhase = validateResourceUpdatePhase(ready);
  await assertNoUnverifiedUpdateCopy(ready);
  validateDesktopSnapshot(ready, "simple");
  validateAccessibilitySnapshot(ready);
  await assertNoRedundantPublicCopy();
  await assertAllergySurface();
  await assertReportOrganization();
  await assertMedicationSurface();
  await assertProfileSexSymbols();
  await ensureCollapsedSections();
  await assertClinicalCollapsedHint();
  modes.simple = validateDesktopSnapshot(await ensurePopulatedSection(), "simple");

  const tooltipProbe = await request("/ui/probeTooltips", {
    method: "POST",
    body: JSON.stringify({}),
  });
  assert(tooltipProbe.visibleTriggerCount > 0, "Populated Simple report has no tooltip triggers to verify");
  assert(
    tooltipProbe.testedTriggerCount === tooltipProbe.visibleTriggerCount,
    "At least one visible non-interactive tooltip trigger did not open a single panel",
  );
  assert(
    tooltipProbe.openedPanelCount === tooltipProbe.withinViewportCount &&
      tooltipProbe.openedPanelCount === tooltipProbe.accessiblePanelCount,
    "A visible tooltip panel was clipped or lacked accessible metadata",
  );

  await clickText("Clinical");
  modes.clinical = validateDesktopSnapshot(await waitForMode("clinical"), "clinical");

  await clickText("Compare");
  modes.compare = validateDesktopSnapshot(await waitForMode("compare"), "compare");

  await clickText("Simple");
  const restored = validateDesktopSnapshot(await waitForMode("simple"), "simple");

  await clickText("Hide data sidebar");
  validateFocusModeSnapshot(await waitForSnapshot((snapshot) => snapshot.layout?.focusMode === true, "Focus Report mode"));
  await clickText("Show data sidebar");
  validateDesktopSnapshot(await waitForSnapshot((snapshot) => snapshot.layout?.focusMode === false, "the restored data sidebar"), "simple");

  await selectTheme("Light mode", "light");
  const lightContrast = await probeContrast("light");
  await selectTheme("Dark mode", "dark");
  const darkContrast = await probeContrast("dark");
  await selectTheme("System default", "system");
  const systemContrast = await probeContrast("system");

  console.log("PASS: Tauri desktop report audit");
  console.log(`  viewport=${ready.layout.viewportWidth}x${ready.layout.viewportHeight}; requested_viewport=${requestedViewportWidth || "current"}x${requestedViewportHeight || "current"}; sex=${restored.sex}; modes=simple,clinical,compare; simple_columns=${modes.simple.columns ?? "n/a"}; compare_columns=${modes.compare.columns ?? "n/a"}`);
  console.log(`  simple_cards=${modes.simple.markerCards}; simple_grid=${modes.simple.gridWidth ?? "n/a"}px; card_max=${modes.simple.cardWidth ?? "n/a"}px; overview=${ready.layout.reportOverviewWidth ?? "n/a"}px; action_queue=${ready.layout.actionQueueWidth ?? "n/a"}px; action_queue_shell=${ready.layout.actionQueueShellWidth ?? "n/a"}px; action_queue_columns=${ready.layout.actionQueueColumnCount ?? "n/a"}; guidance_grid=${ready.layout.dashboardGuidanceWidth ?? "n/a"}px; profiles=${ready.layout.profileNameCount}; profile_name_min_width=${ready.layout.profileNameMinWidth ?? "n/a"}px; connections=${modes.simple.connectionsHeight ?? "n/a"}px; liftover=${modes.simple.liftoverHeight ?? "n/a"}px; sidebar_hide_top=${ready.layout.sidebarFocusControlTop ?? "n/a"}px; sidebar_hide_edge_gap=${ready.layout.sidebarFocusControlRightGap ?? "n/a"}px; clinical_tables=${modes.clinical.clinicalTables}; compare_cards=${modes.compare.markerCards}; medication_surface=clean; public_copy=clean`);
  const simpleContract = modes.simple.simpleCardContract;
  console.log(`  simple_contract=${simpleContract?.cardCount ?? "n/a"}; title=${simpleContract?.titleCount ?? "n/a"}; signal=${simpleContract?.signalCount ?? "n/a"}; why_it_matters=${simpleContract?.whyItMattersCount ?? "n/a"}; review_action=${simpleContract?.reviewActionCount ?? "n/a"}; evidence=${simpleContract?.evidenceCount ?? "n/a"}; section_follow_up=${simpleContract?.sectionFollowUpCount ?? "n/a"}; details=${simpleContract?.detailsCount ?? "n/a"}; technical=${simpleContract?.technicalDataCount ?? "n/a"}`);
  console.log(`  tooltip_triggers=${tooltipProbe.visibleTriggerCount}; tooltip_opened=${tooltipProbe.openedPanelCount}; tooltip_viewport_safe=${tooltipProbe.withinViewportCount}; tooltip_accessible=${tooltipProbe.accessiblePanelCount}`);
  console.log(`  contrast_pairs=light:${lightContrast.checkedPairCount};dark:${darkContrast.checkedPairCount};system:${systemContrast.checkedPairCount}; minimum=light:${lightContrast.minimumRatio};dark:${darkContrast.minimumRatio};system:${systemContrast.minimumRatio}`);
  const warningMetrics = restored.warningMetrics;
  console.log(`  warnings=generic:${warningMetrics?.genericWarningPhraseCount ?? "n/a"};duplicates:${warningMetrics?.duplicateGenericWarningPhraseCount ?? "n/a"};actionable:${warningMetrics?.actionableAlertCount ?? "n/a"};clinical_review:${warningMetrics?.clinicalReviewAlertCount ?? "n/a"};guide:${warningMetrics?.reportGuideCount ?? "n/a"};footer:${warningMetrics?.footerReminderCount ?? "n/a"};legal:${warningMetrics?.legalPrivacyPageCount ?? "n/a"}`);
  console.log(`  resource_status=${resourceStatus}; update_phase=${resourceUpdatePhase ?? "idle"}; app_context_menu=installed`);
}

main().catch((error) => {
  console.error(`FAIL: ${error.message}`);
  process.exitCode = 1;
});
