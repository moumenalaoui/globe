import { BLACK, BORDER, CRIMSON, MONO, MUTED, SANS, SIDEBAR, WHITE } from '../theme'

const Divider = () => <span style={{ width: 1, height: 22, background: BORDER, flexShrink: 0 }} />

function CountrySelect({ value, options, onChange }) {
  return (
    <select
      aria-label="Country"
      value={value}
      onChange={(event) => onChange(event.target.value)}
      style={{
        height: 30,
        background: BLACK,
        border: `1px solid ${BORDER}`,
        borderRadius: 0,
        color: WHITE,
        fontFamily: MONO,
        fontSize: 11,
        letterSpacing: '0.05em',
        padding: '0 8px',
        outline: 'none',
        minWidth: 150,
      }}
    >
      <option value="" disabled style={{ background: BLACK, color: MUTED }}>
        Select country
      </option>
      {options.map((option) => (
        <option key={option.value} value={option.value} style={{ background: BLACK, color: WHITE }}>
          {option.label}
        </option>
      ))}
    </select>
  )
}

// A single labelled counter. Outages turn crimson when non-zero so an active
// disruption is legible from the command bar without opening the feed.
function Stat({ label, value, alert }) {
  return (
    <span style={{ display: 'flex', flexDirection: 'column', lineHeight: 1.15 }}>
      <span style={{ fontFamily: MONO, fontSize: 8, letterSpacing: '0.14em', color: MUTED }}>{label}</span>
      <span
        className="tabular"
        style={{ fontFamily: MONO, fontSize: 13, letterSpacing: '0.03em', color: alert ? CRIMSON : WHITE }}
      >
        {value}
      </span>
    </span>
  )
}

// Presentational command bar. Every interactive control is a controlled input
// driven by props from App — the state and data flow are unchanged from the
// header this replaces.
export default function CommandBar({ countries, selectedCode, onSelectCountry, counts }) {
  return (
    <header
      style={{
        height: 50,
        flexShrink: 0,
        display: 'flex',
        alignItems: 'center',
        gap: 16,
        padding: '0 16px',
        background: SIDEBAR,
        borderBottom: `1px solid ${BORDER}`,
      }}
    >
      <span style={{ fontFamily: SANS, fontWeight: 600, fontSize: 13, letterSpacing: '0.14em', color: WHITE }}>
        CENSORSHIP TRACKER
      </span>

      <Divider />

      <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
        <CountrySelect
          value={selectedCode}
          options={countries.map((c) => ({ value: c.country_code, label: c.country_name }))}
          onChange={onSelectCountry}
        />
      </div>

      <div style={{ marginLeft: 'auto', display: 'flex', gap: 16 }}>
        <Stat label="COUNTRIES" value={counts.countries} />
        <Stat label="SIGNALS" value={counts.signals} />
        <Stat label="OUTAGES" value={counts.outages} alert={counts.outages > 0} />
      </div>
    </header>
  )
}
