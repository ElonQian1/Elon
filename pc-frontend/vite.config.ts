import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

const apiProxyTarget = process.env.VITE_API_PROXY_TARGET
  || process.env.PC_API_PROXY_TARGET
  || 'http://localhost:8080'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  // /pc 是主路由；/pc-next 保留为向后兼容别名
  base: '/pc/',
  build: {
    outDir: 'dist',
    sourcemap: false,
    rollupOptions: {
      output: {
        // 按模块分 chunk，便于长期缓存
        manualChunks: {
          vendor: ['react', 'react-dom', 'react-router-dom'],
          store: ['zustand'],
        },
      },
    },
  },
  server: {
    port: 5173,
    // 开发时将 API 请求代理到 Rust 后端
    proxy: {
      '/api': {
        target: apiProxyTarget,
        changeOrigin: true,
      },
    },
  },
})
