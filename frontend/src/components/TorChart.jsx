import { useEffect, useState } from 'react'
import {
  ComposedChart,
  Area,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ReferenceArea,
  ReferenceLine,
  ResponsiveContainer,
} from 'recharts'
import { BORDER, CRIMSON, MONO, MUTED, SIDEBAR, US_EXPOSURE, WHITE } from '../theme'

// Most labels this axis will ever draw.
//
// One tick per month was already a big reduction from one per day, but the
// series spans ~2.5 years, so it still produced 31 labels. Recharts renders
// every tick handed to it explicitly — it does not thin them — and 31 ×
// "2026-07" cannot fit the 380px sidebar, so they collided and clipped. Six is
// what reads cleanly at this width.
const MAX_TICKS = 6

// One tick per month, then thinned to at most MAX_TICKS by taking every Nth.
// The last month is always kept: the right edge is where the eye lands to ask
// "how current is this?", and dropping it to satisfy the stride is the one
// omission a reader would actually notice.
function monthlyTicks(rows) {
  const seen = new Set()
  const months = []
  for (const row of rows) {
    const month = row.date.slice(0, 7)
    if (!seen.has(month)) {
      seen.add(month)
      months.push(row.date)
    }
  }
  if (months.length <= MAX_TICKS) return months

  const stride = Math.ceil(months.length / MAX_TICKS)
  const ticks = months.filter((_, i) => i % stride === 0)
  const last = months[months.length - 1]
  if (ticks[ticks.length - 1] !== last) {
    // Replace rather than append, so the stride never leaves two labels
    // adjacent enough to overlap again at the right edge.
    if (ticks.length >= MAX_TICKS) ticks.pop()
    ticks.push(last)
  }
  return ticks
}

// Consecutive HIGH_BLOCKING dates (within 2 days of each other) collapse
// into one event range instead of one reference line per day — a single
// blocking episode of 10 days shouldn't render as 10 separate markers.
function groupConsecutiveDates(dates, maxGapDays = 2) {
  if (dates.length === 0) return []
  const sorted = [...dates].sort()
  const groups = [[sorted[0]]]

  for (let i = 1; i < sorted.length; i++) {
    const gapDays = (new Date(sorted[i]) - new Date(sorted[i - 1])) / 86_400_000
    if (gapDays <= maxGapDays) {
      groups[groups.length - 1].push(sorted[i])
    } else {
      groups.push([sorted[i]])
    }
  }

  return groups.map((group) => ({ start: group[0], end: group[group.length - 1] }))
}

// Tor publishes per-transport figures as a low/high interval, not a count.
// The midpoint is what's shown; both bounds ride along in the API response and
// in the title tooltip, so the published uncertainty isn't lost.
const TRANSPORTS = [
  { label: 'obfs4', low: 'obfs4_low', high: 'obfs4_high' },
  { label: 'Snowflake', low: 'snowflake_low', high: 'snowflake_high' },
  { label: 'webtunnel', low: 'webtunnel_low', high: 'webtunnel_high' },
]

function midpoint(low, high) {
  if (low == null && high == null) return null
  if (low == null) return high
  if (high == null) return low
  return Math.round((low + high) / 2)
}

// The combined CSV lags the per-country one by a day or two, so the last row
// overall often has no transport split. Walk back to the most recent row that
// actually carries one rather than rendering an empty breakdown.
function latestWithTransports(rows) {
  for (let i = rows.length - 1; i >= 0; i--) {
    const row = rows[i]
    if (TRANSPORTS.some((t) => row[t.low] != null || row[t.high] != null)) return row
  }
  return null
}

function formatMonthYear(dateStr) {
  const d = new Date(dateStr)
  if (Number.isNaN(d.getTime())) return dateStr
  return d.toLocaleDateString('en-US', { month: 'short', year: 'numeric' })
}

export default function TorChart({ countryCode }) {
  const [rows, setRows] = useState(null)
  const [error, setError] = useState(false)

  useEffect(() => {
    let cancelled = false
    setRows(null)
    setError(false)

    fetch(`/api/tor-metrics?country=${countryCode}`)
      .then((r) => {
        if (!r.ok) throw new Error('Failed to fetch tor metrics')
        return r.json()
      })
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

  if (error || !rows || rows.length === 0) return null

  const chartData = rows.map((row) => ({
    date: row.date,
    relay_users: row.relay_users ?? 0,
    bridge_users: row.bridge_users ?? 0,
  }))
  const ticks = monthlyTicks(rows)
  const highBlockingGroups = groupConsecutiveDates(
    rows.filter((r) => r.blocking_signal === 'HIGH_BLOCKING').map((r) => r.date)
  )
  const transportRow = latestWithTransports(rows)

  return (
    <div style={{ width: '100%' }}>
      <div style={{ width: '100%', height: 180 }}>
        <ResponsiveContainer width="100%" height="100%">
          <ComposedChart data={chartData} margin={{ top: 18, right: 4, bottom: 0, left: 0 }}>
            <CartesianGrid stroke={BORDER} vertical={false} />
            <XAxis
              dataKey="date"
              ticks={ticks}
              tickFormatter={(d) => d.slice(0, 7)}
              tick={{ fill: MUTED, fontSize: 9, fontFamily: MONO }}
              axisLine={{ stroke: BORDER }}
              tickLine={false}
            />
            <YAxis
              yAxisId="relay"
              tick={{ fill: MUTED, fontSize: 9, fontFamily: MONO }}
              axisLine={{ stroke: BORDER }}
              tickLine={false}
              width={40}
            />
            <YAxis
              yAxisId="bridge"
              orientation="right"
              tick={{ fill: MUTED, fontSize: 9, fontFamily: MONO }}
              axisLine={{ stroke: BORDER }}
              tickLine={false}
              width={40}
            />
            <Tooltip
              contentStyle={{ background: SIDEBAR, border: `1px solid ${BORDER}`, borderRadius: 0, fontSize: 11, fontFamily: MONO }}
              labelStyle={{ color: WHITE }}
              itemStyle={{ color: MUTED }}
            />
            {highBlockingGroups.map((group) => (
              <ReferenceArea
                key={`area-${group.start}`}
                yAxisId="relay"
                x1={group.start}
                x2={group.end}
                fill={CRIMSON}
                fillOpacity={0.15}
              />
            ))}
            <Area
              yAxisId="relay"
              type="monotone"
              dataKey="relay_users"
              name="Relay users"
              fill={US_EXPOSURE}
              fillOpacity={0.2}
              stroke={US_EXPOSURE}
              strokeWidth={1}
            />
            <Line
              yAxisId="bridge"
              type="monotone"
              dataKey="bridge_users"
              name="Bridge users"
              stroke={CRIMSON}
              strokeWidth={1.5}
              dot={false}
            />
            {highBlockingGroups.map((group) => (
              <ReferenceLine
                key={`line-${group.start}`}
                yAxisId="relay"
                x={group.start}
                stroke={CRIMSON}
                strokeOpacity={0.7}
                label={{
                  value: formatMonthYear(group.start),
                  position: 'top',
                  style: { fontSize: 9, fill: CRIMSON, fontFamily: MONO },
                }}
              />
            ))}
          </ComposedChart>
        </ResponsiveContainer>
      </div>

      <div style={{ display: 'flex', gap: 16, marginTop: 4, fontFamily: MONO, fontSize: 9 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          <div style={{ width: 8, height: 8, background: US_EXPOSURE, flexShrink: 0 }} />
          <span style={{ color: MUTED }}>Relay users (direct)</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
          <div style={{ width: 8, height: 8, background: CRIMSON, flexShrink: 0 }} />
          <span style={{ color: MUTED }}>Bridge users (circumvention)</span>
        </div>
      </div>

      {transportRow && (
        <div style={{ display: 'flex', justifyContent: 'center', gap: 16, marginTop: 6, fontFamily: MONO, fontSize: 9 }}>
          {TRANSPORTS.map((t) => {
            const value = midpoint(transportRow[t.low], transportRow[t.high])
            if (value == null) return null
            const range =
              transportRow[t.low] != null && transportRow[t.high] != null
                ? `${transportRow[t.low].toLocaleString()}–${transportRow[t.high].toLocaleString()}`
                : 'single bound'
            return (
              <div
                key={t.label}
                style={{ display: 'flex', gap: 4, cursor: 'help' }}
                title={`${t.label} bridge users on ${transportRow.date} — Tor estimate range ${range}`}
              >
                <span style={{ color: MUTED }}>{t.label}</span>
                <span style={{ color: WHITE }}>{value.toLocaleString()}</span>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}

