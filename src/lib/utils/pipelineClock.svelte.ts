// ./src/lib/utils/pipelineClock.svelte.ts
/*
Purpose: One reactive clock for a multi-step loading pipeline. It owns the
total elapsed time, per-step durations (so finished steps can show how long
they took), the live elapsed time of the active step, and a smoothed ETA.
Usage: create one per loading surface, call `start()` on mount and `stop()` on
teardown, feed it step states with `sync()` and progress with `observeEta()`.
Notes: ticks four times a second so the displayed second changes within 250 ms
of the real boundary; the stopwatch dial animates on the compositor instead.
Freezing keeps the final numbers on screen once the pipeline is ready/failed.
*/

import type { PipelineStepState } from "../components/common/loading/pipelineTypes";
import { EtaSmoother, type EtaSample } from "./loadingTelemetry";

const TICK_MS = 250;

export class PipelineClock {
  #startedAt: number;
  #frozenAt: number | null = null;
  #now = $state(Date.now());
  #version = $state(0);
  #stepStart = new Map<string, number>();
  #stepEnd = new Map<string, number>();
  #lastBoundary: number;
  #timer: ReturnType<typeof setInterval> | null = null;
  #smoother = new EtaSmoother();
  #etaKey: string | null = null;

  /** Smoothed seconds remaining for the measured span, or null when unknown. */
  eta = $state<number | null>(null);

  constructor(startedAt: number = Date.now()) {
    this.#startedAt = startedAt;
    this.#lastBoundary = startedAt;
  }

  start(): void {
    if (this.#timer !== null || this.#frozenAt !== null) return;
    this.#now = Date.now();
    this.#timer = setInterval(() => {
      this.#now = Date.now();
    }, TICK_MS);
  }

  stop(): void {
    if (this.#timer !== null) clearInterval(this.#timer);
    this.#timer = null;
  }

  /** Stop ticking and keep the final numbers (ready or failed). */
  freeze(): void {
    if (this.#frozenAt !== null) return;
    this.#frozenAt = Date.now();
    this.#now = this.#frozenAt;
    this.stop();
  }

  /** Restart timing from now (used when a held surface begins real work). */
  reset(startedAt: number = Date.now()): void {
    this.#startedAt = startedAt;
    this.#lastBoundary = startedAt;
    this.#frozenAt = null;
    this.#now = Date.now();
    this.#stepStart.clear();
    this.#stepEnd.clear();
    this.#smoother.reset();
    this.#etaKey = null;
    this.eta = null;
    this.#version += 1;
  }

  get frozen(): boolean {
    return this.#frozenAt !== null;
  }

  get elapsedMs(): number {
    return Math.max(0, (this.#frozenAt ?? this.#now) - this.#startedAt);
  }

  /** Record step transitions; safe to call from an effect on every state change. */
  sync(states: Record<string, PipelineStepState>): void {
    const now = this.#frozenAt ?? Date.now();
    let changed = false;
    for (const [id, state] of Object.entries(states)) {
      if ((state === "active" || state === "error") && !this.#stepStart.has(id)) {
        this.#stepStart.set(id, this.#lastBoundary);
        changed = true;
      }
      if ((state === "done" || state === "error") && !this.#stepEnd.has(id)) {
        if (!this.#stepStart.has(id)) this.#stepStart.set(id, this.#lastBoundary);
        this.#stepEnd.set(id, now);
        this.#lastBoundary = now;
        changed = true;
      }
    }
    if (changed) this.#version += 1;
  }

  durationMs(id: string): number | null {
    void this.#version;
    const start = this.#stepStart.get(id);
    const end = this.#stepEnd.get(id);
    return start === undefined || end === undefined ? null : Math.max(0, end - start);
  }

  stepElapsedMs(id: string): number | null {
    void this.#version;
    const start = this.#stepStart.get(id);
    if (start === undefined || this.#stepEnd.has(id)) return null;
    return Math.max(0, (this.#frozenAt ?? this.#now) - start);
  }

  /** Feed the ETA estimator; the smoother resets whenever `key` changes. */
  observeEta(key: string, sample: EtaSample | null): void {
    if (key !== this.#etaKey) {
      this.#etaKey = key;
      this.#smoother.reset();
    }
    this.eta = this.#smoother.update(sample);
  }
}
