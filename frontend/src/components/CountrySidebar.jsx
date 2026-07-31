import { useEffect, useState } from 'react'
import CategoryBreakdown from './CategoryBreakdown'
import DealsChart from './DealsChart'
import FreedomHouseChart from './FreedomHouseChart'
import GlobalIndices from './GlobalIndices'
import MessagingStatus from './MessagingStatus'
import OutageTimeline from './OutageTimeline'
import TimelineChart from './TimelineChart'
import TorChart from './TorChart'
import {
  BLOCKING_REGISTRY,
  BLOCKING_STATUS_COLOR,
  BLOCKING_STATUS_LABEL,
  GROUP_LABELS,
  hasTimeline,
} from '../lib/blockingRegistry'
import { BORDER, MONO, MUTED, SIDEBAR, WHITE } from '../theme'

const ALL_TECHNOLOGIES = Object.values(BLOCKING_REGISTRY).flat()

function BlockSegments({ filledCount, color }) {
  return (
    <div style={{ display: 'flex', gap: 1, flexShrink: 0 }}>
      {Array.from({ length: 8 }).map((_, i) => (
        <div key={i} style={{ width: 6, height: 6, background: i < filledCount ? color : BORDER }} />
      ))}
    </div>
  )
}

// A row is worth showing only if it carries an actual signal: either the
// point-in-time classification resolved to something other than
// inconclusive/no-data, or there's a historical timeline behind it. A blank
// "INCONCLUSIVE / 0" row tells a reader nothing and just adds noise.
function isMeaningful(row, timelineRows) {
  const hasPointSignal = !!row && row.measurement_count > 0 && row.status !== 'INCONCLUSIVE'
  const hasTimelineSignal = (timelineRows?.length ?? 0) > 0
  return hasPointSignal || hasTimelineSignal
}

function BlockingTechRow({ tech, row, countryCode, timelineRows }) {
  const status = row?.status ?? 'NO_DATA'
  const count = row?.measurement_count ?? 0
  const anomalyRate = row?.anomaly_rate ?? 0
  const color = BLOCKING_STATUS_COLOR[status] ?? BORDER
  const filledCount = Math.max(0, Math.min(8, Math.round(anomalyRate * 8)))
  const showTimeline = hasTimeline(countryCode, tech) && (timelineRows?.length ?? 0) > 0

  return (
    <div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, height: 22 }}>
        <span style={{ fontFamily: MONO, fontSize: 10, color: WHITE, width: 100, flexShrink: 0 }}>{tech}</span>
        <BlockSegments filledCount={filledCount} color={color} />
        <span style={{ fontFamily: MONO, fontSize: 10, color, width: 72, flexShrink: 0 }}>
          {BLOCKING_STATUS_LABEL[status]}
        </span>
        <span style={{ fontFamily: MONO, fontSize: 10, color: MUTED }}>{count}</span>
      </div>
      {showTimeline && (
        <div style={{ padding: '6px 0 6px 0' }}>
          <TimelineChart rows={timelineRows} />
        </div>
      )}
    </div>
  )
}

export default function CountrySidebar({ country, layer, onClose }) {
  const [blockingRows, setBlockingRows] = useState([])
  const [timelineByTech, setTimelineByTech] = useState({})

  useEffect(() => {
    let cancelled = false
    setBlockingRows([])

    fetch(`/api/blocking?country=${country.country_code}`)
      .then((response) => (response.ok ? response.json() : []))
      .then((rows) => {
        if (!cancelled) setBlockingRows(rows)
      })
      .catch(() => {
        if (!cancelled) setBlockingRows([])
      })

    return () => {
      cancelled = true
    }
  }, [country.country_code])

  useEffect(() => {
    let cancelled = false
    setTimelineByTech({})

    const promoted = ALL_TECHNOLOGIES.filter((tech) => hasTimeline(country.country_code, tech))
    Promise.all(
      promoted.map((tech) =>
        fetch(`/api/timeline?country=${country.country_code}&technology=${tech}`)
          .then((response) => (response.ok ? response.json() : []))
          .then((rows) => [tech, rows])
          .catch(() => [tech, []]),
      ),
    ).then((entries) => {
      if (!cancelled) setTimelineByTech(Object.fromEntries(entries))
    })

    return () => {
      cancelled = true
    }
  }, [country.country_code])

  const blockingByTech = Object.fromEntries(blockingRows.map((row) => [row.technology, row]))
  const groupsForLayer = layer === 'AI_ACCESS' || layer === 'CIRCUMVENTION'
    ? [layer]
    : ['AI_ACCESS', 'CIRCUMVENTION', 'PRIVACY_OS']

  const visibleGroups = groupsForLayer
    .map((group) => ({
      group,
      techs: BLOCKING_REGISTRY[group].filter((tech) => isMeaningful(blockingByTech[tech], timelineByTech[tech])),
    }))
    .filter(({ techs }) => techs.length > 0)

  return (
    <div
      style={{
        width: 380,
        minWidth: 340,
        height: '100%',
        overflowY: 'auto',
        background: SIDEBAR,
        borderLeft: `1px solid ${BORDER}`,
      }}
    >
      <div style={{ padding: '16px 20px', borderBottom: `1px solid ${BORDER}` }}>
        <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between' }}>
          <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
            <h2 style={{ fontSize: 14, fontWeight: 500, color: WHITE }}>{country.country_name}</h2>
            <span style={{ fontFamily: MONO, fontSize: 11, letterSpacing: '0.05em', color: MUTED }}>
              {country.country_code}
            </span>
          </div>
          <button
            onClick={onClose}
            style={{ background: 'transparent', border: 'none', color: MUTED, fontSize: 18, lineHeight: 1, cursor: 'pointer' }}
          >
            ×
          </button>
        </div>

      </div>

      <div style={{ padding: '16px 20px', display: 'flex', flexDirection: 'column', gap: 18 }}>
        {visibleGroups.length > 0 && (
          <section>
            <div style={{ fontFamily: MONO, fontSize: 10, letterSpacing: '0.1em', color: MUTED, marginBottom: 8 }}>
              BLOCKING STATUS
            </div>
            {visibleGroups.map(({ group, techs }) => (
              <div key={group} style={{ marginBottom: 10 }}>
                <p
                  style={{
                    fontFamily: MONO,
                    fontSize: 9,
                    letterSpacing: '0.1em',
                    textTransform: 'uppercase',
                    color: MUTED,
                    marginBottom: 2,
                  }}
                >
                  {GROUP_LABELS[group]}
                </p>
                {techs.map((tech) => (
                  <BlockingTechRow
                    key={tech}
                    tech={tech}
                    row={blockingByTech[tech]}
                    countryCode={country.country_code}
                    timelineRows={timelineByTech[tech]}
                  />
                ))}
              </div>
            ))}
          </section>
        )}

        {/* Tor relay/bridge usage. Rendered from its own /api/tor-metrics data
            (it returns null when empty) rather than gated on a "meaningful" tor
            blocking row, so it surfaces for every country that has Tor data —
            not just those with a point-in-time blocking classification. */}
        <TorChart countryCode={country.country_code} />

        <MessagingStatus countryCode={country.country_code} />

        <CategoryBreakdown countryCode={country.country_code} />

        <OutageTimeline countryCode={country.country_code} />

        <FreedomHouseChart countryCode={country.country_code} countryName={country.country_name} />

        <GlobalIndices countryCode={country.country_code} />

        <section>
          <div style={{ fontFamily: MONO, fontSize: 10, letterSpacing: '0.1em', color: MUTED, marginBottom: 8 }}>
            BILATERAL COMPUTE DEALS
          </div>
          <DealsChart countryCode={country.country_code} countryName={country.country_name} />
        </section>
      </div>
    </div>
  )
}
