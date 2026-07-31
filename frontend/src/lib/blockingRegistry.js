import { AMBER, BORDER, CRIMSON, DIM, LOCAL } from '../theme'

export const BLOCKING_REGISTRY = {
  AI_ACCESS: ['openai.com', 'claude.ai', 'deepseek', 'huggingface'],
  CIRCUMVENTION: ['tor', 'torproject', 'signal', 'i2p', 'psiphon', 'torsf'],
  PRIVACY_OS: ['grapheneos', 'tails'],
}

export const GROUP_LABELS = {
  AI_ACCESS: 'AI access',
  CIRCUMVENTION: 'Circumvention',
  PRIVACY_OS: 'Privacy OS',
}

export const BLOCKING_STATUS_COLOR = {
  CONFIRMED_BLOCKED: CRIMSON,
  LIKELY_BLOCKED: AMBER,
  ACCESSIBLE: LOCAL,
  INCONCLUSIVE: DIM,
  NO_DATA: BORDER,
}

// Ranks status severity so a country/layer with mixed technology statuses is
// represented by its single worst one.
export const BLOCKING_SEVERITY = {
  CONFIRMED_BLOCKED: 3,
  LIKELY_BLOCKED: 2,
  ACCESSIBLE: 1,
  INCONCLUSIVE: 0,
}

const LAYERS = ['AI_ACCESS', 'CIRCUMVENTION']

function worseOf(current, next) {
  if (current == null) return next
  return (BLOCKING_SEVERITY[next] ?? -1) > (BLOCKING_SEVERITY[current] ?? -1) ? next : current
}

/// Collapses per-technology blocking rows into a per-country worst status,
/// overall and per layer.
///
/// Single pass over `rows`. The previous shape took a country roster and ran
/// four full `rows.filter()` sweeps per country, i.e. O(countries x rows) —
/// which is fine for five countries and roughly half a million comparisons for
/// the whole world. Keyed by whatever appears in the data rather than by a
/// supplied roster, since every caller already treats a missing key as NO_DATA.
export function buildBlockingMap(rows) {
  const map = {}
  for (const row of rows) {
    const entry = (map[row.country_code] ??= { ALL: null, AI_ACCESS: null, CIRCUMVENTION: null })
    entry.ALL = worseOf(entry.ALL, row.status)
    // PRIVACY_OS rows still count toward ALL but have no layer of their own,
    // matching the layer toggles the header actually offers.
    if (LAYERS.includes(row.category)) {
      entry[row.category] = worseOf(entry[row.category], row.status)
    }
  }
  for (const entry of Object.values(map)) {
    for (const key of ['ALL', 'AI_ACCESS', 'CIRCUMVENTION']) entry[key] ??= 'NO_DATA'
  }
  return map
}

export const BLOCKING_STATUS_LABEL = {
  CONFIRMED_BLOCKED: 'BLOCKED',
  LIKELY_BLOCKED: 'LIKELY',
  ACCESSIBLE: 'ACCESSIBLE',
  INCONCLUSIVE: 'INCONCLUSIVE',
  NO_DATA: 'NO DATA',
}

// (country, technology) pairs the backend backfills a daily timeline for
// (see TIMELINE_TARGETS in backend/src/fetchers/ooni.rs) — these are the
// only pairs worth rendering a timeline chart for by default. Must be kept
// in sync with the backend list; generated the same way (every circumvention
// technology x every tracked country) rather than hand-listed, so the two
// can't drift apart again.
const TIMELINE_COUNTRIES = ['IR', 'SY', 'AE', 'SA', 'IQ']
const TIMELINE_TECHNOLOGIES = ['torproject', 'signal', 'i2p', 'psiphon', 'torsf']

export const TIMELINE_TARGETS = [
  ...TIMELINE_COUNTRIES.flatMap((country) =>
    TIMELINE_TECHNOLOGIES.map((technology) => ({ country, technology })),
  ),
  { country: 'IR', technology: 'openai.com' },
]

export function hasTimeline(countryCode, technology) {
  return TIMELINE_TARGETS.some((t) => t.country === countryCode && t.technology === technology)
}
