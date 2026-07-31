import { useEffect, useState } from 'react'
import { AMBER, CRIMSON, DIM, MONO, MUTED, US_EXPOSURE, WHITE } from '../theme'

// Freedom on the Net methodology: each sub-score has its own maximum, and
// higher is freer. Showing raw values against a single shared axis (as a
// grouped bar chart would) makes Access look artificially small next to
// Rights — normalizing each against its own max is what makes them
// comparable at a glance.
const METRICS = [
  {
    key: 'score_access',
    max: 25,
    label: 'Access',
    color: US_EXPOSURE,
    description: 'Obstacles to access: infrastructure, connectivity cost, and shutdowns that limit getting online at all.',
  },
  {
    key: 'score_content',
    max: 35,
    label: 'Content',
    color: AMBER,
    description: 'Limits on content: blocking, filtering, and platform manipulation that restrict what can be published or reached.',
  },
  {
    key: 'score_rights',
    max: 40,
    label: 'Rights',
    color: CRIMSON,
    description: 'Violations of user rights: legal protections, surveillance, and repercussions users face for online activity.',
  },
]

export default function FreedomHouseChart({ countryCode, countryName }) {
  const [score, setScore] = useState(null)
  const [error, setError] = useState(false)

  useEffect(() => {
    let cancelled = false
    setScore(null)
    setError(false)

    fetch(`/api/country-scores?country=${countryCode}`)
      .then((r) => (r.ok ? r.json() : []))
      .then((rows) => {
        if (cancelled) return
        const fh = rows.find((row) => row.source === 'FREEDOM_HOUSE')
        setScore(fh ?? null)
      })
      .catch(() => {
        if (!cancelled) setError(true)
      })

    return () => {
      cancelled = true
    }
  }, [countryCode])

  if (error) return null
  if (score === null) return null

  return (
    <div style={{ width: '100%' }}>
      <div style={{ fontFamily: MONO, fontSize: 10, letterSpacing: '0.1em', color: MUTED, marginBottom: 8 }}>
        FREEDOM HOUSE FOTN
      </div>

      <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', marginBottom: 10 }}>
        <span style={{ fontSize: 12, color: WHITE }}>{countryName} — Freedom on the Net {score.year}</span>
        <span style={{ fontFamily: MONO, fontSize: 11, color: MUTED }}>
          {score.score_overall}/100 · {score.classification}
        </span>
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
        {METRICS.map((metric) => {
          const value = score[metric.key]
          if (value == null) return null
          const pct = Math.max(0, Math.min(100, (value / metric.max) * 100))

          return (
            <div key={metric.key}>
              <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 3 }}>
                <span style={{ fontFamily: MONO, fontSize: 10, letterSpacing: '0.05em', color: WHITE }}>{metric.label}</span>
                <span style={{ fontFamily: MONO, fontSize: 10, color: MUTED }}>{value}/{metric.max}</span>
              </div>
              <div style={{ height: 6, background: DIM, position: 'relative' }}>
                <div style={{ position: 'absolute', left: 0, top: 0, bottom: 0, width: `${pct}%`, background: metric.color }} />
              </div>
              <p style={{ marginTop: 3, fontSize: 10, lineHeight: 1.4, color: MUTED }}>{metric.description}</p>
            </div>
          )
        })}
      </div>
    </div>
  )
}
