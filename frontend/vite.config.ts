import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

// Dev-only proxy target. The native backend defaults to :8000 via config.yaml;
// override when running it elsewhere: MOCKTAIL_API_URL=http://localhost:8787.
const backend = process.env.MOCKTAIL_API_URL ?? 'http://localhost:8000';

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],
  server: {
    // Keep the same-origin API contract as the embedded build. changeOrigin
    // stays false so the backend sees the apex Host it routes on.
    proxy: {
      '/__admin': { target: backend, changeOrigin: false },
      '/auth': { target: backend, changeOrigin: false }
    }
  }
});
