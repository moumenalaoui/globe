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

// One tick per month rather than one per day — data spans ~2.5 years daily,
// so a tick per row would be unreadable. Picks the first row seen in each
// year-month, relying on the API's date-ascending order.
function monthlyTicks(rows) {
  const seen = new Set()
  const ticks = []
  for (const row of rows) {
    const month = row.date.slice(0, 7)
    if (!seen.has(month)) {
      seen.add(month)
      ticks.push(row.date)
    }
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
    </div>
  )
}

