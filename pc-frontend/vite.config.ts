import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  // 所有资产路径以 /pc-next/ 开头，与 Rust 路由的 nest_service("/pc-next") 对应
  base: '/pc-next/',
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
        target: 'http://localhost:8080',
        changeOrigin: true,
      },
    },
  },
})
