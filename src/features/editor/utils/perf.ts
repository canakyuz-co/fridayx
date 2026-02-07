type LatencySummary = {
  count: number;
  p50: number;
  p95: number;
  latest: number;
};

export type EditorMetricEventDetail = {
  label: string;
  summary: LatencySummary;
};

function percentile(sorted: number[], ratio: number) {
  if (sorted.length === 0) {
    return 0;
  }
  const index = Math.max(
    0,
    Math.min(sorted.length - 1, Math.ceil(sorted.length * ratio) - 1),
  );
  return sorted[index];
}

export function createLatencyTracker(
  label: string,
  reportEvery = 20,
  maxSamples = 200,
) {
  let samples: number[] = [];
  return (valueMs: number): LatencySummary | null => {
    if (!Number.isFinite(valueMs) || valueMs < 0) {
      return null;
    }
    samples.push(valueMs);
    if (samples.length > maxSamples) {
      samples = samples.slice(samples.length - maxSamples);
    }
    if (samples.length % reportEvery !== 0) {
      return null;
    }
    const sorted = [...samples].sort((a, b) => a - b);
    const summary: LatencySummary = {
      count: samples.length,
      p50: percentile(sorted, 0.5),
      p95: percentile(sorted, 0.95),
      latest: valueMs,
    };
    console.info(
      `[editor-metric] ${label} count=${summary.count} p50=${summary.p50.toFixed(2)}ms p95=${summary.p95.toFixed(2)}ms latest=${summary.latest.toFixed(2)}ms`,
    );
    if (typeof window !== "undefined") {
      window.dispatchEvent(
        new CustomEvent<EditorMetricEventDetail>("fridex-editor-metric", {
          detail: { label, summary },
        }),
      );
    }
    return summary;
  };
}

export function isRustEditorSearchEnabled() {
  if (typeof window === "undefined") {
    return true;
  }
  return window.localStorage.getItem("fridex.flags.rustEditorSearch") !== "false";
}
