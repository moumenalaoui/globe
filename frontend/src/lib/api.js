const BASE = '/api'

export async function getCountries() {
  const r = await fetch(`${BASE}/countries`)
  if (!r.ok) throw new Error('Failed to fetch countries')
  return r.json()
}

export async function getBlocking() {
  const r = await fetch(`${BASE}/blocking`)
  if (!r.ok) throw new Error('Failed to fetch blocking status')
  return r.json()
}

export async function getCountry(code) {
  const r = await fetch(`${BASE}/countries/${code}`)
  if (!r.ok) throw new Error(`Failed to fetch country ${code}`)
  return r.json()
}

export async function evaluate(countryCode, sensitivity, orgType) {
  const r = await fetch(`${BASE}/evaluate`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      country_code: countryCode,
      sensitivity,
      org_type: orgType,
    }),
  })
  if (!r.ok) throw new Error('Evaluate failed')
  return r.json()
}

export async function getSignals(countryCode) {
  const r = await fetch(`${BASE}/signals?country=${countryCode}`)
  if (!r.ok) throw new Error('Failed to fetch signals')
  return r.json()
}

// Country identity + map geometry (centroids) for every drawable country.
// Used to place the global internet-outage overlay, which can light up any
// country, not just the researched five.
export async function getGeo() {
  const r = await fetch(`${BASE}/geo`)
  if (!r.ok) throw new Error('Failed to fetch geo')
  return r.json()
}

// IODA internet-outage events. `active` restricts to outages that ended
// within the backend's recent-activity grace window (the "live" set);
// `country` restricts to one country's history.
export async function getOutages({ country, active } = {}) {
  const params = new URLSearchParams()
  if (country) params.set('country', country)
  if (active) params.set('active', 'true')
  const qs = params.toString()
  const r = await fetch(`${BASE}/outages${qs ? `?${qs}` : ''}`)
  if (!r.ok) throw new Error('Failed to fetch outages')
  return r.json()
}

// Per-content-category censorship for a country (OONI web_connectivity
// aggregated by Citizen Lab category_code), ordered most-censored first.
export async function getCategories(countryCode) {
  const r = await fetch(`${BASE}/categories?country=${countryCode}`)
  if (!r.ok) throw new Error('Failed to fetch categories')
  return r.json()
}

// Messaging-app blocking (whatsapp/telegram/facebook_messenger/signal),
// stored as MESSAGING-category rows in technology_blocks and served by the
// existing /api/blocking endpoint filtered by layer=MESSAGING.
export async function getMessaging(countryCode) {
  const r = await fetch(`${BASE}/blocking?country=${countryCode}&layer=MESSAGING`)
  if (!r.ok) throw new Error('Failed to fetch messaging status')
  return r.json()
}
