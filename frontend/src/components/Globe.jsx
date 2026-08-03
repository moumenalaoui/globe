import { useEffect, useRef, useState } from 'react'
import * as Cesium from 'cesium'
import * as topojson from 'topojson-client'
import 'cesium/Build/Cesium/Widgets/widgets.css'
import { AMBER, BLACK, CRIMSON } from '../theme'
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

// The acute layer's only motion: outage blooms breathe in brightness. Alpha
// only — never geometry — so nothing re-tessellates per frame. Smooth 0→1→0.
const OUTAGE_PULSE_PERIOD_MS = 1600

function pulse01(periodMs) {
  const now = performance.now()
  return 0.5 - 0.5 * Math.cos(((now % periodMs) / periodMs) * Math.PI * 2)
}

// Confirmed-blocked bloom radius scales with the composite index (more censored
// → larger glow) so the acute layer carries hierarchy instead of a uniform
// field. Outage bloom radius steps with IODA severity. Metres, on the surface.
function confirmedRadius(index) {
  const t = index == null ? 0.5 : Math.max(0, Math.min(100, index)) / 100
  return 130_000 + t * 260_000
}

function outageRadius(score) {
  if (score >= 200) return 340_000
  if (score >= 60) return 240_000
  return 160_000
}

// Whole-globe framing, centred on ~20°E/15°N rather than 0/0 so the front
// hemisphere on load holds Europe, Africa, the Middle East and South Asia — the
// densest censorship geography — instead of the mid-Atlantic.
const HOME_VIEW = { lon: 20.0, lat: 15.0, height: 20_000_000 }

// The acute layer is crimson, full stop: confirmed blocks and live outages are
// both "trouble". Reusing the confirmed-blocked status colour keeps a glow on
// the globe meaning the same thing as the crimson status in the sidebar.
const ACUTE_HEX = BLOCKING_STATUS_COLOR.CONFIRMED_BLOCKED ?? CRIMSON

// Likely-blocked countries used to draw nothing at all, so 39 countries the
// sidebar reported as blocked had no mark on the globe — and on the AI_ACCESS
// layer, where only one country is confirmed, the globe looked broken. They now
// bloom amber, reusing the same status colour the sidebar uses, so the
// confirmed/likely distinction survives instead of being flattened or dropped.
const LIKELY_HEX = BLOCKING_STATUS_COLOR.LIKELY_BLOCKED ?? AMBER

// Likely blooms are drawn smaller as well as amber: severity reads through size
// even for a viewer who cannot separate the two hues.
const LIKELY_RADIUS_SCALE = 0.62

// A css rgba() string from a theme hex + alpha, for the radial-gradient canvas
// textures below.
function rgbaFrom(hex, alpha) {
  const c = Cesium.Color.fromCssColorString(hex)
  return `rgba(${Math.round(c.red * 255)}, ${Math.round(c.green * 255)}, ${Math.round(c.blue * 255)}, ${alpha})`
}

// Dark-slate land fill so every country reads as land over black ocean, even
// with no index score. Opaque (the black globe sits beneath it) and a hair
// below the SIDEBAR chrome tone so land stays subordinate to the panels.
const LAND_COLOR = Cesium.Color.fromCssColorString('#111823')

// Choropleth ramp for the composite censorship index (0 = free → 100 = most
// censored): green → amber → crimson. Local constants so this doesn't depend on
// the shared theme import line. The fill is translucent so borders and markers
// still read on top.
const CHORO_LOW = Cesium.Color.fromCssColorString('#6c9a5b')
const CHORO_MID = Cesium.Color.fromCssColorString('#d97706')
const CHORO_HIGH = Cesium.Color.fromCssColorString('#c8102e')
const CHORO_ALPHA = 0.55

function choroplethColor(censorship) {
  const t = Math.max(0, Math.min(100, censorship)) / 100
  const c = t <= 0.5
    ? Cesium.Color.lerp(CHORO_LOW, CHORO_MID, t / 0.5, new Cesium.Color())
    : Cesium.Color.lerp(CHORO_MID, CHORO_HIGH, (t - 0.5) / 0.5, new Cesium.Color())
  return c.withAlpha(CHORO_ALPHA)
}

// Canvases are cached per (kind, hex) instead of created per country. There are
// only a handful of distinct statuses, so this is a bounded set no matter how
// many countries are drawn.
const canvasCache = new Map()

// Soft radial glow (bright core → transparent edge) — the "thermal bloom" that
// replaces the old dot+ring marker. Painted onto a surface ellipse so it reads
// as signal radiating off the map rather than a symbol standing on top of it.
function bloomCanvas(hex) {
  const key = `bloom:${hex}`
  if (!canvasCache.has(key)) {
    const size = 128
    const canvas = document.createElement('canvas')
    canvas.width = size
    canvas.height = size
    const ctx = canvas.getContext('2d')
    const r = size / 2
    const gradient = ctx.createRadialGradient(r, r, 0, r, r, r)
    // Hot near-white core → saturated mid → transparent edge. The bright core
    // keeps the mark legible even over crimson choropleth land, where a plain
    // crimson glow blended into the globe.
    gradient.addColorStop(0, 'rgba(255, 241, 224, 0.95)')
    gradient.addColorStop(0.14, rgbaFrom(hex, 0.92))
    gradient.addColorStop(0.4, rgbaFrom(hex, 0.4))
    gradient.addColorStop(1, rgbaFrom(hex, 0))
    ctx.fillStyle = gradient
    ctx.beginPath()
    ctx.arc(r, r, r, 0, Math.PI * 2)
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
  indexByCode = {},
  showIndex = true,
  onCountrySelect,
  onLoadError,
  layer = 'ALL',
  selectedCode = '',
}) {
  const containerRef = useRef(null)
  const viewerRef = useRef(null)
  // Per-country marker state: a crimson bloom entity for each confirmed-blocked
  // country (the only visible status marks). Picking is handled by the land
  // fill, not markers, so there are no invisible pick billboards anymore.
  const markerStateRef = useRef({})
  const outageStateRef = useRef({})
  // Choropleth: the loaded basemap geojson is stashed here in init so the
  // fill effect (which reacts to index data arriving later) can reuse it, and
  // the current fill Primitive is tracked so it can be swapped/removed.
  const geojsonRef = useRef(null)
  const choroplethRef = useRef(null)
  // Static dark-slate land fill (built once from the basemap geometry); tracked
  // so it can be torn down with the viewer.
  const landRef = useRef(null)
  const [ready, setReady] = useState(false)

  // Keep the latest callbacks in refs so the init effect (which only runs
  // once) always calls the current prop without needing to re-run.
  const onCountrySelectRef = useRef(onCountrySelect)
  const onLoadErrorRef     = useRef(onLoadError)
  useEffect(() => { onCountrySelectRef.current = onCountrySelect }, [onCountrySelect])
  useEffect(() => { onLoadErrorRef.current = onLoadError }, [onLoadError])

  // `prevSelectedRef` lets the framing effect tell a real deselect (return to
  // the whole-globe view) apart from the empty selection on first load (which
  // must stay instant, since init already frames HOME_VIEW).
  const prevSelectedRef = useRef(selectedCode)

  useEffect(() => {
    if (!containerRef.current) return

    let cancelled = false
    let viewer

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
        // Render at the display's native devicePixelRatio instead of Cesium's
        // default 1x CSS-pixel resolution — the globe rendered soft/low-res on
        // HiDPI (Retina) screens, most visibly when zoomed out. MSAA is already
        // 4x by default, so this is purely a pixel-density fix.
        useBrowserRecommendedResolution: false,
      })

      // Bail out if this effect was cleaned up (e.g. React StrictMode
      // double-invoke) while the Viewer was being constructed.
      if (cancelled) {
        viewer.destroy()
        return
      }

      viewer.scene.globe.enableLighting = false
      // Ocean = the globe base colour: a dark neutral charcoal, lifted off pure
      // black so it doesn't read as a dead void, but with no blue tone.
      viewer.scene.globe.baseColor = Cesium.Color.fromCssColorString('#121212')
      // Keep only the sky-atmosphere limb (a thin rim just outside the globe) so
      // the sphere still reads against space — but drop the ground atmosphere,
      // which is what tinted the ocean blue. The rim is dimmed and heavily
      // desaturated toward neutral so it doesn't reintroduce a blue cast.
      // brightnessShift/saturationShift ∈ [-1,1]; atmosphereLightIntensity
      // default is 50.
      viewer.scene.globe.showGroundAtmosphere = false
      viewer.scene.skyAtmosphere.show = true
      viewer.scene.skyAtmosphere.brightnessShift = -0.5
      viewer.scene.skyAtmosphere.saturationShift = -0.7
      viewer.scene.skyAtmosphere.atmosphereLightIntensity = 5
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
      // Stashed for the choropleth fill effect, which needs this geometry but
      // runs separately so it can react to index data that arrives after init.
      geojsonRef.current = geojson

      // Thin outline-only borders for every country.
      const outlineCollection = new Cesium.PolylineCollection()
      viewer.scene.primitives.add(outlineCollection)

      const addOutlineRing = (coords) => {
        const positions = coords.map(([lng, lat]) => Cesium.Cartesian3.fromDegrees(lng, lat, 2000))
        outlineCollection.add({
          positions,
          width: 1.2,
          material: Cesium.Material.fromType('Color', {
            // Cool slate-white at ~2.3:1 on black — legible without the harsh
            // full-white hairline. Was #ffffff @ 0.15 (~1.3:1), sub-pixel.
            color: Cesium.Color.fromCssColorString('#9db4c9').withAlpha(0.3),
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

      // Markers and blooms are built by their own effects, from props — see
      // below. Init owns only what the scene needs once: the viewer, the borders
      // and the input handlers. There is deliberately no per-frame animation
      // loop — the only moving part (outage bloom alpha) animates itself through
      // CallbackProperty, so there's nothing to tear down.
      const handler = new Cesium.ScreenSpaceEventHandler(viewer.scene.canvas)

      // Resolve a country code from a pick: the land-fill polygons carry the code
      // directly as their GeometryInstance id (a string), and the crimson blooms
      // carry it in entity properties. Clicking the ocean resolves to nothing.
      // This is what lets any country — not just signalled ones — be selected
      // from the globe, matching the header dropdown.
      const codeFromPick = (picked) =>
        typeof picked?.id === 'string' ? picked.id : picked?.id?.properties?.code?.getValue()

      handler.setInputAction(({ endPosition }) => {
        const code = codeFromPick(viewer.scene.pick(endPosition))
        viewer.scene.canvas.style.cursor = code ? 'pointer' : 'default'
      }, Cesium.ScreenSpaceEventType.MOUSE_MOVE)

      // Click → report selection up. The camera fly-to lives in the selection-
      // framing effect so click and dropdown share one framing path.
      handler.setInputAction(({ position }) => {
        const code = codeFromPick(viewer.scene.pick(position))
        if (!code) return
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
      if (viewer && !viewer.isDestroyed()) {
        viewer.destroy()
      }
      if (viewerRef.current === viewer) {
        viewerRef.current = null
      }
    }
  }, [])

  // Acute status layer, rebuilt when the data or active layer changes. Only
  // CONFIRMED blocks get a visible mark — a crimson surface bloom; everything
  // else (likely, accessible, inconclusive) is carried by the choropleth, so
  // the globe stops being a field of identical stickers. Picking is handled by
  // the land fill below, so no marker is needed just to make a country clickable.
  useEffect(() => {
    if (!ready) return
    const viewer = viewerRef.current
    if (!viewer || viewer.isDestroyed()) return

    const markerState = markerStateRef.current
    for (const [code, m] of Object.entries(markerState)) {
      if (m.bloomEntity) viewer.entities.remove(m.bloomEntity)
      delete markerState[code]
    }

    for (const [code, entry] of Object.entries(blockingByCode)) {
      const status = entry?.[layer] ?? 'NO_DATA'
      const confirmed = status === 'CONFIRMED_BLOCKED'
      const likely = status === 'LIKELY_BLOCKED'
      if (!confirmed && !likely) continue

      const geo = geoByCode[code]
      if (!geo || geo.centroid_lon == null || geo.centroid_lat == null) continue

      // A steady surface bloom, sized by the composite index for hierarchy.
      // Sits at 3000 m and hugs the surface, so it reads as signal radiating
      // off the country rather than a pin standing on it. Full-alpha material
      // lets the hot core carry the brightness, and it carries the country code
      // so clicking the bloom selects, just like clicking the land.
      const radius =
        confirmedRadius(indexByCode[code]) * (confirmed ? 1 : LIKELY_RADIUS_SCALE)
      const bloomEntity = viewer.entities.add({
        position: Cesium.Cartesian3.fromDegrees(geo.centroid_lon, geo.centroid_lat),
        properties: { code },
        ellipse: {
          semiMajorAxis: radius,
          semiMinorAxis: radius,
          height: 3000,
          material: new Cesium.ImageMaterialProperty({
            image: bloomCanvas(confirmed ? ACUTE_HEX : LIKELY_HEX),
            transparent: true,
            color: Cesium.Color.WHITE,
          }),
        },
      })

      markerState[code] = { bloomEntity }
    }
  }, [ready, blockingByCode, geoByCode, indexByCode, layer])

  // Global internet-outage overlay, rebuilt whenever the active-outage set
  // changes. Independent of the blocking markers so it can light up any country
  // in the world; each entry carries its own centroid from /api/geo.
  useEffect(() => {
    if (!ready) return
    const viewer = viewerRef.current
    if (!viewer || viewer.isDestroyed()) return

    const outageState = outageStateRef.current
    for (const [code, o] of Object.entries(outageState)) {
      viewer.entities.remove(o.bloomEntity)
      delete outageState[code]
    }

    outages.forEach((o) => {
      if (o.lat == null || o.lon == null) return
      // A crimson surface bloom whose brightness breathes — the only motion on
      // the map, marking a LIVE disruption. Only the material alpha animates
      // (via CallbackProperty), never the ellipse size, so the geometry is
      // never re-tessellated. Surface-hugging + depth-tested, so the far side
      // is occluded by the globe like everything else.
      const radius = outageRadius(o.maxScore)
      const bloomEntity = viewer.entities.add({
        position: Cesium.Cartesian3.fromDegrees(o.lon, o.lat),
        properties: { code: o.code },
        ellipse: {
          semiMajorAxis: radius,
          semiMinorAxis: radius,
          height: 3000,
          material: new Cesium.ImageMaterialProperty({
            image: bloomCanvas(CRIMSON),
            transparent: true,
            color: new Cesium.CallbackProperty(
              () => Cesium.Color.WHITE.withAlpha(0.3 + 0.55 * pulse01(OUTAGE_PULSE_PERIOD_MS)),
              false,
            ),
          }),
        },
      })
      outageState[o.code] = { bloomEntity }
    })
  }, [ready, outages])

  // Dark-slate land underlay: one flat Primitive filling every country polygon
  // so land reads as land over the dark ocean, even where we have no index
  // score. Built at height 250 (beneath the choropleth at 600 and borders at
  // 2000, which read on top). It doubles as the click target — each polygon
  // carries its country code as a pick id — so it rebuilds when geoByCode
  // arrives. `asynchronous` so the whole-world tessellation never blocks first
  // paint (a brief land pop-in on load is the tradeoff).
  useEffect(() => {
    if (!ready) return
    const viewer = viewerRef.current
    if (!viewer || viewer.isDestroyed()) return
    const geojson = geojsonRef.current
    if (!geojson) return

    // basemap features are keyed by ISO numeric; map to our alpha-2 codes so
    // each land polygon can carry its country code as a pick id — clicking the
    // land is what selects a country now (no invisible marker billboards).
    const numericToCode = new Map()
    for (const [code, geo] of Object.entries(geoByCode)) {
      if (geo && geo.iso_numeric != null) numericToCode.set(parseInt(geo.iso_numeric, 10), code)
    }

    const instances = []
    const addRing = (ring, code) => {
      if (!ring || ring.length < 3) return
      const flat = []
      for (const [lng, lat] of ring) flat.push(lng, lat)
      instances.push(new Cesium.GeometryInstance({
        id: code, // string country code → scene.pick returns it directly
        geometry: new Cesium.PolygonGeometry({
          polygonHierarchy: new Cesium.PolygonHierarchy(Cesium.Cartesian3.fromDegreesArray(flat)),
          height: 250, // below choropleth (600) and borders (2000)
          vertexFormat: Cesium.PerInstanceColorAppearance.VERTEX_FORMAT,
        }),
        attributes: { color: Cesium.ColorGeometryInstanceAttribute.fromColor(LAND_COLOR) },
      }))
    }

    for (const feature of geojson.features) {
      const code = numericToCode.get(parseInt(feature.id, 10))
      const g = feature.geometry
      if (g.type === 'Polygon') addRing(g.coordinates[0], code)
      else if (g.type === 'MultiPolygon') g.coordinates.forEach((poly) => addRing(poly[0], code))
    }

    if (instances.length === 0) return
    const primitive = new Cesium.Primitive({
      geometryInstances: instances,
      appearance: new Cesium.PerInstanceColorAppearance({ flat: true, translucent: false }),
      asynchronous: true,
    })
    viewer.scene.primitives.add(primitive)
    landRef.current = primitive

    return () => {
      if (landRef.current && !viewer.isDestroyed()) {
        viewer.scene.primitives.remove(landRef.current)
      }
      landRef.current = null
    }
  }, [ready, geoByCode])

  // Composite-index choropleth: fill every country whose score we have, from
  // green (free) to crimson (most censored). Built as a single translucent
  // Primitive (one draw, like the border collection) rather than hundreds of
  // entities, and kept below the borders/markers by a small height offset.
  // Rebuilt when the index data, geometry, or toggle changes.
  useEffect(() => {
    if (!ready) return
    const viewer = viewerRef.current
    if (!viewer || viewer.isDestroyed()) return
    const geojson = geojsonRef.current
    if (!geojson) return

    if (choroplethRef.current) {
      viewer.scene.primitives.remove(choroplethRef.current)
      choroplethRef.current = null
    }
    if (!showIndex) return

    // basemap features are keyed by ISO numeric; map those to our alpha-2 codes.
    const numericToCode = new Map()
    for (const [code, geo] of Object.entries(geoByCode)) {
      if (geo && geo.iso_numeric != null) numericToCode.set(parseInt(geo.iso_numeric, 10), code)
    }

    const instances = []
    const addRing = (ring, color) => {
      if (!ring || ring.length < 3) return
      const flat = []
      for (const [lng, lat] of ring) flat.push(lng, lat)
      instances.push(new Cesium.GeometryInstance({
        geometry: new Cesium.PolygonGeometry({
          polygonHierarchy: new Cesium.PolygonHierarchy(Cesium.Cartesian3.fromDegreesArray(flat)),
          height: 600, // above the black globe (avoids z-fight), below borders (2000)
          vertexFormat: Cesium.PerInstanceColorAppearance.VERTEX_FORMAT,
        }),
        attributes: { color: Cesium.ColorGeometryInstanceAttribute.fromColor(color) },
      }))
    }

    for (const feature of geojson.features) {
      const numeric = parseInt(feature.id, 10)
      if (Number.isNaN(numeric)) continue
      const code = numericToCode.get(numeric)
      if (!code) continue
      const score = indexByCode[code]
      if (score == null) continue
      const color = choroplethColor(score)
      const g = feature.geometry
      if (g.type === 'Polygon') addRing(g.coordinates[0], color)
      else if (g.type === 'MultiPolygon') g.coordinates.forEach((poly) => addRing(poly[0], color))
    }

    if (instances.length === 0) return
    const primitive = new Cesium.Primitive({
      geometryInstances: instances,
      appearance: new Cesium.PerInstanceColorAppearance({ flat: true, translucent: true }),
      asynchronous: false,
      // Non-pickable so clicks fall through to the land polygons beneath, which
      // carry the country code — otherwise scored countries couldn't be clicked.
      allowPicking: false,
    })
    viewer.scene.primitives.add(primitive)
    choroplethRef.current = primitive
  }, [ready, indexByCode, geoByCode, showIndex])

  // Reactive camera framing: the globe follows the app-wide selection, whatever
  // set it — a marker click here or the country dropdown in the header. A
  // selected country is framed from its bbox-derived altitude; clearing the
  // selection flies back to the whole-globe HOME_VIEW. Keeping framing here
  // rather than in the click handler means both selection paths share one code
  // path (no double-fly) and a dropdown pick moves the camera too.
  useEffect(() => {
    if (!ready) return
    const viewer = viewerRef.current
    if (!viewer || viewer.isDestroyed()) return

    const prev = prevSelectedRef.current
    prevSelectedRef.current = selectedCode

    if (selectedCode) {
      const geo = geoByCode[selectedCode]
      if (!geo || geo.centroid_lon == null || geo.centroid_lat == null) return
      viewer.camera.flyTo({
        destination: Cesium.Cartesian3.fromDegrees(geo.centroid_lon, geo.centroid_lat, altitudeFor(geo)),
        duration: 1.2,
      })
    } else if (prev) {
      // A real deselect (not the empty selection on first load) → return to the
      // whole-globe framing. init already placed the camera at HOME_VIEW at
      // duration 0, so skipping the `!prev` case keeps startup instant.
      viewer.camera.flyTo({
        destination: Cesium.Cartesian3.fromDegrees(HOME_VIEW.lon, HOME_VIEW.lat, HOME_VIEW.height),
        duration: 1.5,
      })
    }
  }, [ready, selectedCode, geoByCode])

  return <div ref={containerRef} className="w-full h-full" />
}
