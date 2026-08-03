import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import cesium from 'vite-plugin-cesium'

export default defineConfig({
  // vite-plugin-cesium copies Cesium's static assets (workers, Assets/,
  // Widgets/) into the bundle and sets CESIUM_BASE_URL. Cesium can't run from
  // a bare module import without them — `import 'cesium/Build/Cesium/Widgets/widgets.css'`
  // in Globe.jsx resolves, but the globe renders blank without the workers.
  plugins: [react(), cesium()],
  server: {
    port: 5173,
    proxy: {
      // lib/api.js and the components all fetch a relative `/api`, so the dev
      // server has to forward that to the backend rather than answering it
      // itself — otherwise every request 404s against Vite.
      '/api': {
        target: 'http://localhost:3001',
        changeOrigin: true,
      },
      // `/health` is not under `/api` — it is deliberately kept at the root so
      // an uptime monitor can poll a stable path. It still has to be forwarded
      // here: App.jsx reads the data-freshness figure from it, and without
      // this entry Vite answers with index.html, the JSON parse throws, and
      // the status bar's DATA AGE silently reads "—" in dev only.
      '/health': {
        target: 'http://localhost:3001',
        changeOrigin: true,
      },
    },
  },
})
