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

// Age of the underlying data, from /health. `last_updated` is stored as a
// plain date, so whole days is the honest resolution — a seconds-level counter
// here would imply a precision the pipeline does not have.
function formatAge(dataAge) {
  if (!dataAge || dataAge.age_days == null) return '—'
  const days = dataAge.age_days
  if (days <= 0) return 'today'
  return days === 1 ? '1 day old' : `${days} days old`
}

export default function StatusBar({ status = 'ok', dataAge = null }) {
  const link = LINK[status] ?? LINK.ok
  // The backend already decided what counts as stale (HEALTH_MAX_AGE_DAYS);
  // don't duplicate the threshold here, just colour by its verdict.
  const stale = dataAge?.status === 'stale'

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
      <span
        style={{ color: MUTED, cursor: dataAge?.newest_data ? 'help' : 'default' }}
        title={
          dataAge?.newest_data
            ? `Newest fetched data: ${dataAge.newest_data} · stale after ${dataAge.max_age_days} day(s)`
            : 'No fetched data yet'
        }
      >
        DATA AGE{' '}
        <span className="tabular" style={{ color: stale ? CRIMSON : WHITE }}>
          {formatAge(dataAge)}
        </span>
      </span>
    </footer>
  )
}
