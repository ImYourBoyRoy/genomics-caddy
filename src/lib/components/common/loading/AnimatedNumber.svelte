<!-- ./src/lib/components/common/loading/AnimatedNumber.svelte -->
<script lang="ts">
  /*
  Purpose: Eased count-up for headline numbers so a freshly loaded report feels
  like it is resolving rather than appearing.
  Inputs: value (target integer), durationMs (default 700).
  Outputs: Plain text; the final value is always set by a timer fallback so a
  paused animation frame loop can never leave a stale number on screen.
  Notes: Respects prefers-reduced-motion via animateCountUp. Renders a bare
  text node so the parent's typography (headline sizes) always applies.
  */
  import { animateCountUp } from '../bootstrap/bootstrapCountUp';

  interface Props {
    value: number;
    durationMs?: number;
  }

  let { value, durationMs = 700 }: Props = $props();

  let shown = $state(0);
  let run = 0;

  $effect(() => {
    const target = value;
    const token = ++run;
    shown = 0;
    void animateCountUp(target, durationMs, (next) => {
      if (token === run) shown = next;
    });
    const fallback = window.setTimeout(() => {
      if (token === run) shown = target;
    }, durationMs + 250);
    return () => window.clearTimeout(fallback);
  });
</script>

{shown.toLocaleString()}
