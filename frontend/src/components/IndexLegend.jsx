import { BORDER, HIGHLIGHT, MONO, MUTED, SIDEBAR, WHITE } from '../theme'

// Matches the globe choropleth ramp in Globe.jsx (green → amber → crimson).
const RAMP = 'linear-gradient(90deg, #6c9a5b 0%, #d97706 50%, #c8102e 100%)'

export default function IndexLegend({ show, onToggle }) {
  return (
    <div
      style={{
        position: 'absolute',
        bottom: 12,
        left: '50%',
        transform: 'translateX(-50%)',
        background: SIDEBAR,
        border: `1px solid ${BORDER}`,
        padding: '7px 10px',
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        zIndex: 5,
      }}
    >
      <span style={{ fontFamily: MONO, fontSize: 10, letterSpacing: '0.1em', color: WHITE }}>
        CENSORSHIP INDEX
      </span>

      <div style={{ display: 'flex', alignItems: 'center', gap: 6, opacity: show ? 1 : 0.4 }}>
        <span style={{ fontFamily: MONO, fontSize: 9, color: MUTED }}>Free</span>
        <div style={{ width: 120, height: 8, background: RAMP, border: `1px solid ${BORDER}` }} />
        <span style={{ fontFamily: MONO, fontSize: 9, color: MUTED }}>Censored</span>
      </div>

      <button
        type="button"
        onClick={onToggle}
        aria-pressed={show}
        style={{
          background: 'transparent',
          border: `1px solid ${show ? HIGHLIGHT : BORDER}`,
          color: show ? HIGHLIGHT : MUTED,
          fontFamily: MONO,
          fontSize: 9,
          letterSpacing: '0.08em',
          padding: '2px 8px',
          cursor: 'pointer',
        }}
      >
        {show ? 'HIDE' : 'SHOW'}
      </button>

      <span style={{ fontFamily: MONO, fontSize: 8, color: MUTED }}>V-Dem · RSF · FH blend</span>
    </div>
  )
}
