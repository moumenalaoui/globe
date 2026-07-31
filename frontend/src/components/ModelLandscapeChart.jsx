import { useEffect, useState } from 'react'
import { humanize } from '../lib/humanize'
import { BORDER, CN_EXPOSURE, HIGHLIGHT, LOCAL, MONO, MUTED, US_EXPOSURE, WHITE } from '../theme'

// Origin is the whole point of this panel — the section header calls it a
// global reference, and what a reader needs from it is which bloc's weights
// are actually deployable. Colors reuse the stack-exposure palette so an
// origin here reads the same as an exposure elsewhere in the app.
const ORIGIN_COLORS = {
  US: US_EXPOSURE,
  CN: CN_EXPOSURE,
  FR: LOCAL,
  AE: HIGHLIGHT,
}

const ORIGIN_FALLBACK = MUTED

function originColor(code) {
  return ORIGIN_COLORS[code] ?? ORIGIN_FALLBACK
}

// Deployability drives whether an open-weight model is a real option for a
// constrained actor, so it earns a visible marker rather than a tooltip.
const DEPLOYABILITY_LABELS = {
  LAPTOP_OR_WORKSTATION: 'laptop',
  WORKSTATION_TO_SERVER: 'workstation',
  SERVER: 'server',
  SERVER_REQUIRED: 'server req.',
  CLOUD_ONLY: 'cloud only',
}

export default function ModelLandscapeChart() {
  const [models, setModels] = useState(null)
  const [error, setError] = useState(false)

  useEffect(() => {
    let cancelled = false

    fetch('/api/models')
      .then((r) => {
        if (!r.ok) throw new Error('Failed to fetch models')
        return r.json()
      })
      .then((data) => {
        if (!cancelled) setModels(data)
      })
      .catch(() => {
        if (!cancelled) setError(true)
      })

    return () => {
      cancelled = true
    }
  }, [])

  if (error || !models) return null

  // Closed-API models are excluded by the section's own framing ("open-weight
  // model landscape") — they aren't a self-hosting option at all.
  const openWeight = models
    .filter((m) => m.weight_access === 'OPEN_WEIGHT')
    .sort((a, b) => b.release_date.localeCompare(a.release_date))

  if (openWeight.length === 0) return null

  const byOrigin = openWeight.reduce((acc, m) => {
    acc[m.origin_country] = (acc[m.origin_country] ?? 0) + 1
    return acc
  }, {})
  const origins = Object.entries(byOrigin).sort((a, b) => b[1] - a[1])

  return (
    <div style={{ width: '100%' }}>
      <div style={{ display: 'flex', height: 6, marginBottom: 4 }}>
        {origins.map(([code, count]) => (
          <div
            key={code}
            title={`${code}: ${count}`}
            style={{
              width: `${(count / openWeight.length) * 100}%`,
              background: originColor(code),
            }}
          />
        ))}
      </div>

      <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', marginBottom: 10 }}>
        {origins.map(([code, count]) => (
          <div key={code} style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
            <div style={{ width: 6, height: 6, background: originColor(code), flexShrink: 0 }} />
            <span style={{ fontFamily: MONO, fontSize: 9, color: MUTED }}>
              {code} {count}
            </span>
          </div>
        ))}
      </div>

      <div style={{ display: 'flex', flexDirection: 'column' }}>
        {openWeight.map((model) => (
          <div
            key={model.model_id}
            style={{
              display: 'flex',
              alignItems: 'baseline',
              gap: 6,
              padding: '4px 0',
              borderTop: `1px solid ${BORDER}`,
            }}
          >
            <div
              style={{
                width: 6,
                height: 6,
                background: originColor(model.origin_country),
                flexShrink: 0,
                transform: 'translateY(-1px)',
              }}
            />
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontSize: 11, color: WHITE, overflowWrap: 'anywhere' }}>
                {model.model_name}
              </div>
              <div style={{ fontFamily: MONO, fontSize: 9, color: MUTED }}>
                {model.provider} · {model.parameter_class} ·{' '}
                {DEPLOYABILITY_LABELS[model.local_deployability] ??
                  humanize(model.local_deployability)}
              </div>
            </div>
            <span style={{ fontFamily: MONO, fontSize: 9, color: MUTED, flexShrink: 0 }}>
              {model.release_date.slice(0, 7)}
            </span>
          </div>
        ))}
      </div>
    </div>
  )
}
