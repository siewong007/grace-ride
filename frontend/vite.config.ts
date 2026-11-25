import fs from 'node:fs';
import path from 'node:path';
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import type { Plugin } from 'vite';

// One id per build: the running app compares it with /version.json to offer "update now",
// and dist/sw.js stamps the same id into its shell CacheStorage key.
const BUILD_ID = (process.env.GITHUB_SHA ?? '').slice(0, 12) || Date.now().toString(36);
const SW_PLACEHOLDER = '__GR_SW_BUILD_ID__';

function versionManifest(): Plugin {
  return {
    name: 'grace-ride-version-json',
    apply: 'build',
    generateBundle() {
      this.emitFile({
        type: 'asset',
        fileName: 'version.json',
        source: JSON.stringify({ build: BUILD_ID }),
      });
    },
    writeBundle(options) {
      const outDir = options.dir ? path.resolve(options.dir) : path.resolve('dist');
      const swPath = path.join(outDir, 'sw.js');
      if (!fs.existsSync(swPath)) return;
      const src = fs.readFileSync(swPath, 'utf-8');
      if (!src.includes(SW_PLACEHOLDER)) {
        throw new Error(`Expected ${SW_PLACEHOLDER} placeholder in ${swPath}`);
      }
      fs.writeFileSync(swPath, src.replaceAll(SW_PLACEHOLDER, BUILD_ID), 'utf-8');
    },
  };
}

export default defineConfig({
  plugins: [react(), versionManifest()],
  define: { __APP_BUILD_ID__: JSON.stringify(BUILD_ID) },
  server: {
    port: 5173,
    proxy: { '/api': 'http://127.0.0.1:8080' },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (
            id.includes('node_modules/react-dom') ||
            id.includes('node_modules/react/') ||
            id.includes('node_modules/react-router')
          ) {
            return 'vendor-react';
          }
          if (
            id.includes('node_modules/i18next') ||
            id.includes('node_modules/react-i18next')
          ) {
            return 'vendor-i18n';
          }
        },
      },
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: './src/test/setup.ts',
    css: false,
    exclude: ['**/node_modules/**', '**/e2e/**', '**/dist/**'],
  },
});
