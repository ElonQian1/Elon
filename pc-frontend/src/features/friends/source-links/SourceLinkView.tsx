import { useState } from 'react'
import { sourceLabel, sourceLink, webSourceUrl } from './sourceLink'
import { scanImageLinks } from './scanImageLinks'
import type { SourceLink } from './sourceLink'
import styles from './SourceLinkView.module.css'

export function SourceLinkView({ value }: { value: unknown }) {
  const source = sourceLink(value)
  return source ? <a className={styles.link} href={source.url} target="_blank" rel="noopener noreferrer">{sourceLabel(source.url)} · {new URL(source.url).hostname}</a> : null
}
export function TextSourceCard({ text }: { text: string }) {
  const url = webSourceUrl(text.trim())
  return url ? <SourceLinkView value={{ version: 1, url, method: 'share' }} /> : null
}
export function ManualImageQr({ url }: { url: string }) {
  const [links, setLinks] = useState<SourceLink[]>([]), [notice, setNotice] = useState(''), [busy, setBusy] = useState(false)
  async function scan() {
    if (busy) return
    setBusy(true); setNotice('正在本地识别…')
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(15000) })
      if (!response.ok) throw new Error('图片不可用，请下载原图后重新选择')
      const found = await scanImageLinks(await response.blob()); setLinks(found)
      setNotice(found.length ? '选择要打开的链接' : '未识别到网页二维码，可尝试原图或复制链接')
    } catch (error) { setNotice((error as Error).message) } finally { setBusy(false) }
  }
  return <div className={styles.manual}><button type="button" disabled={busy} onClick={() => void scan()}>识别二维码</button>
    <span role="status">{notice}</span>{links.map(link => <SourceLinkView key={link.url} value={link} />)}</div>
}
