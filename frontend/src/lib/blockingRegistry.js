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
