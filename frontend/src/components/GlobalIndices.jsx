import { useEffect, useState } from 'react'
import { AMBER, CRIMSON, DIM, LOCAL, MONO, MUTED, WHITE } from '../theme'

// Which sources this component renders, and how to label them. Freedom House
// is intentionally excluded — FreedomHouseChart already renders its detailed
// sub-scores above this.
const SOURCES = [
  { key: 'V_DEM', label: 'V-Dem — Freedom of Expression' },
  { key: 'RSF', label: 'RSF — Press Freedom' },
]

// All scores are normalised to 0–100, higher = more free, so one colour ramp
// reads consistently across indices.
function scoreColor(score) {
  if (score >= 60) return LOCAL
  if (score >= 35) return AMBER
  return CRIMSON
}

export default function GlobalIndices({ countryCode }) {
  const [rows, setRows] = useState(null)
  const [error, setError] = useState(false)

  useEffect(() => {
    let cancelled = false
    setRows(null)
    setError(false)

    fetch(`/api/country-scores?country=${countryCode}`)
      .then((r) => (r.ok ? r.json() : []))
      .then((data) => {
        if (!cancelled) setRows(data)
      })
      .catch(() => {
        if (!cancelled) setError(true)
      })

    return () => {
      cancelled = true
    }
  }, [countryCode])

  if (error || !rows) return null

  const bySource = Object.fromEntries(rows.map((r) => [r.source, r]))
  const present = SOURCES.map((s) => ({ ...s, row: bySource[s.key] })).filter(
    (s) => s.row && s.row.score_overall != null,
  )

  if (present.length === 0) return null

  return (
    <section>
      <div style={{ fontFamily: MONO, fontSize: 10, letterSpacing: '0.1em', color: MUTED, marginBottom: 8 }}>
        GLOBAL FREEDOM INDICES
      </div>

      <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
        {present.map(({ key, label, row }) => {
          const score = row.score_overall
          const color = scoreColor(score)
          return (
            <div key={key}>
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', marginBottom: 3 }}>
                <span style={{ fontSize: 11, color: WHITE }}>{label}</span>
                <span style={{ fontFamily: MONO, fontSize: 9, color: MUTED }}>{row.year}</span>
              </div>
              <div style={{ height: 6, background: DIM, position: 'relative' }}>
                <div style={{ position: 'absolute', left: 0, top: 0, bottom: 0, width: `${Math.max(0, Math.min(100, score))}%`, background: color }} />
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 3 }}>
                <span style={{ fontFamily: MONO, fontSize: 9, color }}>{row.classification}</span>
                <span style={{ fontFamily: MONO, fontSize: 9, color: MUTED }}>{Math.round(score)}/100 free</span>
              </div>
            </div>
          )
        })}
      </div>

      <div style={{ fontFamily: MONO, fontSize: 8, color: MUTED, letterSpacing: '0.05em', marginTop: 6 }}>
        Normalized so higher = more free · RSF shown is the pre-2022 index
      </div>
    </section>
  )
}
