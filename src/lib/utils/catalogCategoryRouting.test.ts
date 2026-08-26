import { describe, expect, it } from "vitest";
import {
  buildCatalogCategories,
  defaultCatalogCategorySelection,
  selectedCatalogCategoryIds
} from "./catalogCategoryRouting";

describe("catalog category routing", () => {
  const markers = [
    { category: "hormones_reproductive" },
    { category: "core" },
    { category: "hormones_reproductive" },
    { category: "new_domain" },
    { category: "" },
    { category: null }
  ];

  it("derives every present category with stable counts and labels", () => {
    expect(buildCatalogCategories(markers)).toEqual([
      { id: "core", label: "🧬 Core Traits", count: 1 },
      { id: "hormones_reproductive", label: "🌙 Hormones & Reproductive", count: 2 },
      { id: "new_domain", label: "🔬 New Domain", count: 1 }
    ]);
  });

  it("defaults only the bounded core scan set and keeps newly added domains reachable", () => {
    const categories = buildCatalogCategories(markers);
    const selection = defaultCatalogCategorySelection(categories);

    expect(selection.core).toBe(true);
    expect(selection.hormones_reproductive).toBe(true);
    expect(selection.new_domain).toBe(false);
    expect(selectedCatalogCategoryIds(categories, selection)).toEqual(
      new Set(["core", "hormones_reproductive"])
    );
  });

  it("does not route unknown or stale UI ids into a catalog scan", () => {
    const categories = buildCatalogCategories(markers);
    expect(selectedCatalogCategoryIds(categories, { new_domain: true, stale_id: true })).toEqual(
      new Set(["new_domain"])
    );
  });
});
