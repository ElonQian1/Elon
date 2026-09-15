import { api } from '../../api/client'
export const ARTICLE_PREFIX = '【一龙文章】\n'
export type Block = { type: 'paragraph' | 'heading' | 'quote'; text: string } | { type: 'image'; media_id: string; caption: string }
export interface Document { title: string; summary: string; cover: string | null; blocks: Block[] }
export interface Card { id: string; revision: number; title: string; summary: string; author_name: string; author_id: string; status: string; updated_at: string; cover_data_url?: string }
export interface Article extends Card { document: Document; media: Record<string, string> }
export interface Page { items: Card[]; next_offset: number | null }
export interface Group { id: string; name: string }
export interface ArticleReference { article_id: string; revision: number; title: string; summary: string }
export function articleReference(content: string): ArticleReference | null {
  if (!content.startsWith(ARTICLE_PREFIX)) return null
  try {
    const r = JSON.parse(content.slice(ARTICLE_PREFIX.length))
    return r.schema === 1 && /^article_[\w-]+$/.test(r.article_id) && Number.isSafeInteger(r.revision) && r.revision > 0 && typeof r.title === 'string' && typeof r.summary === 'string' ? r : null
  } catch { return null }
}
const path = (id: string) => `/api/me/articles/${encodeURIComponent(id)}`
export const articles = {
  list: (group?: string, offset = 0) => api.get<Page>(`/api/me/articles?offset=${offset}${group ? `&group_id=${encodeURIComponent(group)}` : ''}`),
  draft: (id: string) => api.get<Article>(`${path(id)}/draft`),
  read: (id: string, revision: number, compact = false) => api.get<Article>(`${path(id)}/revisions/${revision}?compact=${compact}`),
  create: () => api.post<Article>('/api/me/articles', { title: '', summary: '', cover: null, blocks: [] }),
  save: (a: Article) => api.put<Article>(`${path(a.id)}/draft`, { version: a.revision, document: a.document }),
  publish: (a: Article, group_ids: string[]) => api.post<{ messages: unknown[] }>(`${path(a.id)}/publish`, { version: a.revision, group_ids }),
  withdraw: (a: Article) => api.post(`${path(a.id)}/withdraw`, { version: a.revision }),
}
export function errorMessage(error: unknown): string {
  return error && typeof error === 'object' && 'message' in error ? String(error.message) : '操作失败，请重试'
}
export async function uploadImage(file: File): Promise<{ id: string; data_url: string }> {
  if (!/^image\/(png|jpeg|webp)$/.test(file.type) || file.size > 20 * 1024 * 1024) throw new Error('请选择小于20MB的PNG、JPEG或WebP图片')
  const url = URL.createObjectURL(file)
  try {
    const img = new Image(); img.src = url; await img.decode()
    const scale = Math.min(1, 1600 / Math.max(img.width, img.height))
    const canvas = document.createElement('canvas'); canvas.width = Math.round(img.width * scale); canvas.height = Math.round(img.height * scale)
    const ctx = canvas.getContext('2d'); if (!ctx) throw new Error('无法处理图片')
    ctx.fillStyle = '#fff'; ctx.fillRect(0, 0, canvas.width, canvas.height); ctx.drawImage(img, 0, 0, canvas.width, canvas.height)
    let data = ''
    for (const quality of [.85, .7, .55, .4]) { data = canvas.toDataURL('image/jpeg', quality); if (data.length < 699000) break }
    if (data.length >= 699000) throw new Error('图片仍然过大，请裁剪后重试')
    return await api.post('/api/me/articles/media', { base64: data.split(',')[1] })
  } finally { URL.revokeObjectURL(url) }
}
