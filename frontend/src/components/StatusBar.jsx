import { useEffect, useState } from 'react'
import { AMBER, BORDER, BORDER_STRONG, CRIMSON, LOCAL, MONO, MUTED, SIDEBAR, WHITE } from '../theme'
import { SOURCES } from '../lib/sources'

// Honest link state: mirrors whether the primary country fetch is in flight,
// succeeded, or errored — not a fabricated socket. App derives `status` from
// the same load state that drives everything else.
const LINK = {
  loading: { color: AMBER, label: 'SYNCING' },
  ok: { color: LOCAL, label: 'LINK OK' },
  error: { color: CRIMSON, label: 'LINK ERR' },
}

function relSync(ms) {
  if (!ms) return '—'
  const s = Math.max(0, Math.round((Date.now() - ms) / 1000))
  if (s < 60) return `${s}s ago`
  const m = Math.floor(s / 60)
  if (m < 60) return `${m}m ${String(s % 60).padStart(2, '0')}s ago`
  const h = Math.floor(m / 60)
  return `${h}h ${String(m % 60).padStart(2, '0')}m ago`
}

export default function StatusBar({ status = 'ok', lastSync = null }) {
  // Local 1s tick so "LAST SYNC" counts up without re-rendering the app. The
  // value itself (lastSync) only changes when App actually completes a fetch.
  const [, setTick] = useState(0)
  useEffect(() => {
    const id = setInterval(() => setTick((t) => t + 1), 1000)
    return () => clearInterval(id)
  }, [])

  const link = LINK[status] ?? LINK.ok

  return (
    <footer
      style={{
        height: 24,
        flexShrink: 0,
        display: 'flex',
        alignItems: 'center',
        gap: 10,
        padding: '0 16px',
        background: SIDEBAR,
        borderTop: `1px solid ${BORDER}`,
        fontFamily: MONO,
        fontSize: 9,
        letterSpacing: '0.08em',
      }}
    >
      <span style={{ color: MUTED }}>SOURCES</span>
      <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        {SOURCES.map((source, i) => (
          <span key={source.id} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            {i > 0 && <span style={{ color: BORDER_STRONG }}>·</span>}
            <span style={{ color: WHITE }}>{source.label}</span>
          </span>
        ))}
      </span>

      <span style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 6 }}>
        <span style={{ width: 6, height: 6, borderRadius: '50%', background: link.color }} />
        <span style={{ color: link.color }}>{link.label}</span>
      </span>
      <span style={{ width: 1, height: 12, background: BORDER }} />
      <span style={{ color: MUTED }}>
        LAST SYNC <span className="tabular" style={{ color: WHITE }}>{relSync(lastSync)}</span>
      </span>
    </footer>
  )
}
