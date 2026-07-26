import { resolve } from 'node:path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

const apiProxyTarget = process.env.VITE_API_PROXY_TARGET
  || process.env.PC_API_PROXY_TARGET
  || 'http://localhost:8080'

function manualChunks(id: string) {
  const normalized = id.replace(/\\/g, '/')
  if (!normalized.includes('/node_modules/')) return undefined
  if (
    normalized.includes('/node_modules/react/')
    || normalized.includes('/node_modules/react-dom/')
    || normalized.includes('/node_modules/react-router-dom/')
  ) {
    return 'vendor'
  }
  if (normalized.includes('/node_modules/zustand/')) {
    return 'store'
  }
  return undefined
}

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  // /pc 是主路由；/pc-next 保留为向后兼容别名
  base: '/pc/',
  build: {
    outDir: 'dist',
    sourcemap: false,
    rollupOptions: {
      input: {
        app: resolve(__dirname, 'index.html'),
        taskProgressPreview: resolve(__dirname, 'task-progress-preview.html'),
      },
      output: {
        // 按模块分 chunk，便于长期缓存
        manualChunks,
      },
    },
  },
  server: {
    port: 5173,
    fs: {
      allow: [resolve(__dirname, '..')],
    },
    // 开发时将 API 请求代理到 Rust 后端
    proxy: {
      '/api': {
        target: apiProxyTarget,
        changeOrigin: true,
      },
      '/ws': {
        target: apiProxyTarget,
        changeOrigin: true,
        ws: true,
      },
    },
  },
})
