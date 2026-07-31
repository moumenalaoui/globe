import { useEffect, useRef, useState } from 'react'
import * as Cesium from 'cesium'
import * as topojson from 'topojson-client'
import 'cesium/Build/Cesium/Widgets/widgets.css'
import { BLACK, CRIMSON, DIM } from '../theme'
import { BLOCKING_STATUS_COLOR } from '../lib/blockingRegistry'

// Read from the environment rather than inlined here: anything in this file
// ships to the browser *and* to git. The Viewer below runs with
// `imageryProvider: false` and no terrain provider, so no Ion asset is
// actually requested and an unset token is harmless — it's wired up only so
// enabling Ion imagery later doesn't require hardcoding a credential again.
const ION_TOKEN = import.meta.env.VITE_CESIUM_ION_TOKEN
if (ION_TOKEN) {
  Cesium.Ion.defaultAccessToken = ION_TOKEN
}

const PULSE_PERIOD_MS = 2500
const FLARE_DURATION_MS = 500

// Outage markers pulse faster than the tier markers so a "live disruption"
// reads as more urgent than the steady baseline presence markers.
const OUTAGE_PULSE_PERIOD_MS = 1400

// Whole-globe framing, centred on ~20°E/15°N rather than 0/0 so the front
// hemisphere on load holds Europe, Africa, the Middle East and South Asia — the
// densest censorship geography — instead of the mid-Atlantic.
const HOME_VIEW = { lon: 20.0, lat: 15.0, height: 20_000_000 }

// Marker colour is blocking status, full stop. This used to be the country's
// sanctions tier, which coupled the globe to researched policy data it does not
// otherwise need, and had two failure modes at scale: an unrecognised tier
// produced `undefined`, and `Cesium.Color.fromCssColorString(undefined)` throws
// out of the init path and takes down the *entire* globe render; and a country
// with no tier at all was silently skipped. Reusing BLOCKING_STATUS_COLOR also
// means a colour on the globe and the same colour in the sidebar status row are
// guaranteed to mean the same thing.
const CESIUM_STATUS_COLOR = Object.fromEntries(
  Object.entries(BLOCKING_STATUS_COLOR).map(([status, hex]) => [
    status,
    Cesium.Color.fromCssColorString(hex),
  ]),
)
const CESIUM_DIM = Cesium.Color.fromCssColorString(DIM)
const CESIUM_CRIMSON = Cesium.Color.fromCssColorString(CRIMSON)

function cesiumColorFor(status) {
  return CESIUM_STATUS_COLOR[status] ?? CESIUM_DIM
}

// "Measured, but unresolved" is a real and different state from "blocked" — it
// gets a small dim point rather than a full pulsing marker, so it reads as
// present-but-inconclusive instead of competing for attention.
function isInconclusive(status) {
  return status === 'INCONCLUSIVE'
}

// Canvases are cached per (kind, hex) instead of created per country. There are
// only a handful of distinct statuses, so this is a bounded set no matter how
// many countries are drawn.
const canvasCache = new Map()

function ringCanvas(hex) {
  const key = `ring:${hex}`
  if (!canvasCache.has(key)) {
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
    canvasCache.set(key, canvas)
  }
  return canvasCache.get(key)
}

function dotCanvas(hex) {
  const key = `dot:${hex}`
  if (!canvasCache.has(key)) {
    const size = 32
    const canvas = document.createElement('canvas')
    canvas.width = size
    canvas.height = size
    const ctx = canvas.getContext('2d')
    ctx.fillStyle = hex
    ctx.beginPath()
    ctx.arc(size / 2, size / 2, size / 2 - 2, 0, Math.PI * 2)
    ctx.fill()
    canvasCache.set(key, canvas)
  }
  return canvasCache.get(key)
}

// Fly-to altitude derived from the country's bounding box rather than a
// hand-authored per-country table, so it frames Vatican City and Russia
// sensibly without anyone maintaining a list. Clamped because a bbox that spans
// the antimeridian or a whole hemisphere would otherwise zoom to orbit.
function altitudeFor(geo) {
  if (!geo || geo.bbox_min_lon == null || geo.bbox_max_lon == null) return 4_000_000
  const span = Math.max(geo.bbox_max_lon - geo.bbox_min_lon, geo.bbox_max_lat - geo.bbox_min_lat)
  return Math.min(Math.max(span * 180_000, 700_000), 8_000_000)
}

// `geoByCode` supplies centroids and bounding boxes for every country the
// basemap can draw (from /api/geo). `blockingByCode` decides which of those get
// a marker and what colour it is — the globe no longer reads sanctions_tier, so
// the researched policy dossiers are deliberately not a prop.
export default function Globe({
  blockingByCode = {},
  geoByCode = {},
  outages = [],
  onCountrySelect,
  onLoadError,
  layer = 'ALL',
}) {
  const containerRef = useRef(null)
  const viewerRef = useRef(null)
  const markerStateRef = useRef({})
  const outageStateRef = useRef({})
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

      viewer.camera.flyTo({
        destination: Cesium.Cartesian3.fromDegrees(HOME_VIEW.lon, HOME_VIEW.lat, HOME_VIEW.height),
        duration: 0,
      })

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

      // Markers are built by their own effect, from props — see below. This
      // effect owns only what the scene needs once: the viewer, the borders,
      // the animation loop and the input handlers.
      const markerState = markerStateRef.current

      const preRenderCallback = () => {
        const now = performance.now()
        const cycle = (now % PULSE_PERIOD_MS) / PULSE_PERIOD_MS

        Object.values(markerState).forEach((m) => {
          const alpha = m.alpha ?? 1

          // Inconclusive markers are dot-only, so guard the ring.
          if (m.ringEntity) {
            m.ringEntity.billboard.scale = 1 + 0.4 * cycle
            m.ringEntity.billboard.color = m.color.withAlpha((1 - cycle) * alpha)
          }

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

          const baseColor = m.color.withAlpha(alpha)
          m.dotEntity.billboard.scale = dotScale * (m.hovered ? 1.15 : 1)
          m.dotEntity.billboard.color = m.hovered
            ? Cesium.Color.clone(baseColor).brighten(0.5, new Cesium.Color())
            : baseColor
        })

        // Outage markers: an expanding, fading crimson ring — a radar "ping"
        // over any country IODA currently reports disrupted. Kept in their own
        // state object (referenced via the ref, so this once-created closure
        // still sees rebuilds) and deliberately not clickable.
        const outageCycle = (now % OUTAGE_PULSE_PERIOD_MS) / OUTAGE_PULSE_PERIOD_MS
        Object.values(outageStateRef.current).forEach((o) => {
          o.ringEntity.billboard.scale = 0.8 + 1.4 * outageCycle
          o.ringEntity.billboard.color = CESIUM_CRIMSON.withAlpha((1 - outageCycle) * 0.9)
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

      // Click → flare, fly to country (framed from its bbox), report selection up
      handler.setInputAction(({ position }) => {
        const picked = viewer.scene.pick(position)
        const code   = picked?.id?.properties?.code?.getValue()
        if (!code || !markerState[code]) return

        markerState[code].flareStart = performance.now()

        const geo = markerState[code].geo
        if (geo && geo.centroid_lon != null && geo.centroid_lat != null) {
          viewer.camera.flyTo({
            destination: Cesium.Cartesian3.fromDegrees(geo.centroid_lon, geo.centroid_lat, altitudeFor(geo)),
            duration: 1.2,
          })
        }

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

  // Markers, rebuilt whenever the data or active layer changes. Driven by
  // `blockingByCode` (which countries have a signal, and what it is) positioned
  // via `geoByCode` centroids — so this scales to every country the moment its
  // measurements land, with no hardcoded roster. Rebuilding on `layer` change
  // is what recolours/reshapes markers for the selected layer.
  useEffect(() => {
    if (!ready) return
    const viewer = viewerRef.current
    if (!viewer || viewer.isDestroyed()) return

    const markerState = markerStateRef.current

    // Mutated in place rather than reassigned: the input handlers in the scene
    // effect close over this same object.
    for (const [code, m] of Object.entries(markerState)) {
      if (m.ringEntity) viewer.entities.remove(m.ringEntity)
      if (m.dotEntity) viewer.entities.remove(m.dotEntity)
      delete markerState[code]
    }

    for (const [code, entry] of Object.entries(blockingByCode)) {
      const status = entry?.[layer] ?? 'NO_DATA'
      // No marker for countries with no signal in the active layer — the
      // outline is enough, and a marker per no-data country would bury the
      // ones that mean something.
      if (status === 'NO_DATA') continue

      const geo = geoByCode[code]
      if (!geo || geo.centroid_lon == null || geo.centroid_lat == null) continue

      const hex = BLOCKING_STATUS_COLOR[status] ?? DIM
      const color = cesiumColorFor(status)
      const inconclusive = isInconclusive(status)
      const position = Cesium.Cartesian3.fromDegrees(geo.centroid_lon, geo.centroid_lat)

      let ringEntity = null
      if (!inconclusive) {
        ringEntity = viewer.entities.add({
          position,
          properties: { code },
          billboard: {
            image: ringCanvas(hex),
            width: 24,
            height: 24,
            color,
            disableDepthTestDistance: Number.POSITIVE_INFINITY,
          },
        })
      }

      const dotEntity = viewer.entities.add({
        position,
        properties: { code },
        billboard: {
          image: dotCanvas(hex),
          width: inconclusive ? 6 : 10,
          height: inconclusive ? 6 : 10,
          color,
          disableDepthTestDistance: Number.POSITIVE_INFINITY,
        },
      })

      markerState[code] = {
        ringEntity,
        dotEntity,
        color,
        alpha: inconclusive ? 0.55 : 1,
        hovered: false,
        flareStart: null,
        geo,
      }
    }
  }, [ready, blockingByCode, geoByCode, layer])

  // Global internet-outage overlay, rebuilt whenever the active-outage set
  // changes. Independent of the blocking markers so it can light up any country
  // in the world; each entry carries its own centroid from /api/geo.
  useEffect(() => {
    if (!ready) return
    const viewer = viewerRef.current
    if (!viewer || viewer.isDestroyed()) return

    const outageState = outageStateRef.current
    for (const [code, o] of Object.entries(outageState)) {
      viewer.entities.remove(o.ringEntity)
      delete outageState[code]
    }

    outages.forEach((o) => {
      if (o.lat == null || o.lon == null) return
      const position = Cesium.Cartesian3.fromDegrees(o.lon, o.lat)
      const ringEntity = viewer.entities.add({
        position,
        properties: { outage: true },
        billboard: {
          image: ringCanvas(CRIMSON),
          width: 28,
          height: 28,
          color: CESIUM_CRIMSON,
          disableDepthTestDistance: Number.POSITIVE_INFINITY,
        },
      })
      outageState[o.code] = { ringEntity }
    })
  }, [ready, outages])

  return <div ref={containerRef} className="w-full h-full" />
}
