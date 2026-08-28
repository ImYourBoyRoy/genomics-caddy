import type { EnrichedSource, GeneratedReport, MarkerSource } from '../types/genomics';

export type ReportReferenceSource = MarkerSource | EnrichedSource;

export interface ReportReference {
  id: string;
  title: string;
  organization: string;
  date: string;
  evidenceRole: string;
  url: string | null;
}

export interface ReportReferenceRegistry {
  references: ReportReference[];
  idsByMarker: Map<string, string[]>;
}

function clean(value: unknown, fallback = 'Not recorded'): string {
  const text = String(value ?? '').trim();
  return text || fallback;
}

function normalizedUrl(value: string | undefined): string {
  return value?.trim().replace(/\/$/, '').toLowerCase() || '';
}

/**
 * Keep source identity stable when findings are added or reordered. A sequential
 * index makes exported reference IDs drift between otherwise equivalent reports.
 */
export function reportReferenceKey(source: ReportReferenceSource): string {
  const url = normalizedUrl(source.url);
  if (url) return `url:${url}`;
  if ('name' in source) {
    return [source.name, source.evidence_type || '', source.notes || '']
      .map((value) => clean(value))
      .join('|')
      .toLowerCase();
  }
  return [source.source_type, source.citation, source.details || '']
    .map((value) => clean(value))
    .join('|')
    .toLowerCase();
}

function hashReferenceKey(key: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < key.length; index += 1) {
    hash ^= key.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, '0').toUpperCase();
}

export function reportReferenceId(source: ReportReferenceSource): string {
  return `REF-${hashReferenceKey(reportReferenceKey(source))}`;
}

export function reportReferenceFromSource(
  source: ReportReferenceSource,
  id = reportReferenceId(source),
): ReportReference {
  if ('name' in source) {
    return {
      id,
      title: clean(source.name, 'Reference'),
      organization: clean(source.name, 'Not specified'),
      date: clean(source.accessed, 'Access date not recorded'),
      evidenceRole: clean(source.evidence_type, 'Marker-pack reference'),
      url: source.url?.trim() || null,
    };
  }
  return {
    id,
    title: clean(source.citation, 'Catalog reference'),
    organization: clean(source.source_type, 'Local reference catalog'),
    date: 'Local catalog record',
    evidenceRole: clean(source.details, 'Catalog evidence'),
    url: source.url?.trim() || null,
  };
}

export function buildReferenceEntries(
  sources: MarkerSource[] = [],
  dbSources: EnrichedSource[] = [],
): ReportReference[] {
  const references: ReportReference[] = [];
  const seen = new Set<string>();
  for (const source of [...sources, ...dbSources]) {
    const key = reportReferenceKey(source);
    if (seen.has(key)) continue;
    seen.add(key);
    references.push(reportReferenceFromSource(source));
  }
  return references;
}

export function reportMarkerReferenceKey(sectionName: string, linkId: string): string {
  return `${sectionName}:${linkId}`;
}

export function buildReportReferenceRegistry(report: GeneratedReport): ReportReferenceRegistry {
  const references: ReportReference[] = [];
  const referencesByKey = new Map<string, ReportReference>();
  const idsByMarker = new Map<string, string[]>();

  for (const section of report.sections) {
    for (const marker of section.markers) {
      const markerIds: string[] = [];
      for (const source of [...marker.sources, ...marker.db_enriched_sources]) {
        const key = reportReferenceKey(source);
        let reference = referencesByKey.get(key);
        if (!reference) {
          reference = reportReferenceFromSource(source);
          referencesByKey.set(key, reference);
          references.push(reference);
        }
        markerIds.push(reference.id);
      }
      idsByMarker.set(
        reportMarkerReferenceKey(section.name, marker.link_id),
        [...new Set(markerIds)],
      );
    }
  }

  return { references, idsByMarker };
}
