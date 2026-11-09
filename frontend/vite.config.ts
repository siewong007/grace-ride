import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// The Rust API runs on :8080; the dev server proxies /api so cookies stay same-origin
// (passkeys are bound to the host the member sees: localhost).
export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: { '/api': 'http://127.0.0.1:8080' },
  },
});
