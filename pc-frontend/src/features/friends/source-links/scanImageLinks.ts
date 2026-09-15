import { webSourceUrl, type SourceLink } from './sourceLink'
export async function scanImageLinks(blob: Blob): Promise<SourceLink[]> {
  if (!blob.type.startsWith('image/') || blob.size > 12 * 1024 * 1024) return []
  const bitmap = await createImageBitmap(blob)
  let worker: Worker | undefined
  try {
    const scale = Math.min(1, 1800 / Math.max(bitmap.width, bitmap.height))
    const canvas = document.createElement('canvas'); canvas.width = Math.max(1, Math.round(bitmap.width * scale)); canvas.height = Math.max(1, Math.round(bitmap.height * scale))
    const context = canvas.getContext('2d', { willReadFrequently: true }); if (!context) return []
    context.drawImage(bitmap, 0, 0, canvas.width, canvas.height)
    const frame = context.getImageData(0, 0, canvas.width, canvas.height)
    worker = new Worker(new URL('./qrWorker.ts', import.meta.url), { type: 'module' })
    const active = worker
    const values = await new Promise<string[]>((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('二维码识别超时，可在图片预览中重试')), 8000)
      active.onmessage = event => { clearTimeout(timer); resolve(event.data) }
      active.onerror = () => { clearTimeout(timer); reject(new Error('二维码识别失败')) }
      active.postMessage({ pixels: frame.data, width: frame.width, height: frame.height }, [frame.data.buffer])
    })
    return [...new Set(values)].filter(value => !!webSourceUrl(value)).map(url => ({ version: 1, url, method: 'qr' }))
  } finally { bitmap.close(); worker?.terminate() }
}
