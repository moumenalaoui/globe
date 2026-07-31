import { useEffect, useState } from 'react'
import Globe from './components/Globe'
import CountrySidebar from './components/CountrySidebar'
import BlockingHeatmap from './components/BlockingHeatmap'
import { getCountries, getCountry } from './lib/api'
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
  const [countries, setCountries] = useState([])
  const [countriesError, setCountriesError] = useState('')
  const [selectedCode, setSelectedCode] = useState('')
  const [selectedCountry, setSelectedCountry] = useState(null)
  const [selectionError, setSelectionError] = useState('')
  const [globeError, setGlobeError] = useState('')
  const [isLoadingCountries, setIsLoadingCountries] = useState(true)
  const [isLoadingSelection, setIsLoadingSelection] = useState(false)
  const [layer, setLayer] = useState('ALL')

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
            onCountrySelect={setSelectedCode}
            onLoadError={setGlobeError}
            layer={layer}
          />

          <BlockingHeatmap />

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

        {selectedCode && (
          <aside style={{ width: 380, flexShrink: 0, height: '100%', overflow: 'hidden' }}>
            {selectedCountry ? (
              <CountrySidebar
                key={selectedCode}
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
