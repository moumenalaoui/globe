import { useEffect, useMemo, useState } from 'react'
import Globe from './components/Globe'
import CountrySidebar from './components/CountrySidebar'
import BlockingHeatmap from './components/BlockingHeatmap'
import OutageFeed from './components/OutageFeed'
import GlobalRanking from './components/GlobalRanking'
import CommandBar from './components/CommandBar'
import StatusBar from './components/StatusBar'
import { buildBlockingMap } from './lib/blockingRegistry'
import { getBlocking, getCountries, getCountry, getGeo, getOutages } from './lib/api'
import { BASE, BORDER, MONO, MUTED, SIDEBAR } from './theme'
import './App.css'

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
  // Timestamp of the most recent successful primary fetch. Display-only: drives
  // the "LAST SYNC" readout in the status bar and nothing else. Stamped in each
  // load effect below; it changes when data actually arrives, never on a timer.
  const [lastSync, setLastSync] = useState(null)

  useEffect(() => {
    let cancelled = false

    async function loadCountries() {
      setIsLoadingCountries(true)
      setCountriesError('')

      try {
        const result = await getCountries()
        if (cancelled) return

        setCountries(result)
        setLastSync(Date.now())
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
        if (!cancelled) {
          setBlocking(rows)
          setLastSync(Date.now())
        }
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
        if (!cancelled) {
          setGeo(rows)
          setLastSync(Date.now())
        }
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
        setLastSync(Date.now())
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

  // Command-bar counters — plain reflections of the datasets already loaded
  // above, computed here so the chrome adds no fetches of its own.
  const counts = {
    countries: countries.length,
    signals: blocking?.length ?? 0,
    outages: outages.length,
  }

  // Status-bar link state, derived from the same country-load flags that gate
  // the rest of the UI — not a separate health check.
  const linkStatus = countriesError ? 'error' : isLoadingCountries ? 'loading' : 'ok'

  return (
    <div style={{ background: BASE, height: '100vh', display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
      <CommandBar
        countries={countries}
        selectedCode={selectedCode}
        onSelectCountry={setSelectedCode}
        layer={layer}
        onLayerChange={setLayer}
        counts={counts}
      />

      <div style={{ flex: 1, display: 'flex', minHeight: 0 }}>
        <main style={{ position: 'relative', flex: 1, overflow: 'hidden' }}>
          <Globe
            geoByCode={geoByCode}
            blockingByCode={blockingByCode}
            outages={outages}
            onCountrySelect={setSelectedCode}
            onLoadError={setGlobeError}
            layer={layer}
            selectedCode={selectedCode}
          />

          <BlockingHeatmap countries={countries} rows={blocking} />

          <OutageFeed outages={outages} />

          <GlobalRanking />

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

      <StatusBar status={linkStatus} lastSync={lastSync} />
    </div>
  )
}
