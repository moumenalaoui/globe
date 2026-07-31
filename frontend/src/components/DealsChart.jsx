import { useEffect, useState } from 'react'
import { humanize } from '../lib/humanize'
import { AMBER, BORDER, CN_EXPOSURE, DIM, LOCAL, MONO, MUTED, US_EXPOSURE, WHITE } from '../theme'

// Which layer of the stack a deal touches is the substantive distinction —
// a chip supply agreement and a model-licensing MoU are not the same kind of
// dependency, even when both are called a "partnership".
const LAYER_COLORS = {
  COMPUTE: US_EXPOSURE,
  CHIPS: US_EXPOSURE,
  INFRASTRUCTURE: AMBER,
  CLOUD: AMBER,
  MODEL: CN_EXPOSURE,
  APPLICATION: LOCAL,
  DATA_CENTER: AMBER,
}

function layerColor(layer) {
  return LAYER_COLORS[layer] ?? DIM
}

export default function DealsChart({ countryCode, countryName }) {
  const [deals, setDeals] = useState(null)
  const [error, setError] = useState(false)

  useEffect(() => {
    let cancelled = false
    setDeals(null)
    setError(false)

    fetch(`/api/deals?country=${countryCode}`)
      .then((r) => {
        if (!r.ok) throw new Error('Failed to fetch deals')
        return r.json()
      })
      .then((data) => {
        if (!cancelled) setDeals(data)
      })
      .catch(() => {
        if (!cancelled) setError(true)
      })

    return () => {
      cancelled = true
    }
  }, [countryCode])

  if (error || !deals) return null

  // An explicit empty state rather than `return null`, unlike the other
  // panels: "no bilateral compute deals" is itself the finding for a
  // sanctioned country, and silently omitting the section would read as
  // missing data instead of an absent relationship.
  if (deals.length === 0) {
    return (
      <p style={{ fontFamily: MONO, fontSize: 10, color: MUTED, lineHeight: 1.5 }}>
        No recorded bilateral compute deals for {countryName}.
      </p>
    )
  }

  const sorted = [...deals].sort((a, b) => b.deal_date.localeCompare(a.deal_date))

  const byLayer = sorted.reduce((acc, d) => {
    acc[d.stack_layer] = (acc[d.stack_layer] ?? 0) + 1
    return acc
  }, {})
  const layers = Object.entries(byLayer).sort((a, b) => b[1] - a[1])

  return (
    <div style={{ width: '100%' }}>
      <div style={{ display: 'flex', height: 6, marginBottom: 4 }}>
        {layers.map(([layer, count]) => (
          <div
            key={layer}
            title={`${humanize(layer)}: ${count}`}
            style={{ width: `${(count / sorted.length) * 100}%`, background: layerColor(layer) }}
          />
        ))}
      </div>

      <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', marginBottom: 10 }}>
        {layers.map(([layer, count]) => (
          <div key={layer} style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
            <div style={{ width: 6, height: 6, background: layerColor(layer), flexShrink: 0 }} />
            <span style={{ fontFamily: MONO, fontSize: 9, color: MUTED }}>
              {humanize(layer)} {count}
            </span>
          </div>
        ))}
      </div>

      <div style={{ display: 'flex', flexDirection: 'column' }}>
        {sorted.map((deal) => (
          <div key={deal.deal_id} style={{ padding: '6px 0', borderTop: `1px solid ${BORDER}` }}>
            <div style={{ display: 'flex', alignItems: 'baseline', gap: 6 }}>
              <div
                style={{
                  width: 6,
                  height: 6,
                  background: layerColor(deal.stack_layer),
                  flexShrink: 0,
                  transform: 'translateY(-1px)',
                }}
              />
              <span style={{ flex: 1, minWidth: 0, fontSize: 11, color: WHITE, overflowWrap: 'anywhere' }}>
                {deal.partner_name}
              </span>
              <span style={{ fontFamily: MONO, fontSize: 9, color: MUTED, flexShrink: 0 }}>
                {deal.deal_date.slice(0, 7)}
              </span>
            </div>

            <div style={{ fontFamily: MONO, fontSize: 9, color: MUTED, marginTop: 1, paddingLeft: 12 }}>
              {humanize(deal.deal_type)} · {humanize(deal.partner_type)}
            </div>

            <p style={{ fontSize: 10, lineHeight: 1.4, color: MUTED, marginTop: 3, paddingLeft: 12 }}>
              {deal.description}
            </p>
          </div>
        ))}
      </div>
    </div>
  )
}
