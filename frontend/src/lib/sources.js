// The open-source feeds the backend ingests (see backend/src/fetchers/*).
// Surfaced in the UI chrome purely as provenance — this list is display-only,
// drives no fetching, and must stay a faithful reflection of what the backend
// actually pulls. The backend owns all ingestion.
//
//   Telemetry / measurement:  OONI, IODA, Tor Metrics, Cloudflare
//   Freedom indices:          V-Dem, RSF
//                             (both arrive via Our World in Data's
//                              republished grapher CSVs — see indices.rs)
export const SOURCES = [
  { id: 'OONI', label: 'OONI' },
  { id: 'IODA', label: 'IODA' },
  { id: 'TOR', label: 'Tor' },
  { id: 'CLOUDFLARE', label: 'Cloudflare' },
  { id: 'ISOC_PULSE', label: 'ISOC Pulse' },
  { id: 'V_DEM', label: 'V-Dem' },
  { id: 'RSF', label: 'RSF' },
]
