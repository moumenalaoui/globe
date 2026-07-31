import { useEffect, useState } from 'react'
import { getRankings } from '../lib/api'
import { AMBER, BORDER, CRIMSON, DIM, HIGHLIGHT, LOCAL, MONO, MUTED, SIDEBAR, WHITE } from '../theme'

const SOURCES = [
  { key: 'V_DEM', short: 'V-DEM', label: 'V-Dem Freedom of Expression' },
  { key: 'RSF', short: 'RSF', label: 'RSF Press Freedom' },
]

// All scores are 0–100, higher = more free. In a "most censored" list the top
// rows are the lowest scores, so short crimson bars read as most repressive.
function scoreColor(score) {
  if (score >= 60) return LOCAL
  if (score >= 35) return AMBER
  return CRIMSON
}

function SourceToggle({ active, short, onClick }) {
  return (
    <button
      type="button"
      onClick={onClick}
      style={{
        background: 'transparent',
        border: `1px solid ${active ? HIGHLIGHT : BORDER}`,
        color: active ? HIGHLIGHT : MUTED,
        fontFamily: MONO,
        fontSize: 9,
        letterSpacing: '0.08em',
        padding: '2px 6px',
        cursor: 'pointer',
      }}
    >
      {short}
    </button>
  )
}

export default function GlobalRanking() {
  const [source, setSource] = useState('V_DEM')
  const [rows, setRows] = useState([])
  const [collapsed, setCollapsed] = useState(false)
  const [error, setError] = useState(false)

  useEffect(() => {
    let cancelled = false
    setError(false)

    getRankings({ source, order: 'asc', limit: 200 })
      .then((data) => {
        if (!cancelled) setRows(data)
      })
      .catch(() => {
        if (!cancelled) {
          setRows([])
          setError(true)
        }
      })

    return () => {
      cancelled = true
    }
  }, [source])

  if (error && rows.length === 0) return null

  const meta = SOURCES.find((s) => s.key === source)
  const year = rows[0]?.year

  return (
    <div
      style={{
        position: 'absolute',
        left: 12,
        // Expanded: span the full left edge (top-left is now free) so far more
        // of the ranking shows before scrolling. Collapsed: drop the top anchor
        // so the box shrinks to just its header instead of a tall empty panel.
        top: collapsed ? 'auto' : 12,
        bottom: 12,
        width: 264,
        display: 'flex',
        flexDirection: 'column',
        background: SIDEBAR,
        border: `1px solid ${BORDER}`,
        zIndex: 5,
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '8px 10px',
          borderBottom: collapsed ? 'none' : `1px solid ${BORDER}`,
        }}
      >
        <button
          type="button"
          onClick={() => setCollapsed((c) => !c)}
          style={{ background: 'transparent', border: 'none', padding: 0, cursor: 'pointer', textAlign: 'left', flex: 1 }}
        >
          <span style={{ fontFamily: MONO, fontSize: 10, letterSpacing: '0.1em', color: WHITE }}>
            MOST CENSORED COUNTRIES
          </span>
        </button>
        <div style={{ display: 'flex', gap: 4 }}>
          {SOURCES.map((s) => (
            <SourceToggle
              key={s.key}
              short={s.short}
              active={source === s.key}
              onClick={() => setSource(s.key)}
            />
          ))}
        </div>
        <button
          type="button"
          onClick={() => setCollapsed((c) => !c)}
          style={{ background: 'transparent', border: 'none', color: MUTED, fontFamily: MONO, fontSize: 10, cursor: 'pointer' }}
        >
          {collapsed ? '+' : '\u2212'}
        </button>
      </div>

      {!collapsed && (
        <>
          <div style={{ flex: 1, minHeight: 0, overflowY: 'auto' }}>
            {rows.map((r, i) => {
              const color = scoreColor(r.score_overall)
              return (
                <div
                  key={r.country_code}
                  title={`${r.country_name} — ${Math.round(r.score_overall)}/100 free${r.classification ? ` · ${r.classification}` : ''}`}
                  style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '4px 10px' }}
                >
                  <span style={{ fontFamily: MONO, fontSize: 9, color: MUTED, width: 18, flexShrink: 0, textAlign: 'right' }}>
                    {i + 1}
                  </span>
                  <span
                    style={{ fontSize: 10, color: WHITE, width: 96, flexShrink: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}
                  >
                    {r.country_name}
                  </span>
                  <div style={{ flex: 1, height: 6, background: DIM, position: 'relative' }}>
                    <div style={{ position: 'absolute', left: 0, top: 0, bottom: 0, width: `${Math.max(2, Math.min(100, r.score_overall))}%`, background: color }} />
                  </div>
                  <span style={{ fontFamily: MONO, fontSize: 9, color, width: 20, flexShrink: 0, textAlign: 'right' }}>
                    {Math.round(r.score_overall)}
                  </span>
                </div>
              )
            })}
          </div>
          <div style={{ padding: '6px 10px', borderTop: `1px solid ${BORDER}`, fontFamily: MONO, fontSize: 8, color: MUTED, letterSpacing: '0.05em' }}>
            {rows.length} countries · {meta?.label}{year ? ` ${year}` : ''} · 0–100, higher = freer
          </div>
        </>
      )}
    </div>
  )
}
