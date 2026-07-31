# Censorship Tracker

An open-source-intelligence (OSINT) tool that visualizes how US and Chinese AI
strategies and infrastructure-level censorship shape digital access across the
Middle East & North Africa — and, from the same open feeds, across the rest of
the world.

A 3D globe renders a composite censorship index, live internet outages, and
confirmed service blocks; a per-country panel drills into the measured evidence
behind each. Every figure comes from open sources: the backend ingests and
normalizes them, the frontend only renders.

## Features

- **Interactive globe (CesiumJS)** — dark neutral ocean and slate land, a
  composite censorship-index choropleth (green → crimson), crimson "thermal
  bloom" markers for confirmed blocks, pulsing blooms for live outages, and
  click-any-country selection.
- **Command + status chrome** — live counters (countries / signals / outages),
  a source bar surfacing every feed, and a `LAST SYNC` / link-state readout.
- **Per-country dossier** — blocking status by technology, messaging-app blocks,
  censored content categories, Tor relay/bridge usage, internet-outage history,
  Freedom House sub-scores, global freedom indices, the open-weight model
  landscape, and bilateral compute deals.

## Architecture

- **`frontend/`** — React 18 + Vite + CesiumJS (globe) + Recharts (charts).
  Styling is plain inline styles driven by design tokens in `src/theme.js` (no
  CSS framework). It talks to the backend over a relative `/api`, proxied to
  `:3001` in dev.
- **`backend/`** — Rust (Axum + Tokio) over an embedded SQLite database
  (`rusqlite`, bundled). On startup it seeds the DB and, in the background,
  runs fetchers that pull the open feeds and store normalized rows; the API
  serves them.

## Data sources

| Feed | Used for |
| --- | --- |
| OONI | Web / messaging / content-category censorship measurements |
| IODA | Internet outages (BGP / active probing / telescope) |
| Tor Metrics | Relay & bridge user counts |
| Cloudflare Radar | Service reachability signals |
| Freedom House (Freedom on the Net) | Access / Content / Rights sub-scores |
| V-Dem (via Our World in Data) | Freedom-of-expression index |
| RSF (via Our World in Data) | Press-freedom index |

## Getting started

Prerequisites: **Rust** (stable, edition 2024) and **Node 18+**.

### 1. Backend — API on `:3001`

```sh
cd backend
cargo run
```

On first run it creates and seeds `mena_ai.db`, then fetches live data in the
background — the API is usable immediately and data fills in as each source
completes.

- Optional: create `backend/.env` with `CLOUDFLARE_API_TOKEN=...` to enable the
  Cloudflare Radar fetcher. The other feeds are public and need no key.
- Override the port with `PORT=...`.

### 2. Frontend — dev server on `:5173`

```sh
cd frontend
npm install
npm run dev
```

Vite proxies `/api` → `http://localhost:3001`, so start the backend first, then
open <http://localhost:5173>.

- Optional: `frontend/.env.local` may set `VITE_CESIUM_ION_TOKEN`, but the globe
  uses no Cesium Ion imagery, so it is not required.
- Production build: `npm run build` (output in `frontend/dist/`).

## API

Base path `/api`, JSON responses.

| Method & path | Returns |
| --- | --- |
| `GET /api/countries` | researched country dossiers |
| `GET /api/countries/:code` | one country dossier |
| `GET /api/geo` | country centroids + bounding boxes |
| `GET /api/blocking` | per-(country, technology) block status |
| `GET /api/categories` | OONI content-category censorship |
| `GET /api/timeline` | daily anomaly timeline |
| `GET /api/tor-metrics` | Tor relay / bridge users |
| `GET /api/outages` | IODA internet outages |
| `GET /api/censorship-index` | composite 0–100 index (drives the globe choropleth) |
| `GET /api/rankings` | whole-world freedom-index ranking |
| `GET /api/country-scores` | Freedom House / V-Dem / RSF sub-scores |
| `GET /api/models` | open-weight model releases |
| `GET /api/deals` | bilateral compute deals |
| `GET /api/signals` | adoption / reachability signals |
| `POST /api/evaluate` | modeled AI-deployment access assessment |
| `GET /health` | liveness check |

## Project structure

```
globe/
├── backend/    Rust + Axum API, SQLite store, data fetchers
├── frontend/   React + Vite + CesiumJS single-page app
└── docs/
```

## Notes

- This is a development setup: CORS is permissive and there is no auth — it is
  not hardened for public deployment.
- All figures reflect what the open feeds report; freshness depends on when the
  backend last completed each fetch (shown as `LAST SYNC` in the status bar).
