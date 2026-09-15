import { square } from './squareApi'
interface Media { id: string; data_url: string }
async function canvasUpload(source: CanvasImageSource, width: number, height: number): Promise<Media> {
  const c = document.createElement('canvas'), scale = Math.min(1, 1600 / Math.max(width, height))
  c.width = Math.max(1, Math.round(width * scale)); c.height = Math.max(1, Math.round(height * scale))
  const ctx = c.getContext('2d'); if (!ctx) throw Error('无法处理媒体封面')
  ctx.fillStyle = '#fff'; ctx.fillRect(0, 0, c.width, c.height); ctx.drawImage(source, 0, 0, c.width, c.height)
  let data = ''; for (const q of [.85, .7, .55, .4]) { data = c.toDataURL('image/jpeg', q); if (data.length < 699000) break }
  if (data.length >= 699000) throw Error('图片过大，请先裁剪')
  return square('/media', 'POST', { base64: data.split(',')[1] })
}
export async function squareImage(file: File): Promise<Media> {
  if (!/^image\/(png|jpeg|webp)$/.test(file.type) || file.size > 20 * 1024 * 1024) throw Error('请选择小于20MB的PNG、JPEG或WebP图片')
  const url = URL.createObjectURL(file)
  try { const img = new Image(); img.src = url; await img.decode(); return await canvasUpload(img, img.width, img.height) } finally { URL.revokeObjectURL(url) }
}
export async function squareVideo(file: File): Promise<{ id: string; name: string; cover: string }> {
  if (!['video/mp4', 'video/webm'].includes(file.type) || file.size > 32 * 1024 * 1024) throw Error('请选择小于32MB的MP4或WebM视频')
  const url = URL.createObjectURL(file), video = document.createElement('video'); video.muted = true; video.preload = 'auto'; video.playsInline = true
  try {
    await new Promise<void>((resolve, reject) => { const timer = setTimeout(() => reject(Error('视频无法读取，请更换格式')), 15000); video.onloadeddata = () => { clearTimeout(timer); resolve() }; video.onerror = () => { clearTimeout(timer); reject(Error('无法读取视频')) }; video.src = url; video.load() })
    if (!Number.isFinite(video.duration) || video.duration <= 0 || video.duration > 600) throw Error('视频时长需在10分钟以内')
    const cover = await canvasUpload(video, video.videoWidth, video.videoHeight)
    const form = new FormData(); form.append('cover_id', cover.id); form.append('duration', String(video.duration)); form.append('video', file)
    const result = await square<{ id: string }>('/videos', 'POST', form)
    return { id: result.id, name: file.name, cover: cover.data_url }
  } finally { video.pause(); video.removeAttribute('src'); video.load(); URL.revokeObjectURL(url) }
}
