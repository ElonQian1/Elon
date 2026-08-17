import { ExternalLink } from 'lucide-react'
import { isLocalAiBrowserAvailable } from '../user-browser/localAiBrowserApi'
import { openInternalBrowserLink } from '../user-browser/internalBrowserApi'
import type { AiSource } from './AiChatMessageRow'
import styles from './AiSourceLinks.module.css'

export default function AiSourceLinks({ sources }: { sources?: AiSource[] }) {
  if (!sources?.length) return null
  const internalTabs = isLocalAiBrowserAvailable()
  return (
    <div className={styles.list} aria-label="回答来源">
      {sources.map((source) => internalTabs ? (
        <span className={styles.choice} key={source.url}>
          <button type="button" title="在一龙标签页打开" onClick={() => openInternalBrowserLink(source)}>
            {source.title || source.url}
          </button>
          <a href={source.url} target="_blank" rel="noreferrer" title="使用系统浏览器打开" aria-label={`使用系统浏览器打开 ${source.title || source.url}`}>
            <ExternalLink size={12} aria-hidden="true" />
          </a>
        </span>
      ) : (
        <a className={styles.link} key={source.url} href={source.url} target="_blank" rel="noreferrer">
          {source.title || source.url}
        </a>
      ))}
    </div>
  )
}
