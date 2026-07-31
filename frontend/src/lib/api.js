const BASE = '/api'

export async function getCountries() {
  const r = await fetch(`${BASE}/countries`)
  if (!r.ok) throw new Error('Failed to fetch countries')
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
