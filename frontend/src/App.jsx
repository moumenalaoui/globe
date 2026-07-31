import { useEffect, useMemo, useState } from 'react'
import Globe from './components/Globe'
import CountrySidebar from './components/CountrySidebar'
import BlockingHeatmap from './components/BlockingHeatmap'
import OutageFeed from './components/OutageFeed'
import { buildBlockingMap } from './lib/blockingRegistry'
import { getBlocking, getCountries, getCountry, getGeo, getOutages } from './lib/api'
import { BLACK, BORDER, CRIMSON, HIGHLIGHT, MONO, MUTED, SIDEBAR, WHITE } from './theme'
import './App.css'

const LAYER_OPTIONS = [
  { value: 'AI_ACCESS', label: 'AI ACCESS' },
  { value: 'CIRCUMVENTION', label: 'CIRCUMVENTION' },
  { value: 'ALL', label: 'ALL' },
]

// Literal per-spec toggle colors — close to, but distinct from, the rest of
// the app's border/muted tokens, so kept local rather than folded into theme.
const TOGGLE_BORDER = '#1e3258'
const TOGGLE_MUTED = '#8892a4'

function LayerToggle({ label, active, onClick }) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={active}
      style={{
        background: 'transparent',
        border: `1px solid ${active ? HIGHLIGHT : TOGGLE_BORDER}`,
        color: active ? HIGHLIGHT : TOGGLE_MUTED,
        fontFamily: MONO,
        fontSize: 10,
        letterSpacing: '0.1em',
        padding: '4px 10px',
        borderRadius: 0,
        cursor: 'pointer',
      }}
    >
      {label}
    </button>
  )
}

function TopBarSelect({ ariaLabel, value, options, placeholder, onChange }) {
  return (
    <select
      aria-label={ariaLabel}
      value={value}
      onChange={(event) => onChange(event.target.value)}
      style={{
        height: 32,
        background: BLACK,
        border: `1px solid ${BORDER}`,
        borderRadius: 0,
        color: WHITE,
        fontFamily: MONO,
        fontSize: 11,
        letterSpacing: '0.05em',
        padding: '0 8px',
        outline: 'none',
      }}
    >
      {placeholder && (
        <option value="" disabled style={{ background: BLACK, color: MUTED }}>
          {placeholder}
        </option>
      )}
      {options.map((option) => (
        <option key={option.value} value={option.value} style={{ background: BLACK, color: WHITE }}>
          {option.label}
        </option>
      ))}
    </select>
  )
}

export default function App() {
  // `countries` and `blocking` are owned here and passed down, rather than
  // fetched independently by Globe and BlockingHeatmap. Three components
  // deriving state from the same two responses is what made the app request
  // /api/countries three times and /api/blocking twice on every load.
  const [countries, setCountries] = useState([])
  const [blocking, setBlocking] = useState(null)
  const [countriesError, setCountriesError] = useState('')
  const [selectedCode, setSelectedCode] = useState('')
  const [selectedCountry, setSelectedCountry] = useState(null)
  const [selectionError, setSelectionError] = useState('')
  const [globeError, setGlobeError] = useState('')
  const [isLoadingCountries, setIsLoadingCountries] = useState(true)
  const [isLoadingSelection, setIsLoadingSelection] = useState(false)
  const [layer, setLayer] = useState('ALL')
  const [geo, setGeo] = useState([])
  const [outages, setOutages] = useState([])

  useEffect(() => {
    let cancelled = false

    async function loadCountries() {
      setIsLoadingCountries(true)
      setCountriesError('')

      try {
        const result = await getCountries()
        if (cancelled) return

        setCountries(result)
      } catch (error) {
        if (cancelled) return
        setCountriesError(error instanceof Error ? error.message : 'Failed to load countries')
      } finally {
        if (!cancelled) {
          setIsLoadingCountries(false)
        }
      }
    }

    loadCountries()

    return () => {
      cancelled = true
    }
  }, [])

  // Blocking status for every country/technology. Non-fatal if it fails: the
  // globe falls back to uncoloured markers and the heatmap hides itself, which
  // is why this doesn't feed `statusMessage` the way the country load does.
  useEffect(() => {
    let cancelled = false

    getBlocking()
      .then((rows) => {
        if (!cancelled) setBlocking(rows)
      })
      .catch(() => {
        if (!cancelled) setBlocking([])
      })

    return () => {
      cancelled = true
    }
  }, [])

  // Derived once here and shared by the globe markers and the heatmap, so
  // neither has to re-scan the row set.
  const blockingByCode = useMemo(() => buildBlockingMap(blocking ?? []), [blocking])

  // Country identity + centroid + bbox for every drawable country. The globe
  // needs this to place markers and to derive a fly-to altitude, replacing the
  // hardcoded 5-entry centroid and altitude tables it used to carry.
  useEffect(() => {
    let cancelled = false

    getGeo()
      .then((rows) => {
        if (!cancelled) setGeo(rows)
      })
      .catch(() => {
        if (!cancelled) setGeo([])
      })

    return () => {
      cancelled = true
    }
  }, [])

  const geoByCode = useMemo(
    () => Object.fromEntries(geo.map((g) => [g.country_code, g])),
    [geo],
  )

  // Live internet-outage overlay. Fetches currently-active IODA events plus
  // every country's centroid, then aggregates events to one entry per country
  // (severity = worst score, recency = latest start) for the globe pings and
  // the feed panel. Non-fatal: a failure just leaves the overlay empty.
  useEffect(() => {
    let cancelled = false

    Promise.all([getGeo(), getOutages({ active: true })])
      .then(([geo, events]) => {
        if (cancelled) return

        const centroid = new Map(
          geo.map((g) => [g.country_code, { name: g.country_name, lat: g.centroid_lat, lon: g.centroid_lon }]),
        )

        const byCountry = new Map()
        for (const e of events) {
          const geoInfo = centroid.get(e.country_code)
          if (!geoInfo || geoInfo.lat == null || geoInfo.lon == null) continue
          const existing = byCountry.get(e.country_code)
          if (existing) {
            existing.count += 1
            existing.maxScore = Math.max(existing.maxScore, e.score)
            existing.latestStart = Math.max(existing.latestStart, e.start_ts)
          } else {
            byCountry.set(e.country_code, {
              code: e.country_code,
              name: geoInfo.name,
              lat: geoInfo.lat,
              lon: geoInfo.lon,
              count: 1,
              maxScore: e.score,
              latestStart: e.start_ts,
            })
          }
        }

        const aggregated = [...byCountry.values()].sort((a, b) => b.latestStart - a.latestStart)
        setOutages(aggregated)
      })
      .catch(() => {
        if (!cancelled) setOutages([])
      })

    return () => {
      cancelled = true
    }
  }, [])

  // No auto-selection on load — the globe's default state is intentionally
  // sparse (outlines + pulsing markers) until the user picks a country via
  // the globe or the dropdown.
  useEffect(() => {
    if (!selectedCode) return

    let cancelled = false

    async function loadSelection() {
      setIsLoadingSelection(true)
      setSelectionError('')

      try {
        const country = await getCountry(selectedCode)
        if (cancelled) return

        setSelectedCountry(country)
      } catch (error) {
        if (cancelled) return
        setSelectedCountry(null)
        setSelectionError(error instanceof Error ? error.message : 'Failed to load country')
      } finally {
        if (!cancelled) {
          setIsLoadingSelection(false)
        }
      }
    }

    loadSelection()

    return () => {
      cancelled = true
    }
  }, [selectedCode])

  const statusMessage = countriesError || selectionError || globeError
    || (isLoadingCountries && 'Loading country data...')
    || (isLoadingSelection && 'Refreshing country...')

  return (
    <div style={{ background: BLACK, height: '100vh', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      <header
        style={{
          height: 44,
          flexShrink: 0,
          display: 'flex',
          alignItems: 'center',
          gap: 20,
          padding: '0 16px',
          background: SIDEBAR,
          borderBottom: `1px solid ${BORDER}`,
        }}
      >
        <span style={{ fontFamily: MONO, fontSize: 11, color: WHITE, letterSpacing: '0.1em', whiteSpace: 'nowrap' }}>
          Censorship Tracker 
        </span>

        <div style={{ display: 'flex', gap: 8 }}>
          <TopBarSelect
            ariaLabel="Country"
            value={selectedCode}
            placeholder="Select country"
            options={countries.map((c) => ({ value: c.country_code, label: c.country_name }))}
            onChange={setSelectedCode}
          />
        </div>

        <div style={{ display: 'flex', gap: 6 }}>
          {LAYER_OPTIONS.map((option) => (
            <LayerToggle
              key={option.value}
              label={option.label}
              active={layer === option.value}
              onClick={() => setLayer(option.value)}
            />
          ))}
        </div>

        <span style={{ marginLeft: 'auto', fontFamily: MONO, fontSize: 11, color: CRIMSON, letterSpacing: '0.1em' }}>
        </span>
      </header>

      <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
        <main style={{ position: 'relative', flex: 1, overflow: 'hidden' }}>
          <Globe
            geoByCode={geoByCode}
            blockingByCode={blockingByCode}
            outages={outages}
            onCountrySelect={setSelectedCode}
            onLoadError={setGlobeError}
            layer={layer}
          />

          <BlockingHeatmap countries={countries} rows={blocking} />

          <OutageFeed outages={outages} />

          {statusMessage && (
            <div
              style={{
                position: 'absolute',
                bottom: 16,
                right: 16,
                background: SIDEBAR,
                border: `1px solid ${BORDER}`,
                padding: '6px 10px',
                fontFamily: MONO,
                fontSize: 10,
                letterSpacing: '0.05em',
                color: MUTED,
              }}
            >
              {statusMessage}
            </div>
          )}
        </main>

        {/* CountrySidebar deliberately carries no `key={selectedCode}`: every
            effect inside it and its charts already depends on the country code,
            so remounting the subtree only forced avoidable teardown — and
            refetched country-independent data like /api/models on every
            selection. */}
        {selectedCode && (
          <aside style={{ width: 380, flexShrink: 0, height: '100%', overflow: 'hidden' }}>
            {selectedCountry ? (
              <CountrySidebar
                country={selectedCountry}
                layer={layer}
                onClose={() => {
                  setSelectedCode('')
                  setSelectedCountry(null)
                }}
              />
            ) : (
              <div
                style={{
                  height: '100%',
                  width: '100%',
                  background: SIDEBAR,
                  borderLeft: `1px solid ${BORDER}`,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  textAlign: 'center',
                  padding: '0 24px',
                  fontSize: 13,
                  color: MUTED,
                }}
              >
                Loading country...
              </div>
            )}
          </aside>
        )}
      </div>
    </div>
  )
}
