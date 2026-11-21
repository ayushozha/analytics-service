import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

// The Rust server serves the build at /app. In dev, API and login calls are
// proxied to it (PULSE_API, default http://localhost:8090).
const api = process.env.PULSE_API ?? 'http://localhost:8090';

export default defineConfig({
  base: '/app/',
  plugins: [react()],
  server: { proxy: { '/api': api, '/dashboard': api } },
  test: { environment: 'jsdom' },
});
