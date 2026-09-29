import { useEffect, useRef, useState } from 'react'
import { X } from 'lucide-react'
import { resolveApiUrl } from '../../../api/runtime'
import { getAuthToken } from '../../../api/client'
import { cardPreviewApi } from '../socialCardPreview'
import { quoteSummary, type SocialQuote } from './socialQuote'
import type { LinkPreview } from '../socialLinks'
import styles from './SocialQuotePreview.module.css'

type QuoteLinkPreview = LinkPreview & { cover_data_url?: string | null }
const cache = new Map<string, QuoteLinkPreview>()
let cacheToken: string | null | undefined
export default function SocialQuotePreview({ quote, owner, onCancel, onOpen, own = false }: {
  quote: SocialQuote; owner: string; onCancel?: () => void; onOpen?: () => void; own?: boolean
}) {
  const root = useRef<HTMLDivElement>(null)
  const [preview, setPreview] = useState<QuoteLinkPreview | null>(null)
  const [broken, setBroken] = useState(false)
  const summary = quoteSummary(quote)
  const link = !quote.unavailable && typeof ElonSocialLinks !== 'undefined' ? ElonSocialLinks.links(summary)[0] : null
  useEffect(() => {
    setPreview(null); setBroken(false)
    if (!link || !root.current) return
    const token = getAuthToken()
    if (cacheToken !== token) { cache.clear(); cacheToken = token }
    const key = `${resolveApiUrl('/')}\n${owner}\n${link.url}`, existing = cache.get(key)
    if (existing) { setPreview(existing); return }
    const abort = new AbortController()
    const observer = new IntersectionObserver(entries => {
      if (!entries.some(e => e.isIntersecting)) return
      observer.disconnect()
      void cardPreviewApi('/api/me/link-preview', { method: 'POST', body: JSON.stringify({ url: link.url }), signal: abort.signal }).then(value => {
        const result = value as QuoteLinkPreview
        if (!abort.signal.aborted && getAuthToken() === token && result?.url === link.url) {
          if (cache.size >= 64) cache.delete(cache.keys().next().value!)
          cache.set(key, result); setPreview(result)
        }
      }).catch(() => {})
    })
    observer.observe(root.current)
    return () => { abort.abort(); observer.disconnect() }
  }, [owner, quote.message_id, quote.content, quote.unavailable, link?.url])
  const image = !quote.unavailable && (quote.attachments.find(a => a.kind === 'image' || a.mime_type?.startsWith('image/'))?.url || preview?.cover_data_url || preview?.image)
  const safeImage = image && /^data:image\/(png|jpeg|webp|gif);base64,[a-z0-9+/=]+$/i.test(image)
    ? image : image && (/^\/(?!\/)/.test(image) || /^https?:\/\//i.test(image)) ? resolveApiUrl(image) : null
  const text = link && ElonSocialLinks.compact(summary) ? (preview?.title || link.title || `[链接] ${link.site}`) : summary
  return <div ref={root} className={`${styles.quote} ${onCancel ? styles.draft : ''} ${own ? styles.own : ''}`} aria-label={onCancel ? '待发送引用' : '引用的消息'}>
    <button className={styles.source} type="button" onClick={onOpen} disabled={!onOpen} title="查看引用消息">
      <span className={styles.text}>{quote.sender_name && <span>{quote.sender_name}：</span>}{text}</span>
      {safeImage && !broken && <img className={styles.thumbnail} src={safeImage} referrerPolicy="no-referrer" alt="" onError={() => setBroken(true)} />}
    </button>
    {onCancel && <button type="button" className={styles.close} onClick={onCancel} aria-label="取消引用" title="取消引用"><X size={16} /></button>}
  </div>
}
