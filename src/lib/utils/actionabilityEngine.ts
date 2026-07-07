// ./src/lib/utils/actionabilityEngine.ts

import type { GeneratedReport, EvaluatedMarker, SeverityClass } from '../types/genomics';

export interface TopFinding {
  rsid: string;
  gene: string;
  variant_name: string;
  severity_class: SeverityClass;
  interpretation: string;
  section_name: string;
  link_id: string;
}

export interface DietaryGuidance {
  favor: string[];
  avoid: string[];
  notes?: string;
}

export interface SupplementItem {
  name: string;
  reason: string;
}

export interface LabTest {
  name: string;
  reason: string;
  urgency: 'routine' | 'consider' | 'urgent';
  requires_counselor?: boolean;
}

export interface ActionablePlan {
  topFindings: TopFinding[];
  diet: DietaryGuidance;
  supplements: SupplementItem[];
  labTests: LabTest[];
}

interface ActionableRule {
  genes: string[];
  severity_classes?: SeverityClass[];
  interpretation_contains?: string[];
  favor?: string[];
  avoid?: string[];
  supplements?: string[];
  lab_tests?: { name: string; urgency: 'routine' | 'consider' | 'urgent'; requires_counselor?: boolean }[];
  notes?: string;
}

const ACTIONABLE_RULES: ActionableRule[] = [
  {
    genes: ['MTHFR'],
    severity_classes: ['high_risk', 'moderate_risk'],
    favor: ['Leafy greens (spinach, kale)', 'Lentils & beans', 'Organic eggs'],
    avoid: ['Folic acid fortified foods', 'Synthetic multivitamins'],
    supplements: ['Methylfolate (L-5-MTHF)', 'Methylcobalamin (active B12)'],
    lab_tests: [
      { name: 'Homocysteine (plasma)', urgency: 'urgent' },
      { name: 'Serum Folate & Vitamin B12', urgency: 'consider' }
    ]
  },
  {
    genes: ['APOE'],
    interpretation_contains: ['ε4', 'e4', 'APOE4'],
    favor: ['Wild-caught fatty fish (salmon, sardines)', 'Walnuts', 'Extra virgin olive oil', 'High-fiber vegetables'],
    avoid: ['Red meat', 'Saturated fats (butter, coconut oil)', 'Trans fats'],
    supplements: ['High-potency Omega-3 DHA/EPA', 'Vitamin D3 + K2'],
    lab_tests: [
      { name: 'ApoB (Apolipoprotein B)', urgency: 'urgent' },
      { name: 'Lipid Panel (NMR LipoProfile)', urgency: 'consider' },
      { name: 'hs-CRP (Inflammation marker)', urgency: 'consider' }
    ]
  },
  {
    genes: ['FADS1', 'FADS2'],
    severity_classes: ['high_risk', 'moderate_risk'],
    favor: ['Pre-formed DHA/EPA (seafood, fatty fish)', 'Algae oil'],
    avoid: ['Industrial seed oils (canola, corn, soy)', 'High omega-6 foods'],
    supplements: ['Algae-derived DHA/EPA or high-quality fish oil'],
    lab_tests: [
      { name: 'Omega-3 Index (red blood cell membrane)', urgency: 'consider' }
    ]
  },
  {
    genes: ['HFE'],
    severity_classes: ['high_risk', 'moderate_risk'],
    favor: ['Green tea or black tea with meals (inhibits iron)', 'Calcium-rich foods'],
    avoid: ['Red meat', 'Iron-fortified cereals', 'Vitamin C supplements with meals'],
    supplements: ['Avoid any supplements containing iron'],
    lab_tests: [
      { name: 'Ferritin & Transferrin Saturation', urgency: 'urgent' },
      { name: 'Total Iron Binding Capacity (TIBC)', urgency: 'consider' }
    ]
  },
  {
    genes: ['LCT', 'MCM6'],
    severity_classes: ['high_risk', 'moderate_risk'],
    favor: ['Lactose-free alternatives', 'Fermented dairy (kefir, organic Greek yogurt)'],
    avoid: ['Cow milk', 'Ice cream', 'Soft unaged cheeses'],
    supplements: ['Lactase enzyme (when consuming dairy)'],
    lab_tests: [
      { name: 'Hydrogen Breath Test (if symptomatic)', urgency: 'routine' }
    ]
  },
  {
    genes: ['ALDH2'],
    severity_classes: ['high_risk', 'moderate_risk'],
    avoid: ['Alcohol', 'Acetaldehyde exposure'],
    supplements: ['N-Acetyl Cysteine (NAC)', 'Glutathione precursors'],
    lab_tests: [
      { name: 'Liver Enzyme Panel (ALT/AST)', urgency: 'consider' }
    ],
    notes: 'Acetaldehyde accumulates rapidly — even minimal alcohol intake elevates esophageal cancer risk'
  },
  {
    genes: ['SLC2A9', 'ABCG2'],
    severity_classes: ['high_risk', 'moderate_risk'],
    favor: ['Optimal hydration (filtered water)', 'Montmorency tart cherry juice', 'Low-fat organic dairy'],
    avoid: ['High-purine foods (organ meats, shellfish)', 'Beer', 'High-fructose corn syrup'],
    supplements: ['Vitamin C (helps promote uric acid excretion)'],
    lab_tests: [
      { name: 'Serum Uric Acid', urgency: 'urgent' }
    ]
  },
  {
    genes: ['VDR'],
    severity_classes: ['high_risk', 'moderate_risk'],
    favor: ['Wild fish', 'Egg yolks', 'UV-irradiated mushrooms'],
    supplements: ['Vitamin D3 + Vitamin K2 (to direct calcium to bones)'],
    lab_tests: [
      { name: '25-hydroxyvitamin D [25(OH)D]', urgency: 'urgent' },
      { name: 'Ionized Calcium & PTH', urgency: 'consider' }
    ]
  },
  {
    genes: ['GSTT1', 'GSTM1'],
    severity_classes: ['high_risk', 'moderate_risk'],
    favor: ['Cruciferous vegetables (broccoli, brussels sprouts)', 'Allium vegetables (garlic, onions)'],
    supplements: ['Sulforaphane / Broccoli sprout extract', 'Milk Thistle (Silymarin)'],
    lab_tests: [
      { name: 'Comprehensive Liver Panel', urgency: 'routine' }
    ]
  },
  {
    genes: ['COMT'],
    interpretation_contains: ['Met/Met', 'slow COMT', 'AA'],
    favor: ['Magnesium-rich foods (pumpkin seeds, dark chocolate)', 'Cruciferous vegetables'],
    avoid: ['Excessive caffeine', 'Chronic sleep deprivation', 'High-stress environments'],
    supplements: ['Magnesium Glycinate', 'L-Theanine (calming support)'],
    lab_tests: [
      { name: 'Salivary Cortisol Rhythm (4-point)', urgency: 'consider' }
    ]
  },
  {
    genes: ['CYP1A2'],
    interpretation_contains: ['slow metabolizer', 'slow caffeine'],
    avoid: ['Caffeine after 12:00 PM', 'High-dose caffeine stimulants'],
    notes: 'Slow caffeine clearance increases arterial stiffness and cardiovascular risk when consuming >200mg daily'
  },
  {
    genes: ['NOS3'],
    severity_classes: ['high_risk', 'moderate_risk'],
    favor: ['Beetroot / Beet juice', 'Arugula & leafy spinach', 'Pomegranates'],
    supplements: ['L-Citrulline or L-Arginine', 'Nitric Oxide support'],
    lab_tests: [
      { name: 'Cardiovascular Risk Panel / Blood Pressure', urgency: 'consider' }
    ]
  },
  {
    genes: ['GPX1'],
    severity_classes: ['high_risk', 'moderate_risk'],
    favor: ['Brazil nuts (1-2 daily)', 'Seafood', 'Sunflower seeds'],
    supplements: ['Selenium (L-selenomethionine)'],
    lab_tests: [
      { name: 'Selenium level (blood)', urgency: 'routine' }
    ]
  },
  {
    genes: ['BRCA1', 'BRCA2'],
    severity_classes: ['high_risk', 'confirmation_required'],
    lab_tests: [
      { name: 'Clinical BRCA1/2 NGS Confirmation Panel', urgency: 'urgent', requires_counselor: true },
      { name: 'Breast MRI / Mammography Referral', urgency: 'urgent', requires_counselor: true }
    ],
    notes: 'Requires professional genetic counseling support.'
  },
  {
    genes: ['MLH1', 'MSH2', 'MSH6', 'PMS2'],
    severity_classes: ['high_risk', 'confirmation_required'],
    lab_tests: [
      { name: 'Clinical Lynch Syndrome NGS Confirmation Panel', urgency: 'urgent', requires_counselor: true },
      { name: 'Colonoscopy Screening Referral', urgency: 'urgent', requires_counselor: true }
    ],
    notes: 'Requires professional genetic counseling support.'
  }
];

export function deriveActionablePlan(report: GeneratedReport): ActionablePlan {
  const topFindings: TopFinding[] = [];
  const favorSet = new Set<string>();
  const avoidSet = new Set<string>();
  const supplementsMap = new Map<string, string[]>(); // supplement -> reasons[]
  const labTestsMap = new Map<string, { reason: string; urgency: 'routine' | 'consider' | 'urgent'; requires_counselor: boolean }>();
  let overallNotes = '';

  // Gather all markers from the report sections
  const allMarkers: { marker: EvaluatedMarker; sectionName: string }[] = [];
  for (const section of report.sections || []) {
    for (const marker of section.markers || []) {
      allMarkers.push({ marker, sectionName: section.name });
    }
  }

  // 1. Extract Top Findings
  // Rankings:
  // - High Risk: 100 points
  // - Confirmation Required (clinical): 90 points
  // - Moderate Risk: 50 points
  // - Low Risk / Risk: 20 points
  // - Protective: 0 points (not a concern)
  const scoredFindings = allMarkers
    .filter(({ marker }) => marker.interpretation_allowed)
    .map(({ marker, sectionName }) => {
      let score = 0;
      if (marker.severity_class === 'high_risk') score = 100;
      else if (marker.severity_class === 'confirmation_required') score = 90;
      else if (marker.severity_class === 'moderate_risk') score = 50;
      else if (marker.severity_class === 'low_risk') score = 20;

      return {
        score,
        finding: {
          rsid: marker.rsid,
          gene: marker.gene,
          variant_name: marker.variant_name || marker.rsid,
          severity_class: marker.severity_class,
          interpretation: marker.interpretation,
          section_name: sectionName,
          link_id: marker.link_id
        }
      };
    })
    .filter(item => item.score > 0)
    .sort((a, b) => b.score - a.score);

  // Keep top 5 unique findings by gene/rsid combo
  const uniqueFindings: typeof scoredFindings = [];
  const seenKeys = new Set<string>();
  for (const item of scoredFindings) {
    const key = `${item.finding.gene}-${item.finding.rsid}`;
    if (!seenKeys.has(key)) {
      seenKeys.add(key);
      uniqueFindings.push(item);
      if (uniqueFindings.length >= 5) break;
    }
  }
  topFindings.push(...uniqueFindings.map(item => item.finding));

  // 2. Evaluate Rules for Diet, Supplements, and Lab Tests
  for (const rule of ACTIONABLE_RULES) {
    // Find if the user has an active variant matching this rule
    const matchingMarkers = allMarkers.filter(({ marker }) => {
      // Must match one of the genes
      if (!rule.genes.includes(marker.gene)) return false;

      // Must be allowed for interpretation
      if (!marker.interpretation_allowed) return false;

      // Check severity match
      if (rule.severity_classes && !rule.severity_classes.includes(marker.severity_class)) {
        return false;
      }

      // Check interpretation contains match
      if (rule.interpretation_contains) {
        const text = (marker.interpretation || '').toLowerCase();
        const matchesText = rule.interpretation_contains.some(pattern => text.includes(pattern.toLowerCase()));
        if (!matchesText) return false;
      }

      // For rules without specific severity/interpretation constraints, default to requiring some risk effect count > 0
      if (!rule.severity_classes && !rule.interpretation_contains) {
        if (marker.effect_count != null && marker.effect_count === 0) {
          return false;
        }
        if (marker.severity_class === 'benign' || marker.severity_class === 'protective') {
          return false;
        }
      }

      return true;
    });

    if (matchingMarkers.length > 0) {
      // Rule is active! Populate recommendations
      const reason = `Based on your ${rule.genes.join('/')} variant (${matchingMarkers.map(m => m.marker.rsid).join(', ')})`;

      if (rule.favor) {
        rule.favor.forEach(f => favorSet.add(f));
      }
      if (rule.avoid) {
        rule.avoid.forEach(a => avoidSet.add(a));
      }
      if (rule.supplements) {
        rule.supplements.forEach(s => {
          const list = supplementsMap.get(s) || [];
          list.push(reason);
          supplementsMap.set(s, list);
        });
      }
      if (rule.lab_tests) {
        rule.lab_tests.forEach(lt => {
          const existing = labTestsMap.get(lt.name);
          if (existing) {
            // Keep the higher urgency
            const urgencies = { routine: 0, consider: 1, urgent: 2 };
            if (urgencies[lt.urgency] > urgencies[existing.urgency]) {
              existing.urgency = lt.urgency;
            }
            existing.requires_counselor = existing.requires_counselor || !!lt.requires_counselor;
          } else {
            labTestsMap.set(lt.name, {
              reason,
              urgency: lt.urgency,
              requires_counselor: !!lt.requires_counselor
            });
          }
        });
      }
      if (rule.notes) {
        overallNotes += (overallNotes ? '\n' : '') + `• ${rule.genes.join('/')}: ${rule.notes}`;
      }
    }
  }

  // Map to flat lists
  const supplements: SupplementItem[] = Array.from(supplementsMap.entries()).map(([name, reasons]) => ({
    name,
    reason: `${reasons.join('; ')}`
  }));

  const labTests: LabTest[] = Array.from(labTestsMap.entries()).map(([name, val]) => ({
    name,
    reason: val.reason,
    urgency: val.urgency,
    requires_counselor: val.requires_counselor
  })).sort((a, b) => {
    const urgencies = { urgent: 2, consider: 1, routine: 0 };
    return urgencies[b.urgency] - urgencies[a.urgency];
  });

  return {
    topFindings,
    diet: {
      favor: Array.from(favorSet),
      avoid: Array.from(avoidSet),
      notes: overallNotes || undefined
    },
    supplements,
    labTests
  };
}
