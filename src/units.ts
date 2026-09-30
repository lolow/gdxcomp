const UNIT_GROUP = /\[([^\]]+)\]/g;

/** Every `[...]` group of a symbol description, in order. */
export function extractUnits(text: string): string[] {
  return [...text.matchAll(UNIT_GROUP)].map((m) => m[1]);
}

/** Default unit: descriptions listing several units (e.g.
 *  `[T$/TW] [T$/million vehicles] ... (DAC)`) put the main one first. */
export function extractUnit(text: string): string | null {
  return extractUnits(text)[0] ?? null;
}

/** Description split into plain text and unit parts, so units can be rendered as buttons. */
export function unitSegments(text: string): { text: string; unit: boolean }[] {
  const segments: { text: string; unit: boolean }[] = [];
  let last = 0;
  for (const m of text.matchAll(UNIT_GROUP)) {
    if (m.index > last) segments.push({ text: text.slice(last, m.index), unit: false });
    segments.push({ text: m[1], unit: true });
    last = m.index + m[0].length;
  }
  if (last < text.length) segments.push({ text: text.slice(last), unit: false });
  return segments;
}
