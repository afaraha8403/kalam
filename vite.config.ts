import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import { sveltePreprocess } from 'svelte-preprocess'
import path from 'path'
import { fileURLToPath } from 'url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

export default defineConfig(async () => ({
  resolve: {
    alias: {
      $lib: path.resolve(__dirname, 'src/lib'),
    },
  },
  // Charts use `apexcharts` directly (DashboardApex). Old dev caches may still list removed
  // `svelte-apexcharts`, whose published `module` path often ENOENTs. `exclude` is part of
  // `optimizeDeps` config hash, so this invalidates stale `node_modules/.vite` once.
  optimizeDeps: {
    exclude: ['svelte-apexcharts'],
  },
  plugins: [
    svelte({
      preprocess: sveltePreprocess(),
    })
  ],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
  },
}))
