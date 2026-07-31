export const BLACK       = '#000000'
export const SIDEBAR     = '#0a0a0a'
export const BORDER      = '#1e1e1e'
export const WHITE       = '#e6e9ef'
export const MUTED       = '#8c95a3'

export const US_EXPOSURE = '#a0aec0'
export const CN_EXPOSURE = '#c9822b'
export const LOCAL       = '#6c9a5b'
export const CRIMSON     = '#c8102e'
export const HIGHLIGHT   = '#d6b36a'
export const AMBER       = '#d97706'
export const DIM         = '#3d4f6b'

export const MONO        = '"IBM Plex Mono", "Fira Mono", monospace'
export const SANS        = '"Inter", system-ui, sans-serif'

// Tier colors are pinned to the stack-dependency palette rather than an
// arbitrary ramp — blue reads as "safe/trusted" which is wrong for a tier
// defined by foreign compute dependency.
export const TIER_COLORS = {
  COMPREHENSIVELY_SANCTIONED: CRIMSON,
  POST_SANCTIONS_LAG:         CN_EXPOSURE,
  US_COMPUTE_STACK:           US_EXPOSURE,
  RESOURCE_CONSTRAINED:       LOCAL,
}
