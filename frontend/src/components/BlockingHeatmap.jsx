import { Fragment, useState } from 'react'
import { BORDER, MONO, MUTED, SIDEBAR, WHITE } from '../theme'
import { BLOCKING_REGISTRY, BLOCKING_STATUS_COLOR } from '../lib/blockingRegistry'

const ALL_TECHNOLOGIES = Object.values(BLOCKING_REGISTRY).flat()

// `countries` and `rows` come from App, which owns both responses — this used
// to fetch /api/countries and /api/blocking itself, duplicating what its parent
// and the globe had already requested.
export default function BlockingHeatmap({ countries, rows }) {
  const [collapsed, setCollapsed] = useState(false)

  if (!countries?.length || !rows) return null

  const byKey = Object.fromEntries(rows.map((r) => [`${r.country_code}:${r.technology}`, r]))

  // A row earns its place in the matrix only if at least one country has a
  // resolved status for it — an all-gray row (every country inconclusive or
  // unmeasured) is noise, not signal.
  const meaningfulTechnologies = ALL_TECHNOLOGIES.filter((tech) =>
    countries.some((c) => {
      const row = byKey[`${c.country_code}:${tech}`]
      return !!row && row.measurement_count > 0 && row.status !== 'INCONCLUSIVE'
    }),
  )

  if (meaningfulTechnologies.length === 0) return null

  return (
    <div
      style={{
        position: 'absolute',
        left: 24,
        top: 16,
        background: SIDEBAR,
        border: `1px solid ${BORDER}`,
        padding: '10px 12px',
        maxWidth: collapsed ? 'none' : 420,
      }}
    >
      <button
        type="button"
        onClick={() => setCollapsed((c) => !c)}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          background: 'transparent',
          border: 'none',
          padding: 0,
          marginBottom: collapsed ? 0 : 8,
          cursor: 'pointer',
          fontFamily: MONO,
          fontSize: 10,
          letterSpacing: '0.1em',
          color: MUTED,
        }}
      >
        <span style={{ color: WHITE, width: 10 }}>{collapsed ? '▸' : '▾'}</span>
        REGIONAL BLOCKING MATRIX
      </button>

      {!collapsed && (
        <div style={{ display: 'grid', gridTemplateColumns: `110px repeat(${countries.length}, 22px)`, rowGap: 3, columnGap: 3, alignItems: 'center' }}>
          <div />
          {countries.map((c) => (
            <div key={c.country_code} style={{ fontFamily: MONO, fontSize: 9, color: MUTED, textAlign: 'center' }}>
              {c.country_code}
            </div>
          ))}

          {meaningfulTechnologies.map((tech) => (
            <Fragment key={tech}>
              <div style={{ fontFamily: MONO, fontSize: 9, color: WHITE, paddingRight: 6 }}>
                {tech}
              </div>
              {countries.map((c) => {
                const row = byKey[`${c.country_code}:${tech}`]
                const status = row?.status ?? 'NO_DATA'
                const color = BLOCKING_STATUS_COLOR[status] ?? BORDER
                return (
                  <div
                    key={`${tech}-${c.country_code}`}
                    title={`${c.country_name} / ${tech}: ${status.replaceAll('_', ' ')}${row ? ` (${Math.round((row.anomaly_rate ?? 0) * 100)}% anomaly, n=${row.measurement_count})` : ''}`}
                    style={{ width: 18, height: 12, background: color, margin: '0 auto' }}
                  />
                )
              })}
            </Fragment>
          ))}
        </div>
      )}
    </div>
  )
}
