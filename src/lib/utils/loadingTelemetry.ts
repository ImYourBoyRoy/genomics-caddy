// ./src/lib/utils/loadingTelemetry.ts
/*
Purpose: Pure helpers behind the loading telemetry UI (clock text, step
durations, smoothed time-remaining estimates, and the monotonic report-step
progress model). No DOM, no Svelte state; everything here is unit tested.

Inputs: elapsed milliseconds, 0-1 progress fractions, backend report progress.
Outputs: display strings and numbers; the UI decides how to draw them.
Notes: estimates are intentionally conservative. Showing no estimate is better
than showing a wild one, so every estimator returns null until it has enough
signal to be honest.
*/

import type { ReportGenerationProgress } from "../types/genomics";

const MS_PER_SECOND = 1000;

/** Stopwatch text: `0:07`, `12:03`, `1:02:03`. Fixed-format so digits do not jitter. */
export function formatClock(ms: number): string {
  const totalSeconds = Math.max(0, Math.floor((Number.isFinite(ms) ? ms : 0) / MS_PER_SECOND));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  const ss = String(seconds).padStart(2, "0");
  if (hours > 0) return `${hours}:${String(minutes).padStart(2, "0")}:${ss}`;
  return `${minutes}:${ss}`;
}

/** Compact duration for step chips: `0.4s`, `12s`, `1m 05s`. */
export function formatStepDuration(ms: number): string {
  const safe = Math.max(0, Number.isFinite(ms) ? ms : 0);
  if (safe < 950) return `${(Math.round(safe / 100) / 10).toFixed(1)}s`;
  const seconds = Math.round(safe / MS_PER_SECOND);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  return `${minutes}m ${String(seconds % 60).padStart(2, "0")}s`;
}

/** Remaining-time text: `<5s`, `~12s`, `~1m 20s`. */
export function formatRemaining(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return "";
  if (seconds < 5) return "<5s";
  if (seconds < 60) return `~${Math.round(seconds)}s`;
  const rounded = Math.round(seconds / 5) * 5;
  const minutes = Math.floor(rounded / 60);
  const rest = rounded % 60;
  return rest === 0 ? `~${minutes}m` : `~${minutes}m ${String(rest).padStart(2, "0")}s`;
}

export function formatCount(value: number): string {
  return new Intl.NumberFormat().format(Math.max(0, Math.round(value)));
}

export function clampFraction(value: number): number {
  if (!Number.isFinite(value)) return 0;
  return Math.min(1, Math.max(0, value));
}

export interface EtaSample {
  /** Progress of the measured span, 0-1. */
  fraction: number;
  /** Time spent on that same span, in milliseconds. */
  elapsedMs: number;
}

/**
 * Smooths a linear-rate ETA so the number counts down instead of flickering.
 * Needs a few seconds and a few percent of signal before it speaks, ignores
 * upward jumps larger than one second per update, and goes quiet near the end.
 */
export class EtaSmoother {
  static readonly MIN_ELAPSED_MS = 2500;
  static readonly MIN_FRACTION = 0.03;
  static readonly DONE_FRACTION = 0.995;
  static readonly ALPHA = 0.3;

  #ema: number | null = null;
  #shown: number | null = null;

  reset(): void {
    this.#ema = null;
    this.#shown = null;
  }

  update(sample: EtaSample | null): number | null {
    if (!sample) {
      this.reset();
      return null;
    }
    const fraction = clampFraction(sample.fraction);
    if (
      fraction >= EtaSmoother.DONE_FRACTION ||
      fraction < EtaSmoother.MIN_FRACTION ||
      sample.elapsedMs < EtaSmoother.MIN_ELAPSED_MS
    ) {
      return this.#shown === null ? null : Math.max(0, Math.ceil(this.#shown));
    }
    const raw = (sample.elapsedMs * (1 - fraction)) / fraction / MS_PER_SECOND;
    this.#ema = this.#ema === null ? raw : this.#ema * (1 - EtaSmoother.ALPHA) + raw * EtaSmoother.ALPHA;
    this.#shown =
      this.#shown === null || this.#ema <= this.#shown
        ? this.#ema
        : Math.min(this.#ema, this.#shown + 1);
    return Math.max(0, Math.ceil(this.#shown));
  }
}

/**
 * One monotonic 0-1 fraction for the whole report step. The backend reports
 * per-phase counters that restart at zero, so a naive percent jumps backwards
 * when the scan moves from curated markers to ClinVar.
 */
const REPORT_PHASE_SPAN = {
  preparing: { start: 0, span: 0.06 },
  markers: { start: 0.06, span: 0.3 },
  clinvar: { start: 0.36, span: 0.56 },
  finalizing: { start: 0.92, span: 0.08 },
} as const;

export function reportStepFraction(progress: ReportGenerationProgress | null): number | null {
  if (!progress) return null;
  const range = REPORT_PHASE_SPAN[progress.phase];
  if (!range) return null;
  if (progress.phase === "markers" || progress.phase === "clinvar") {
    const within = progress.total > 0 ? clampFraction(progress.current / progress.total) : 0;
    return clampFraction(range.start + range.span * within);
  }
  return range.start;
}

/** Fraction of the current backend phase, or null when it has no counter. */
export function reportPhaseFraction(progress: ReportGenerationProgress | null): number | null {
  if (!progress || progress.total <= 0) return null;
  if (progress.phase !== "markers" && progress.phase !== "clinvar") return null;
  return clampFraction(progress.current / progress.total);
}

/** Human counter such as `Curated marker checks 1,204 / 4,810 · 312 matched`. */
export function reportCountLabel(progress: ReportGenerationProgress | null): string | null {
  if (!progress || progress.total <= 0) return null;
  const noun =
    progress.phase === "markers"
      ? "Curated marker checks"
      : progress.phase === "clinvar"
        ? "Profile variants checked"
        : null;
  if (!noun) return null;
  const matches =
    progress.matchesLabel && progress.matches > 0
      ? ` · ${formatCount(progress.matches)} ${progress.matchesLabel}`
      : "";
  return `${noun} ${formatCount(progress.current)} / ${formatCount(progress.total)}${matches}`;
}

/** Parse an `(n of m)` counter out of a status line, e.g. `Importing pack (3 of 12)`. */
export function parseMessageProgress(
  message: string,
): { current: number; total: number; fraction: number } | null {
  const match = message.match(/\((\d+)\s+of\s+(\d+)\)/);
  if (!match) return null;
  const current = Number.parseInt(match[1], 10);
  const total = Number.parseInt(match[2], 10);
  if (!Number.isFinite(current) || !Number.isFinite(total) || total <= 0) return null;
  return { current, total, fraction: clampFraction(current / total) };
}
