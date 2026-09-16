// ./src/lib/utils/severityGlyphs.ts
/*
Purpose: Shared severity / association glyphs for Map, Report, and Browser.
Responsibilities:
- Map severity_class strings to letter badges and short labels.
- Keep visual vocabulary consistent (C / ! / ? / V / + / T / ~).
*/

export function severityGlyph(severityClass: string): string {
  switch (severityClass) {
    case "high_risk":
      return "!";
    case "moderate_risk":
      return "?";
    case "confirmation_required":
      return "C";
    case "vector_promoted":
      return "V";
    case "protective":
      return "+";
    case "trait":
      return "T";
    case "context_dependent":
      return "~";
    case "low_risk":
      return "?";
    case "not_evaluated":
      return "~";
    default:
      return "·";
  }
}

export function severityShortLabel(severityClass: string): string {
  switch (severityClass) {
    case "high_risk":
      return "Stronger association";
    case "moderate_risk":
      return "Possible association";
    case "confirmation_required":
      return "Confirm clinically";
    case "vector_promoted":
      return "Vector discovery";
    case "protective":
      return "Protective";
    case "trait":
      return "Trait";
    case "context_dependent":
      return "Context-dependent";
    case "low_risk":
      return "Preliminary signal";
    case "no_data":
      return "Not tested";
    case "not_evaluated":
      return "Not evaluated";
    default:
      return "Not detected";
  }
}
