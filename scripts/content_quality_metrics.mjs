/**
 * Privacy-safe content-quality metrics for curated genomic resources.
 *
 * This module intentionally returns aggregate counts and structural locations,
 * never marker calls, allele values, raw DNA rows, or private report bodies.
 */

/** @typedef {Record<string, any>} JsonObject */
/** @typedef {{ key: string, location: string, value: string }} CopyEntry */
/** @typedef {{ key: string, location: string, values: string[] }} StringArrayEntry */
/** @typedef {{ location: string, expected: number, actual: number, mismatch: boolean }} LinkCountCheck */
/** @typedef {{ location: string, count: number, unique: number, duplicate_references: number }} MultiMarkerGroup */
/** @typedef {{ packId: string, marker: JsonObject, index: number }} MarkerRow */
/** @typedef {{ packId: string, gene: string }} InterpretationOccurrence */

const COPY_KEYS = new Set([
  'simpleMeaning',
  'simple_meaning',
  'simpleImpact',
  'simple_impact',
  'impact',
  'interpretation',
  'raw_dna_limitation',
  'do_not_claim',
  'confirm_with',
  'next_step',
  'next_helpful_step',
  'summary',
  'relevance',
  'description',
  'purpose',
  'implementation_note',
  'disclaimer',
  'safety_notes',
  'favor',
  'avoid',
  'dietary_favor',
  'dietary_avoid',
  'supplements',
  'supplement_favor',
  'supplement_avoid',
  'activity_favor',
  'activity_avoid',
  'food_favor',
  'food_avoid',
  'recommendation',
  'recommendations',
  'hint',
]);

const TOPIC_KEYS = new Set([
  'topic',
  'topic_id',
  'topic_label',
  'category',
  'category_id',
  'category_label',
  'health_area',
  'health_area_id',
  'section',
  'section_id',
]);

const RECOMMENDATION_KEYS = new Set([
  'favor',
  'avoid',
  'dietary_favor',
  'dietary_avoid',
  'supplements',
  'supplement_favor',
  'supplement_avoid',
  'activity_favor',
  'activity_avoid',
  'food_favor',
  'food_avoid',
  'recommendation',
  'recommendations',
  'lab_tests',
]);

const GENERIC_COPY_PATTERNS = [
  { id: 'diagnostic_boundary', pattern: /\b(?:not a diagnosis|does not diagnose|cannot diagnose|does not establish)\b/i },
  { id: 'raw_dna_boundary', pattern: /\braw\s+(?:consumer\s+)?DNA\b/i },
  { id: 'personal_probability_boundary', pattern: /\b(?:personal disease probability|does not predict whether you have|not a personal)\b/i },
  { id: 'clinical_confirmation', pattern: /\b(?:clinical confirmation|clinical testing|confirm(?:ed|ation)? with|clinician review|doctor(?:'s)? confirmation)\b/i },
  { id: 'research_context', pattern: /\b(?:research context|studied in research|research association|associated with research)\b/i },
  { id: 'current_measurement_boundary', pattern: /\bdoes not measure (?:current|your|the)\b/i },
];

const SIMPLE_COPY_FIELDS = ['plainTitle', 'signal', 'whyItMatters', 'reviewAction', 'evidenceLabel'];
const SIMPLE_COPY_VAGUE_PATTERNS = [
  /dedicated plain-English explanation is not yet available/i,
  /^genetic context marker$/i,
  /associated with a research finding/i,
  /studied in genetic research/i,
];

/**
 * Literal reuse is not automatically a content defect. These classes keep
 * the audit useful by separating copy that should be centralized at render
 * time from copy that likely needs a resource rewrite.
 */
/** @type {Record<string, { label: string, keys: Set<string> }>} */
const COPY_REUSE_CLASSES = {
  shared_boundary: {
    label: 'Shared evidence boundary',
    keys: new Set(['raw_dna_limitation', 'do_not_claim', 'disclaimer', 'safety_notes']),
  },
  shared_follow_up: {
    label: 'Shared follow-up target',
    keys: new Set(['confirm_with', 'next_step', 'next_helpful_step']),
  },
  shared_recommendation: {
    label: 'Shared recommendation',
    keys: new Set([
      'favor',
      'avoid',
      'dietary_favor',
      'dietary_avoid',
      'supplements',
      'supplement_favor',
      'supplement_avoid',
      'activity_favor',
      'activity_avoid',
      'food_favor',
      'food_avoid',
      'recommendation',
      'recommendations',
    ]),
  },
  repeated_marker_interpretation: {
    label: 'Repeated marker interpretation',
    keys: new Set(['interpretation']),
  },
  marker_impact: {
    label: 'Marker technical impact',
    keys: new Set(['impact']),
  },
  shared_simple_translation: {
    label: 'Shared Simple translation',
    keys: new Set(['simpleMeaning', 'simple_meaning', 'simpleImpact', 'simple_impact']),
  },
  shared_metadata_copy: {
    label: 'Shared metadata copy',
    keys: new Set(['summary', 'relevance', 'description', 'purpose', 'implementation_note', 'hint']),
  },
  other_copy: {
    label: 'Other repeated copy',
    keys: new Set(),
  },
};

/** @param {unknown} value */
function text(value) {
  return typeof value === 'string' ? value.trim() : '';
}

/** @param {unknown} value */
function normalizedText(value) {
  return text(value).replace(/\s+/g, ' ').toLowerCase();
}

/** @param {unknown} value */
function normalizedRsid(value) {
  const valueText = text(value).toLowerCase();
  return /^rs\d+$/.test(valueText) ? valueText : '';
}

/** @param {unknown} value */
function geneTokens(value) {
  return text(value)
    .split(/[\s/;,]+/)
    .map((item) => item.trim().toUpperCase())
    .filter(Boolean);
}

/** @param {unknown} value @returns {value is string[]} */
function isStringArray(value) {
  return Array.isArray(value) && value.every((item) => typeof item === 'string');
}

/** @param {Map<string, number>} map @param {string} key @param {number} [amount] */
function addCount(map, key, amount = 1) {
  if (!key) return;
  map.set(key, (map.get(key) || 0) + amount);
}

/** @param {Map<string, number>} map */
function mapToRepeatedSummary(map) {
  const rows = Array.from(map.entries())
    .filter(([, count]) => count > 1)
    .sort((left, right) => right[1] - left[1] || left[0].length - right[0].length);
  return {
    unique: map.size,
    repeated_values: rows.length,
    repeated_occurrences: rows.reduce((total, [, count]) => total + count, 0),
    top: rows.slice(0, 10).map(([value, count]) => ({ occurrences: count, characters: value.length })),
  };
}

/** @param {string} key @param {string} value */
function copyReuseClass(key, value) {
  for (const [id, definition] of Object.entries(COPY_REUSE_CLASSES)) {
    if (id !== 'other_copy' && definition.keys.has(key)) return id;
  }
  if (/dedicated plain-English explanation is not yet available|genetic context marker/i.test(value)) {
    return 'fallback_copy';
  }
  return 'other_copy';
}

/** @param {CopyEntry[]} entries @param {number} [fallbackCount] */
function copyReuseSummary(entries, fallbackCount = 0) {
  /** @type {Map<string, Map<string, number>>} */
  const classMaps = new Map();
  for (const id of Object.keys(COPY_REUSE_CLASSES)) classMaps.set(id, new Map());
  classMaps.set('fallback_copy', new Map());

  for (const entry of entries) {
    const classId = copyReuseClass(entry.key, entry.value);
    const values = classMaps.get(classId);
    if (values) addCount(values, normalizedText(entry.value));
  }

  /** @type {Record<string, { label: string, candidate_entries: number, repeated_values: number, repeated_occurrences: number }>} */
  const result = {};
  for (const [id, values] of classMaps.entries()) {
    const summary = mapToRepeatedSummary(values);
    result[id] = {
      label: id === 'fallback_copy' ? 'Generic fallback copy' : COPY_REUSE_CLASSES[id]?.label || 'Repeated copy',
      candidate_entries: values.values().reduce((total, count) => total + count, 0),
      repeated_values: summary.repeated_values,
      repeated_occurrences: summary.repeated_occurrences,
    };
  }
  result.fallback_copy = {
    label: 'Generic fallback meaning',
    candidate_entries: fallbackCount,
    repeated_values: fallbackCount > 0 ? 1 : 0,
    repeated_occurrences: fallbackCount,
  };
  return result;
}

/** @param {MarkerRow[]} rows */
function classifyRepeatedInterpretations(rows) {
  /** @type {Map<string, InterpretationOccurrence[]>} */
  const interpretationGroups = new Map();
  for (const row of rows) {
    const interpretation = text(row.marker?.interpretation);
    if (!interpretation) continue;
    const key = normalizedText(interpretation);
    const existing = interpretationGroups.get(key);
    const occurrence = {
      packId: text(row.packId),
      gene: text(row.marker?.gene).toUpperCase(),
    };
    if (existing) existing.push(occurrence);
    else interpretationGroups.set(key, [occurrence]);
  }

  const repeatedGroups = /** @type {InterpretationOccurrence[][]} */ (
    Array.from(interpretationGroups.values()).filter((group) => group.length > 1)
  );
  const summary = {
    repeated_values: repeatedGroups.length,
    repeated_occurrences: repeatedGroups.reduce((total, group) => total + group.length, 0),
    classifications: {
      marker_family_context: { values: 0, occurrences: 0 },
      section_shared_context: { values: 0, occurrences: 0 },
      cross_section_reuse: { values: 0, occurrences: 0 },
    },
  };

  for (const group of repeatedGroups) {
    const packs = new Set(group.map((row) => row.packId));
    const genes = new Set(group.map((row) => row.gene).filter(Boolean));
    const classification = packs.size > 1
      ? 'cross_section_reuse'
      : genes.size <= 1
        ? 'marker_family_context'
        : 'section_shared_context';
    summary.classifications[classification].values += 1;
    summary.classifications[classification].occurrences += group.length;
  }

  return summary;
}

/** @param {unknown} value @param {string} [key] @param {string} [location] @param {CopyEntry[]} [entries] @returns {CopyEntry[]} */
function collectCopyEntries(value, key = '', location = 'root', entries = []) {
  if (typeof value === 'string') {
    if (COPY_KEYS.has(key) && text(value)) entries.push({ key, location, value: text(value) });
    return entries;
  }
  if (!value || typeof value !== 'object') return entries;
  if (Array.isArray(value)) {
    value.forEach((item, index) => collectCopyEntries(item, key, `${location}[${index}]`, entries));
    return entries;
  }
  for (const [childKey, childValue] of Object.entries(value)) {
    collectCopyEntries(childValue, childKey, `${location}.${childKey}`, entries);
  }
  return entries;
}

/** @param {unknown} value @param {string} [key] @param {string} [location] @param {CopyEntry[]} [labels] @returns {CopyEntry[]} */
function collectTopicLabels(value, key = '', location = 'root', labels = []) {
  if (typeof value === 'string') {
    if (TOPIC_KEYS.has(key) && text(value)) labels.push({ key, location, value: text(value) });
    return labels;
  }
  if (!value || typeof value !== 'object') return labels;
  if (Array.isArray(value)) {
    value.forEach((item, index) => collectTopicLabels(item, key, `${location}[${index}]`, labels));
    return labels;
  }
  for (const [childKey, childValue] of Object.entries(value)) {
    collectTopicLabels(childValue, childKey, `${location}.${childKey}`, labels);
  }
  return labels;
}

/** @param {unknown} value @param {string} [key] @param {string} [location] @param {StringArrayEntry[]} [results] @returns {StringArrayEntry[]} */
function collectStringArrays(value, key = '', location = 'root', results = []) {
  if (!value || typeof value !== 'object') return results;
  if (Array.isArray(value)) {
    if (isStringArray(value) && (key === 'sources' || key === 'source_ids' || key === 'reference_ids')) {
      results.push({ key, location, values: value.map((item) => item.trim()).filter(Boolean) });
      return results;
    }
    value.forEach((item, index) => collectStringArrays(item, key, `${location}[${index}]`, results));
    return results;
  }
  for (const [childKey, childValue] of Object.entries(value)) {
    collectStringArrays(childValue, childKey, `${location}.${childKey}`, results);
  }
  return results;
}

/** @param {unknown} value @param {string} [location] @param {LinkCountCheck[]} [checks] @returns {LinkCountCheck[]} */
function collectLinkCountChecks(value, location = 'root', checks = []) {
  if (!value || typeof value !== 'object') return checks;
  if (Array.isArray(value)) {
    value.forEach((item, index) => collectLinkCountChecks(item, `${location}[${index}]`, checks));
    return checks;
  }
  const object = /** @type {JsonObject} */ (value);

  const countPairs = [
    ['matched_marker_count', 'matched_marker_link_ids'],
    ['marker_count', 'marker_link_ids'],
    ['matched_count', 'matched_marker_link_ids'],
  ];
  for (const [countKey, linksKey] of countPairs) {
    if (typeof object[countKey] === 'number' && Array.isArray(object[linksKey])) {
      checks.push({
        location,
        expected: object[countKey],
        actual: object[linksKey].length,
        mismatch: object[countKey] !== object[linksKey].length,
      });
    }
  }
  if (Array.isArray(object.marker_ids) && Array.isArray(object.marker_link_ids)) {
    checks.push({
      location,
      expected: object.marker_ids.length,
      actual: object.marker_link_ids.length,
      mismatch: object.marker_ids.length !== object.marker_link_ids.length,
    });
  }
  for (const [childKey, childValue] of Object.entries(value)) {
    collectLinkCountChecks(childValue, `${location}.${childKey}`, checks);
  }
  return checks;
}

/** @param {unknown} value @param {string} [key] @param {string} [location] @param {MultiMarkerGroup[]} [groups] @returns {MultiMarkerGroup[]} */
function collectMultiMarkerGroups(value, key = '', location = 'root', groups = []) {
  if (!value || typeof value !== 'object') return groups;
  if (Array.isArray(value)) {
    value.forEach((item, index) => collectMultiMarkerGroups(item, key, `${location}[${index}]`, groups));
    return groups;
  }
  for (const [childKey, childValue] of Object.entries(value)) {
    if (Array.isArray(childValue) && childValue.length > 1 && /(?:marker_ids|matched_marker_ids|rsids|markers)$/i.test(childKey)) {
      const values = childValue.map((item) => text(typeof item === 'string' ? item : item?.rsid || item?.id)).filter(Boolean);
      groups.push({
        location: `${location}.${childKey}`,
        count: values.length,
        unique: new Set(values.map((item) => item.toLowerCase())).size,
        duplicate_references: values.length - new Set(values.map((item) => item.toLowerCase())).size,
      });
    }
    collectMultiMarkerGroups(childValue, childKey, `${location}.${childKey}`, groups);
  }
  return groups;
}

/** @param {unknown} value @param {string} [key] @param {string} [location] @param {JsonObject[]} [items] @returns {JsonObject[]} */
function recommendationItems(value, key = '', location = 'root', items = []) {
  if (!value || typeof value !== 'object') return items;
  if (Array.isArray(value)) {
    value.forEach((item, index) => {
      if (RECOMMENDATION_KEYS.has(key) && (typeof item === 'string' || (item && typeof item === 'object'))) {
        items.push({ location: `${location}[${index}]`, key, item });
      }
      recommendationItems(item, key, `${location}[${index}]`, items);
    });
    return items;
  }
  for (const [childKey, childValue] of Object.entries(value)) {
    if (RECOMMENDATION_KEYS.has(childKey) && typeof childValue === 'string' && text(childValue)) {
      items.push({ location: `${location}.${childKey}`, key: childKey, item: childValue });
    }
    recommendationItems(childValue, childKey, `${location}.${childKey}`, items);
  }
  return items;
}

/** @param {JsonObject} rule */
function hasRecommendationBasis(rule) {
  return [rule?.marker_ids, rule?.rsids, rule?.matched_marker_ids].some((value) => isStringArray(value) && value.length > 0)
    || [rule?.genes, rule?.gene_symbols].some((value) => isStringArray(value) && value.length > 0);
}

/** @param {JsonObject} rule @param {JsonObject} marker */
function actionabilityMatchesMarker(rule, marker) {
  const markerRsid = normalizedRsid(marker?.rsid);
  const ruleMarkerIds = [rule?.marker_ids, rule?.rsids, rule?.matched_marker_ids]
    .filter(isStringArray)
    .flat()
    .map((item) => normalizedRsid(item) || text(item).toLowerCase())
    .filter(Boolean);
  if (markerRsid && ruleMarkerIds.includes(markerRsid)) return true;
  const markerGenes = new Set(geneTokens(marker?.gene));
  const ruleGenes = [rule?.genes, rule?.gene_symbols].filter(isStringArray).flat().flatMap(geneTokens);
  return ruleGenes.some((gene) => markerGenes.has(gene));
}

/** @param {JsonObject} [laypersonTranslations] */
function buildTranslationIndex(laypersonTranslations = {}) {
  const translated = new Set();
  for (const entry of laypersonTranslations?.translations || []) {
    const rsid = normalizedRsid(entry?.rsid);
    if (rsid && text(entry?.simpleMeaning)) translated.add(rsid);
  }
  for (const group of laypersonTranslations?.translation_groups || []) {
    if (!text(group?.simpleMeaning)) continue;
    for (const rsid of group?.rsids || []) {
      const normalized = normalizedRsid(rsid);
      if (normalized) translated.add(normalized);
    }
  }
  return translated;
}

/** @param {JsonObject} marker @param {JsonObject[]} identifierTemplates */
function markerHasIdentifierTemplate(marker, identifierTemplates = []) {
  const variantType = text(marker?.variant_type).toLowerCase();
  if (!variantType) return false;
  return identifierTemplates.some((template) =>
    isStringArray(template?.variant_types)
      && template.variant_types.some((item) => text(item).toLowerCase() === variantType),
  );
}

/** @param {JsonObject} laypersonTranslations */
function inspectSimpleCopyContract(laypersonTranslations = {}) {
  const entries = [
    ...(Array.isArray(laypersonTranslations?.translations) ? laypersonTranslations.translations : []),
    ...(Array.isArray(laypersonTranslations?.translation_groups) ? laypersonTranslations.translation_groups : []),
    ...(Array.isArray(laypersonTranslations?.identifier_templates) ? laypersonTranslations.identifier_templates : []),
  ];
  const missingFields = Object.fromEntries(SIMPLE_COPY_FIELDS.map((field) => [field, 0]));
  let fullyStructured = 0;
  let vagueEntries = 0;
  let overlongEntries = 0;

  for (const entry of entries) {
    let complete = true;
    for (const field of SIMPLE_COPY_FIELDS) {
      if (!text(entry?.[field])) {
        missingFields[field] += 1;
        complete = false;
      }
    }
    if (complete) fullyStructured += 1;
    const legacyMeaning = text(entry?.simpleMeaning);
    if (SIMPLE_COPY_VAGUE_PATTERNS.some((pattern) => pattern.test(legacyMeaning))) vagueEntries += 1;
    if (legacyMeaning.length > 360) overlongEntries += 1;
  }

  return {
    required_fields: [...SIMPLE_COPY_FIELDS],
    authored_entries: entries.length,
    fully_structured_entries: fullyStructured,
    legacy_entries_needing_migration: entries.length - fullyStructured,
    missing_fields: missingFields,
    vague_entries: vagueEntries,
    overlong_entries: overlongEntries,
  };
}

/** @param {JsonObject} marker @param {Set<string>} translatedIds @param {boolean} fallbackAvailable @param {JsonObject[]} identifierTemplates */
function markerHasPlainMeaning(marker, translatedIds, fallbackAvailable, identifierTemplates = []) {
  if (text(marker?.simpleMeaning) || text(marker?.simple_meaning)) return 'authored';
  if (translatedIds.has(normalizedRsid(marker?.rsid))) return 'authored';
  if (markerHasIdentifierTemplate(marker, identifierTemplates)) return 'templated';
  return fallbackAvailable ? 'fallback' : 'missing';
}

/** @param {JsonObject} marker @param {JsonObject[]} actionabilityRules */
function markerHasAction(marker, actionabilityRules) {
  const directKeys = ['simpleAction', 'simple_action', 'next_step', 'next_helpful_step', 'action'];
  if (directKeys.some((key) => text(marker?.[key]))) return true;
  if (isStringArray(marker?.confirm_with) && marker.confirm_with.some((item) => text(item))) return true;
  return actionabilityRules.some((rule) => actionabilityMatchesMarker(rule, marker));
}

/** @param {string} packId @param {MarkerRow[]} rows @param {Set<string>} translatedIds @param {boolean} fallbackAvailable @param {JsonObject[]} actionabilityRules @param {JsonObject[]} identifierTemplates */
function packSummary(packId, rows, translatedIds, fallbackAvailable, actionabilityRules, identifierTemplates = []) {
  const standardCounts = new Map();
  for (const row of rows) {
    const rsid = normalizedRsid(row.marker?.rsid);
    if (rsid) addCount(standardCounts, rsid);
  }
  let authoredPlain = 0;
  let templatedPlain = 0;
  let fallbackPlain = 0;
  let missingPlain = 0;
  let missingAction = 0;
  /** @type {CopyEntry[]} */
  const copyEntries = [];
  /** @type {CopyEntry[]} */
  const topicEntries = [];
  for (const row of rows) {
    const plainStatus = markerHasPlainMeaning(row.marker, translatedIds, fallbackAvailable, identifierTemplates);
    if (plainStatus === 'authored') authoredPlain += 1;
    else if (plainStatus === 'templated') templatedPlain += 1;
    else if (plainStatus === 'fallback') fallbackPlain += 1;
    else missingPlain += 1;
    if (!markerHasAction(row.marker, actionabilityRules)) missingAction += 1;
    collectCopyEntries(row.marker, '', `${packId}.markers[${row.index}]`, copyEntries);
    collectTopicLabels(row.marker, '', `${packId}.markers[${row.index}]`, topicEntries);
  }
  return {
    id: packId,
    marker_rows: rows.length,
    unique_standard_rsids: standardCounts.size,
    duplicate_standard_rsids: Array.from(standardCounts.values()).filter((count) => count > 1).length,
    duplicate_standard_rows: Array.from(standardCounts.values()).reduce((total, count) => total + (count > 1 ? count : 0), 0),
    duplicate_standard_excess_rows: Array.from(standardCounts.values()).reduce((total, count) => total + Math.max(count - 1, 0), 0),
    plain_meaning: {
      authored: authoredPlain,
      templated: templatedPlain,
      fallback: fallbackPlain,
      missing: missingPlain,
    },
    findings_without_action: missingAction,
    copy_entries: copyEntries,
    topic_entries: topicEntries,
  };
}

/**
 * Analyze curated marker/support resources using aggregate-only output.
 * @param {{packDocs?: Array<{id:string, markers:JsonObject[]}>, supportResources?: Array<JsonObject>, actionability?: JsonObject, laypersonTranslations?: JsonObject, discoveryCatalog?: JsonObject, sourceRegistry?: JsonObject, reportSurfaces?: Array<JsonObject>}} [input]
 */
export function analyzeContentQuality({
  packDocs = [],
  supportResources = [],
  actionability = {},
  laypersonTranslations = {},
  discoveryCatalog = {},
  sourceRegistry = {},
  reportSurfaces = [],
} = {}) {
  const rows = packDocs.flatMap((pack) => (Array.isArray(pack?.markers) ? pack.markers : [])
    .map((marker, index) => ({ packId: text(pack?.id) || 'unknown', marker, index })));
  const translatedIds = buildTranslationIndex(laypersonTranslations);
  const identifierTemplates = Array.isArray(laypersonTranslations?.identifier_templates)
    ? laypersonTranslations.identifier_templates
    : [];
  const fallbackAvailable = text(laypersonTranslations?.fallback?.simpleMeaning) !== '';
  const simpleCopyContract = inspectSimpleCopyContract(laypersonTranslations);
  const actionabilityRules = Array.isArray(actionability?.rules) ? actionability.rules : [];
  const allCopyEntries = [];
  const allTopicEntries = [];
  const packSummaries = [];

  for (const pack of packDocs) {
    const packRows = rows.filter((row) => row.packId === text(pack?.id));
    const summary = packSummary(text(pack?.id) || 'unknown', packRows, translatedIds, fallbackAvailable, actionabilityRules, identifierTemplates);
    packSummaries.push({
      id: summary.id,
      marker_rows: summary.marker_rows,
      unique_standard_rsids: summary.unique_standard_rsids,
      duplicate_standard_rsids: summary.duplicate_standard_rsids,
      duplicate_standard_rows: summary.duplicate_standard_rows,
      duplicate_standard_excess_rows: summary.duplicate_standard_excess_rows,
      plain_meaning: summary.plain_meaning,
      findings_without_action: summary.findings_without_action,
    });
    allCopyEntries.push(...summary.copy_entries);
    allTopicEntries.push(...summary.topic_entries);
  }

  for (const resource of supportResources) {
    collectCopyEntries(resource?.value ?? resource, '', resource?.id || 'support', allCopyEntries);
    collectTopicLabels(resource?.value ?? resource, '', resource?.id || 'support', allTopicEntries);
  }
  collectCopyEntries(laypersonTranslations, '', 'layperson_translations', allCopyEntries);
  collectCopyEntries(discoveryCatalog, '', 'discovery_catalog', allCopyEntries);

  const copyCounts = new Map();
  const interpretationCounts = new Map();
  for (const entry of allCopyEntries) {
    const normalized = normalizedText(entry.value);
    if (!normalized) continue;
    addCount(copyCounts, normalized);
    if (entry.key === 'interpretation') addCount(interpretationCounts, normalized);
  }

  const genericPatternCounts = new Map(GENERIC_COPY_PATTERNS.map(({ id }) => [id, 0]));
  const genericUniqueValues = new Map(GENERIC_COPY_PATTERNS.map(({ id }) => [id, new Set()]));
  for (const entry of allCopyEntries) {
    for (const { id, pattern } of GENERIC_COPY_PATTERNS) {
      if (pattern.test(entry.value)) {
        genericPatternCounts.set(id, (genericPatternCounts.get(id) || 0) + 1);
        genericUniqueValues.get(id)?.add(normalizedText(entry.value));
      }
    }
  }

  const topicCounts = new Map();
  for (const entry of allTopicEntries) addCount(topicCounts, normalizedText(entry.value));

  const recommendationEntries = actionabilityRules.flatMap((rule, index) => recommendationItems(rule, '', `actionability.rules[${index}]`));
  const recommendationsWithoutBasis = actionabilityRules
    .filter((rule) => recommendationItems(rule).length > 0 && !hasRecommendationBasis(rule)).length;
  const recommendationCount = recommendationEntries.length;
  const repeatedInterpretationReview = classifyRepeatedInterpretations(rows);

  const sourceRegistryIds = new Set(Object.keys(sourceRegistry?.sources || {}));
  const referenceArrays = [
    ...supportResources.map((resource) => resource?.value ?? resource),
    actionability,
    laypersonTranslations,
    discoveryCatalog,
  ].flatMap((resource) => collectStringArrays(resource));
  const invalidReferenceCount = referenceArrays
    .flatMap((entry) => entry.values)
    .filter((id) => !/^https?:\/\//i.test(id) && !sourceRegistryIds.has(id)).length;

  const linkChecks = [
    ...supportResources.flatMap((resource) => collectLinkCountChecks(resource?.value ?? resource, resource?.id || 'support')),
    ...reportSurfaces.flatMap((surface, index) => collectLinkCountChecks(surface, `report_surfaces[${index}]`)),
  ];
  const linkMismatches = linkChecks.filter((check) => check.mismatch).length;
  const multiMarkerGroups = [
    ...supportResources.flatMap((resource) => collectMultiMarkerGroups(resource?.value ?? resource, '', resource?.id || 'support')),
    ...reportSurfaces.flatMap((surface, index) => collectMultiMarkerGroups(surface, '', `report_surfaces[${index}]`)),
  ];

  const plainMeaning = packSummaries.reduce((result, pack) => {
    result.authored += pack.plain_meaning.authored;
    result.templated += pack.plain_meaning.templated;
    result.fallback += pack.plain_meaning.fallback;
    result.missing += pack.plain_meaning.missing;
    return result;
  }, { authored: 0, templated: 0, fallback: 0, missing: 0 });
  const findingsWithoutAction = packSummaries.reduce((total, pack) => total + pack.findings_without_action, 0);
  const standardRsidCounts = new Map();
  for (const row of rows) addCount(standardRsidCounts, normalizedRsid(row.marker?.rsid));
  standardRsidCounts.delete('');

  const errors = [];
  if (invalidReferenceCount > 0) errors.push(`invalid reference IDs detected: ${invalidReferenceCount}`);
  if (plainMeaning.missing > 0) errors.push(`findings missing a required Simple-mode meaning: ${plainMeaning.missing}`);
  if (findingsWithoutAction > 0) errors.push(`findings missing a usable action: ${findingsWithoutAction}`);
  if (linkMismatches > 0) errors.push(`multi-marker link/count mismatches detected: ${linkMismatches}`);

  const warnings = [];
  if (plainMeaning.fallback > 0) warnings.push(`findings using the generic Simple-mode fallback: ${plainMeaning.fallback}`);
  if (simpleCopyContract.legacy_entries_needing_migration > 0) {
    warnings.push(`Simple finding copy entries missing structured fields: ${simpleCopyContract.legacy_entries_needing_migration}`);
  }
  if (simpleCopyContract.vague_entries > 0) warnings.push(`Simple finding copy entries with vague fallback language: ${simpleCopyContract.vague_entries}`);
  if (simpleCopyContract.overlong_entries > 0) warnings.push(`Simple finding copy entries over 360 characters: ${simpleCopyContract.overlong_entries}`);
  if (recommendationsWithoutBasis > 0) warnings.push(`recommendation rules without an explicit marker/gene basis: ${recommendationsWithoutBasis}`);
  const reuseSummary = copyReuseSummary(allCopyEntries, plainMeaning.fallback);
  if (repeatedInterpretationReview.classifications.cross_section_reuse.values > 0) {
    warnings.push(`repeated marker interpretations cross pack sections and need review: ${repeatedInterpretationReview.classifications.cross_section_reuse.values} values / ${repeatedInterpretationReview.classifications.cross_section_reuse.occurrences} occurrences`);
  }

  return {
    marker_rows: rows.length,
    unique_standard_rsids: standardRsidCounts.size,
    duplicate_standard_rsids: Array.from(standardRsidCounts.values()).filter((count) => count > 1).length,
    duplicate_standard_rows: Array.from(standardRsidCounts.values()).reduce((total, count) => total + (count > 1 ? count : 0), 0),
    duplicate_standard_excess_rows: Array.from(standardRsidCounts.values()).reduce((total, count) => total + Math.max(count - 1, 0), 0),
    copy: {
      candidate_entries: allCopyEntries.length,
      repeated_visible_phrases: mapToRepeatedSummary(copyCounts),
      generic_interpretation_phrases: mapToRepeatedSummary(interpretationCounts),
      repeated_interpretation_review: repeatedInterpretationReview,
      reuse_classes: reuseSummary,
      generic_boundary_hits: Object.fromEntries(Array.from(genericPatternCounts.entries()).map(([id, count]) => [
        id,
        { entries: count, unique: genericUniqueValues.get(id)?.size || 0 },
      ])),
    },
    topics: mapToRepeatedSummary(topicCounts),
    plain_meaning: {
      ...plainMeaning,
      translated_standard_rsids: translatedIds.size,
      fallback_available: fallbackAvailable,
    },
    simple_copy_contract: simpleCopyContract,
    findings_without_action: findingsWithoutAction,
    recommendations: {
      total: recommendationCount,
      rules: actionabilityRules.length,
      rules_without_marker_basis: recommendationsWithoutBasis,
    },
    references: {
      registered_source_ids: sourceRegistryIds.size,
      checked_arrays: referenceArrays.length,
      invalid_ids: invalidReferenceCount,
    },
    multi_marker: {
      groups: multiMarkerGroups.length,
      groups_with_duplicate_references: multiMarkerGroups.filter((group) => group.duplicate_references > 0).length,
      duplicate_references: multiMarkerGroups.reduce((total, group) => total + (Number.isFinite(group.duplicate_references) ? group.duplicate_references : 0), 0),
      link_count_checks: linkChecks.length,
      link_count_mismatches: linkMismatches,
    },
    pack_summaries: packSummaries,
    errors,
    warnings,
  };
}
