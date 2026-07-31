import { useEffect, useRef, useState } from 'react'
import * as Cesium from 'cesium'
import * as topojson from 'topojson-client'
import 'cesium/Build/Cesium/Widgets/widgets.css'
import { AMBER, BLACK, CRIMSON, TIER_COLORS } from '../theme'

// Read from the environment rather than inlined here: anything in this file
// ships to the browser *and* to git. The Viewer below runs with
// `imageryProvider: false` and no terrain provider, so no Ion asset is
// actually requested and an unset token is harmless — it's wired up only so
// enabling Ion imagery later doesn't require hardcoding a credential again.
const ION_TOKEN = import.meta.env.VITE_CESIUM_ION_TOKEN
if (ION_TOKEN) {
  Cesium.Ion.defaultAccessToken = ION_TOKEN
}

// ISO numeric codes (world-atlas) -> ISO alpha-2
const COUNTRY_CODES = {
  '364': 'IR',  // Iran
  '760': 'SY',  // Syria
  '784': 'AE',  // UAE
  '682': 'SA',  // Saudi Arabia
  '368': 'IQ',  // Iraq
}

// Supported country centroids — marker + arc-origin positions
const COUNTRY_CENTROIDS = {
  IR: [53.7, 32.4],
  SY: [38.5, 35.0],
  AE: [53.8, 23.4],
  SA: [45.0, 23.9],
  IQ: [43.7, 33.2],
}

// Fly-to altitudes per country on click
const FLY_ALTITUDE = {
  IR: 2200000,
  SY: 1200000,
  AE: 900000,
  SA: 2500000,
  IQ: 1500000,
}

const PULSE_PERIOD_MS = 2500
const FLARE_DURATION_MS = 500

// Ranks blocking status severity so a country/layer with mixed technology
// statuses is represented by its single worst one.
const BLOCKING_SEVERITY = { CONFIRMED_BLOCKED: 3, LIKELY_BLOCKED: 2, ACCESSIBLE: 1, INCONCLUSIVE: 0 }

function worstStatus(rows) {
  if (rows.length === 0) return 'NO_DATA'
  return rows.reduce((worst, row) => {
    const rank = BLOCKING_SEVERITY[row.status] ?? -1
    const worstRank = BLOCKING_SEVERITY[worst] ?? -1
    return rank > worstRank ? row.status : worst
  }, rows[0].status)
}

function buildBlockingMap(rows, countryCodes) {
  const map = {}
  countryCodes.forEach((code) => {
    const forCountry = rows.filter((r) => r.country_code === code)
    map[code] = {
      ALL: worstStatus(forCountry),
      AI_ACCESS: worstStatus(forCountry.filter((r) => r.category === 'AI_ACCESS')),
      CIRCUMVENTION: worstStatus(forCountry.filter((r) => r.category === 'CIRCUMVENTION')),
    }
  })
  return map
}

function drawRingCanvas(hex) {
  const size = 64
  const canvas = document.createElement('canvas')
  canvas.width = size
  canvas.height = size
  const ctx = canvas.getContext('2d')
  ctx.strokeStyle = hex
  ctx.lineWidth = 4
  ctx.beginPath()
  ctx.arc(size / 2, size / 2, size / 2 - 4, 0, Math.PI * 2)
  ctx.stroke()
  return canvas
}

function drawDotCanvas(hex) {
  const size = 32
  const canvas = document.createElement('canvas')
  canvas.width = size
  canvas.height = size
  const ctx = canvas.getContext('2d')
  ctx.fillStyle = hex
  ctx.beginPath()
  ctx.arc(size / 2, size / 2, size / 2 - 2, 0, Math.PI * 2)
  ctx.fill()
  return canvas
}

export default function Globe({ onCountrySelect, onLoadError, layer = 'ALL' }) {
  const containerRef = useRef(null)
  const viewerRef = useRef(null)
  const markerStateRef = useRef({})
  const blockingMapRef = useRef({})
  const [ready, setReady] = useState(false)

  // Keep the latest callbacks in refs so the init effect (which only runs
  // once) always calls the current prop without needing to re-run.
  const onCountrySelectRef = useRef(onCountrySelect)
  const onLoadErrorRef     = useRef(onLoadError)
  useEffect(() => { onCountrySelectRef.current = onCountrySelect }, [onCountrySelect])
  useEffect(() => { onLoadErrorRef.current = onLoadError }, [onLoadError])

  useEffect(() => {
    if (!containerRef.current) return

    let cancelled = false
    let viewer
    let removePreRender

    const init = async () => {
      viewer = new Cesium.Viewer(containerRef.current, {
        // `baseLayer: false` (not the pre-1.107 `imageryProvider: false`) is
        // what suppresses the base imagery now. Cesium silently ignores the
        // old key and falls back to Ion World Imagery, which both requires an
        // Ion token and paints over the intended black globe.
        baseLayer:            false,
        baseLayerPicker:      false,
        geocoder:             false,
        homeButton:           false,
        sceneModePicker:      false,
        navigationHelpButton: false,
        animation:            false,
        timeline:             false,
        fullscreenButton:     false,
        infoBox:              false,
        selectionIndicator:   false,
        creditContainer:      Object.assign(document.createElement('div'), { style: 'display:none' }),
      })

      // Bail out if this effect was cleaned up (e.g. React StrictMode
      // double-invoke) while the Viewer was being constructed.
      if (cancelled) {
        viewer.destroy()
        return
      }

      viewer.scene.globe.enableLighting = false
      viewer.scene.globe.baseColor = Cesium.Color.BLACK
      viewer.scene.globe.showGroundAtmosphere = false
      viewer.scene.skyAtmosphere.show = false
      viewer.scene.backgroundColor = Cesium.Color.fromCssColorString(BLACK)
      viewer.scene.sun.show  = false
      viewer.scene.moon.show = false

      viewerRef.current = viewer

      // Fly to MENA on load
      viewer.camera.flyTo({
        destination: Cesium.Cartesian3.fromDegrees(45.0, 28.0, 4200000),
        duration: 0,
      })

      // Fetch tier data from backend
      const countriesResponse = await fetch('/api/countries')
      if (!countriesResponse.ok) throw new Error('Failed to load countries for globe view')
      const countries = await countriesResponse.json()
      if (cancelled) return
      const tierMap = Object.fromEntries(countries.map(c => [c.country_code, c.sanctions_tier]))

      // Blocking status per country/layer, used to recolor markers below.
      // Non-fatal if it fails — markers just fall back to tier coloring.
      try {
        const blockingResponse = await fetch('/api/blocking')
        const blockingRows = blockingResponse.ok ? await blockingResponse.json() : []
        blockingMapRef.current = buildBlockingMap(blockingRows, Object.keys(COUNTRY_CENTROIDS))
      } catch {
        blockingMapRef.current = {}
      }
      if (cancelled) return

      // Load accurate, locally-bundled world borders from world-atlas
      const worldData = await import('world-atlas/countries-50m.json')
      if (cancelled) return
      const geojson = topojson.feature(worldData.default, worldData.default.objects.countries)

      // Thin outline-only borders for every country — no choropleth fills.
      const outlineCollection = new Cesium.PolylineCollection()
      viewer.scene.primitives.add(outlineCollection)

      const addOutlineRing = (coords) => {
        const positions = coords.map(([lng, lat]) => Cesium.Cartesian3.fromDegrees(lng, lat, 2000))
        outlineCollection.add({
          positions,
          width: 0.5,
          material: Cesium.Material.fromType('Color', {
            color: Cesium.Color.fromCssColorString('#ffffff').withAlpha(0.15),
          }),
        })
      }

      geojson.features.forEach((feature) => {
        if (feature.geometry.type === 'Polygon') {
          addOutlineRing(feature.geometry.coordinates[0])
        } else if (feature.geometry.type === 'MultiPolygon') {
          feature.geometry.coordinates.forEach((poly) => addOutlineRing(poly[0]))
        }
      })

      // Pulsing intelligence markers at each supported country centroid.
      const markerState = markerStateRef.current

      Object.entries(COUNTRY_CENTROIDS).forEach(([code, [lng, lat]]) => {
        const tier = tierMap[code]
        if (!tier) return
        const hex = TIER_COLORS[tier]
        const position = Cesium.Cartesian3.fromDegrees(lng, lat)

        const ringEntity = viewer.entities.add({
          position,
          properties: { code },
          billboard: {
            image: drawRingCanvas(hex),
            width: 24,
            height: 24,
            color: Cesium.Color.fromCssColorString(hex),
            disableDepthTestDistance: Number.POSITIVE_INFINITY,
          },
        })

        const dotEntity = viewer.entities.add({
          position,
          properties: { code },
          billboard: {
            image: drawDotCanvas(hex),
            width: 10,
            height: 10,
            color: Cesium.Color.fromCssColorString(hex),
            disableDepthTestDistance: Number.POSITIVE_INFINITY,
          },
        })

        markerState[code] = {
          ringEntity,
          dotEntity,
          tierColor: hex,
          effectiveColor: hex,
          effectiveAlpha: 1,
          hovered: false,
          flareStart: null,
        }
      })

      const preRenderCallback = () => {
        const now = performance.now()
        const cycle = (now % PULSE_PERIOD_MS) / PULSE_PERIOD_MS

        Object.values(markerState).forEach((m) => {
          const alpha = m.effectiveAlpha ?? 1
          const colorHex = m.effectiveColor ?? m.tierColor

          m.ringEntity.billboard.scale = 1 + 0.4 * cycle
          m.ringEntity.billboard.color = Cesium.Color.fromCssColorString(colorHex).withAlpha((1 - cycle) * alpha)

          let dotScale = 1
          if (m.flareStart != null) {
            const elapsed = now - m.flareStart
            if (elapsed >= FLARE_DURATION_MS) {
              m.flareStart = null
            } else {
              const t = elapsed / FLARE_DURATION_MS
              dotScale = t < 0.4 ? 1 + 2 * t : 1.8 - (1.8 - 1) * ((t - 0.4) / 0.6)
            }
          }

          const baseColor = Cesium.Color.fromCssColorString(colorHex).withAlpha(alpha)
          m.dotEntity.billboard.scale = dotScale * (m.hovered ? 1.15 : 1)
          m.dotEntity.billboard.color = m.hovered
            ? Cesium.Color.clone(baseColor).brighten(0.5, new Cesium.Color())
            : baseColor
        })
      }

      viewer.scene.preRender.addEventListener(preRenderCallback)
      removePreRender = () => viewer.scene.preRender.removeEventListener(preRenderCallback)

      // Hover effect
      const handler = new Cesium.ScreenSpaceEventHandler(viewer.scene.canvas)
      let hoveredCode = null

      handler.setInputAction(({ endPosition }) => {
        const picked = viewer.scene.pick(endPosition)
        const code   = picked?.id?.properties?.code?.getValue()

        if (code === hoveredCode) return

        if (hoveredCode && markerState[hoveredCode]) {
          markerState[hoveredCode].hovered = false
        }

        hoveredCode = code || null

        if (hoveredCode && markerState[hoveredCode]) {
          markerState[hoveredCode].hovered = true
          viewer.scene.canvas.style.cursor = 'pointer'
        } else {
          viewer.scene.canvas.style.cursor = 'default'
        }
      }, Cesium.ScreenSpaceEventType.MOUSE_MOVE)

      // Click → flare, fly to country, report selection up
      handler.setInputAction(({ position }) => {
        const picked = viewer.scene.pick(position)
        const code   = picked?.id?.properties?.code?.getValue()
        if (!code || !markerState[code]) return

        markerState[code].flareStart = performance.now()

        const [lng, lat] = COUNTRY_CENTROIDS[code]
        viewer.camera.flyTo({
          destination: Cesium.Cartesian3.fromDegrees(lng, lat, FLY_ALTITUDE[code] ?? 2000000),
          duration: 1.2,
        })

        onCountrySelectRef.current?.(code)
      }, Cesium.ScreenSpaceEventType.LEFT_CLICK)

      setReady(true)
    }

    init().catch(error => {
      console.error(error)
      onLoadErrorRef.current?.(error instanceof Error ? error.message : 'Failed to initialize globe')
    })

    return () => {
      cancelled = true
      removePreRender?.()
      if (viewer && !viewer.isDestroyed()) {
        viewer.destroy()
      }
      if (viewerRef.current === viewer) {
        viewerRef.current = null
      }
    }
  }, [])

  // Recolor markers for the active layer: blocked technologies override the
  // tier color, while no-data/inconclusive countries dim rather than change hue.
  useEffect(() => {
    if (!ready) return

    Object.entries(markerStateRef.current).forEach(([code, m]) => {
      const status = blockingMapRef.current[code]?.[layer] ?? 'NO_DATA'

      if (status === 'CONFIRMED_BLOCKED') {
        m.effectiveColor = CRIMSON
        m.effectiveAlpha = 1
      } else if (status === 'LIKELY_BLOCKED') {
        m.effectiveColor = AMBER
        m.effectiveAlpha = 1
      } else if (status === 'ACCESSIBLE') {
        m.effectiveColor = m.tierColor
        m.effectiveAlpha = 1
      } else {
        m.effectiveColor = m.tierColor
        m.effectiveAlpha = 0.4
      }
    })
  }, [layer, ready])

  return <div ref={containerRef} className="w-full h-full" />
}
