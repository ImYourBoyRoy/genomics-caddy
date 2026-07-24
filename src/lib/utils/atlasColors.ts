// ./src/lib/utils/atlasColors.ts
/**
 * Category → color mapping for the Vector Atlas scatter plot.
 * Stable hues so the same trait category keeps the same color across rebuilds.
 */

const PALETTE = [
  "#38bdf8",
  "#34d399",
  "#a78bfa",
  "#f472b6",
  "#fbbf24",
  "#fb7185",
  "#2dd4bf",
  "#818cf8",
  "#4ade80",
  "#e879f9",
  "#67e8f9",
  "#fcd34d",
] as const;

const UNCATEGORIZED = "#64748b";

function hashCategory(name: string): number {
  let h = 0;
  for (let i = 0; i < name.length; i++) {
    h = (h * 31 + name.charCodeAt(i)) >>> 0;
  }
  return h;
}

export function atlasCategoryColor(category: string | null | undefined): string {
  const key = (category || "").trim();
  if (!key) return UNCATEGORIZED;
  return PALETTE[hashCategory(key.toLowerCase()) % PALETTE.length];
}

export function atlasDotRadius(dataQuality: number): number {
  const q = Math.max(0, Math.min(1, dataQuality));
  return 3.2 + q * 3.4;
}
