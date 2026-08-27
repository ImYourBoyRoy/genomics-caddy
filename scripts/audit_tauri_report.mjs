#!/usr/bin/env node
/*
Purpose: Exercise the running Tauri desktop report through its local UI bridge.
How to run: `npm run audit:tauri-ui` while `npm run tauri:dev` is running.
Outputs: Aggregate-only desktop mode, geometry, sex-label, and public-copy checks.
Privacy: Never prints sample names, genotype calls, technical disclosures, or raw UI text.
*/

import { setTimeout as sleep } from "node:timers/promises";

const baseUrl = (process.env.GENOMICS_AGENT_UI_URL || "http://127.0.0.1:17321").replace(/\/$/, "");
const timeoutMs = parsePositiveInteger(process.env.GENOMICS_TAURI_AUDIT_TIMEOUT_MS, 120_000);
const pollMs = 500;

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
  assert(layout.overflowingElements.length === 0, "Desktop bridge reported overflowing elements");
  assert(layout.focusControlOverlapsContent === false, "Focus Report control overlaps the first content block");
  if (layout.focusControlBottom !== null && layout.firstContentTop !== null) {
    assert(layout.focusControlBottom <= layout.firstContentTop, "Focus Report control is not above the first content block");
    assert(
      layout.firstContentTop - layout.focusControlBottom >= 4,
      "Focus Report control has no visual separation from the first content block",
    );
  }
  assert(layout.themeControlInToolbar === true, "Theme control is not in the desktop toolbar");
  assert(layout.actionQueueItemCount <= 5, "Action queue exceeds the five-item desktop contract");
  if (layout.actionQueueWidth !== null && layout.mainContentWidth !== null) {
    assert(layout.actionQueueWidth < layout.mainContentWidth, "Action queue still stretches across the full report pane");
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

  if (expectedMode === "clinical") {
    assert(layout.clinicalTableCount === 1, "Clinical mode did not render its structured findings table");
    assert(layout.clinicalProvenanceCount === 1, "Clinical mode did not render its single provenance disclosure");
    assert(layout.markerCardCount === 0, "Clinical mode rendered duplicate marker cards");
  } else {
    assert(layout.clinicalTableCount === 0, `${expectedMode} mode rendered a clinical table`);
    assert(layout.clinicalProvenanceCount === 0, `${expectedMode} mode rendered Clinical provenance content`);
  }

  return {
    sex,
    markerCards: layout.markerCardCount,
    clinicalTables: layout.clinicalTableCount,
    clinicalProvenance: layout.clinicalProvenanceCount,
    columns: layout.markerGridColumnCount,
    gridWidth: layout.markerGridWidth,
    connectionsHeight: layout.connectionsLaunchHeight,
    liftoverHeight: layout.liftoverStatusHeight,
  };
}

async function selectTheme(label, expectedMode) {
  await clickText(label);
  const snapshot = await waitForSnapshot(
    (candidate) => candidate.themeMode === expectedMode,
    `${expectedMode} theme mode`,
  );
  assert(snapshot.themeMode === expectedMode, `Expected ${expectedMode} theme mode`);
  return snapshot;
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

async function clickText(text) {
  const result = await request("/ui/clickText", {
    method: "POST",
    body: JSON.stringify({ text }),
  });
  assert(result?.ok === true, `Could not activate ${text}`);
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

async function ensurePopulatedSection() {
  let snapshot = await request("/ui/snapshot");
  if (snapshot.layout?.markerCardCount > 0) return snapshot;

  await request("/ui/clickSection", {
    method: "POST",
    body: JSON.stringify({ section: "Pharmacogenomics (PGx)" }),
  });
  snapshot = await waitForSnapshot(
    (candidate) => candidate.layout?.activePresentationMode === "simple" && candidate.layout.markerCardCount > 0,
    "a populated Simple report section",
    5_000,
  );
  assert(snapshot.layout.markerGridColumnCount !== null, "Populated Simple section did not render a finding grid");
  return snapshot;
}

async function ensureCollapsedSection() {
  const snapshot = await request("/ui/snapshot");
  if (snapshot.layout?.markerCardCount === 0) return;

  await request("/ui/clickSection", {
    method: "POST",
    body: JSON.stringify({ section: "Pharmacogenomics (PGx)" }),
  });
  await waitForSnapshot(
    (candidate) => candidate.layout?.activePresentationMode === "simple" && candidate.layout.markerCardCount === 0,
    "the populated section to collapse",
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
  let ready = await waitForSnapshot(
    (snapshot) => snapshot.hasReport === true && snapshot.sample && snapshot.layout?.activePresentationMode,
    "a loaded Tauri report"
  );

  if (ready.layout.activePresentationMode !== "simple") {
    await clickText("Simple");
    ready = await waitForMode("simple");
  }

  const modes = {};
  validateDesktopSnapshot(ready, "simple");
  await assertNoRedundantPublicCopy();
  await ensureCollapsedSection();
  await assertClinicalCollapsedHint();
  modes.simple = validateDesktopSnapshot(await ensurePopulatedSection(), "simple");

  await clickText("Sex estimate from DNA");
  validateTooltipSnapshot(await waitForSnapshot((snapshot) => snapshot.tooltip?.openPanelCount === 1, "the sex-estimate tooltip"));
  await clickText("Sex estimate from DNA");
  await waitForSnapshot((snapshot) => snapshot.tooltip?.openPanelCount === 0, "the sex-estimate tooltip to close");

  await clickText("Clinical");
  modes.clinical = validateDesktopSnapshot(await waitForMode("clinical"), "clinical");

  await clickText("Compare");
  modes.compare = validateDesktopSnapshot(await waitForMode("compare"), "compare");

  await clickText("Simple");
  const restored = validateDesktopSnapshot(await waitForMode("simple"), "simple");

  await clickText("Color theme");
  await selectTheme("Light mode", "light");
  await clickText("Color theme");
  await selectTheme("Dark mode", "dark");
  await clickText("Color theme");
  await selectTheme("System default", "system");

  console.log("PASS: Tauri desktop report audit");
  console.log(`  sex=${restored.sex}; modes=simple,clinical,compare; simple_columns=${modes.simple.columns ?? "n/a"}; compare_columns=${modes.compare.columns ?? "n/a"}`);
  const toolbarGap = ready.layout.firstContentTop !== null && ready.layout.focusControlBottom !== null
    ? ready.layout.firstContentTop - ready.layout.focusControlBottom
    : "n/a";
  console.log(`  simple_cards=${modes.simple.markerCards}; simple_grid=${modes.simple.gridWidth ?? "n/a"}px; connections=${modes.simple.connectionsHeight ?? "n/a"}px; liftover=${modes.simple.liftoverHeight ?? "n/a"}px; toolbar_gap=${toolbarGap}px; clinical_tables=${modes.clinical.clinicalTables}; compare_cards=${modes.compare.markerCards}; public_copy=clean`);
}

main().catch((error) => {
  console.error(`FAIL: ${error.message}`);
  process.exitCode = 1;
});
