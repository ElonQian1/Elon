import { sourceLabel, sourceLink, webSourceUrl } from './sourceLink'
import { openImageScan } from '../../scan/scanEntry'
import styles from './SourceLinkView.module.css'

export function SourceLinkView({ value }: { value: unknown }) {
  const source = sourceLink(value)
  return source ? <a className={styles.link} href={source.url} target="_blank" rel="noopener noreferrer">{sourceLabel(source.url)} · {new URL(source.url).hostname}</a> : null
}
export function TextSourceCard({ text }: { text: string }) {
  if (globalThis.ElonSocialLinks?.links(text).length) return null
  const url = webSourceUrl(text.trim())
  return url ? <SourceLinkView value={{ version: 1, url, method: 'share' }} /> : null
}
export function ManualImageQr({ url }: { url: string }) {
  return <div className={styles.manual}><button type="button" onClick={() => openImageScan(url)}>识别二维码</button></div>
}
