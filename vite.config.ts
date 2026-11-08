import path from 'node:path'
import { defineConfig } from 'vite'
import react, { reactCompilerPreset } from '@vitejs/plugin-react'
import babel from '@rolldown/plugin-babel'
import tailwindcss from '@tailwindcss/vite'

// Set by the Tauri CLI during `tauri android dev` / `tauri ios dev` so the
// physical device or emulator can reach the dev server over the network.
const host = process.env.TAURI_DEV_HOST

// https://vite.dev/config/
export default defineConfig({
  server: {
    port: 3000,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: 'ws',
          host,
          port: 3001,
        }
      : undefined,
    watch: {
      // Don't trigger reloads for Rust / generated Android project changes.
      ignored: ['**/src-tauri/**'],
    },
  },
  build: {
    outDir: 'build',
  },
  plugins: [
    react(),
    tailwindcss(),
    babel({ presets: [reactCompilerPreset()] })
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
})
