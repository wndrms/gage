import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { VitePWA } from 'vite-plugin-pwa';
import path from 'node:path';

const apiProxyTarget =
  process.env.API_TARGET ||
  process.env.VITE_API_PROXY_TARGET ||
  'http://localhost:8080';

console.log('[vite] proxy target:', apiProxyTarget);

export default defineConfig({
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src')
    }
  },
  plugins: [
    react(),
    VitePWA({
      registerType: 'autoUpdate',
      includeAssets: ['favicon.svg'],
      manifest: {
        name: '개인 가계부',
        short_name: '가계부',
        description: '개인 자산과 소비를 관리하는 가계부 앱',
        lang: 'ko-KR',
        theme_color: '#0f766e',
        background_color: '#f0fdfa',
        display: 'standalone',
        start_url: '/',
        icons: [
          {
            src: '/favicon.svg',
            sizes: 'any',
            type: 'image/svg+xml'
          }
        ]
      }
    })
  ],
  server: {
    host: '0.0.0.0',
    port: 5173,
    proxy: {
      '/api': {
        target: apiProxyTarget,
        changeOrigin: true
      }
    }
  }
});
