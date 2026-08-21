import { ChevronDown, ChevronUp, ExternalLink, Globe2 } from 'lucide-react'
import { useState } from 'react'
import { isLocalAiBrowserAvailable } from '../user-browser/localAiBrowserApi'
import { openInternalBrowserLink } from '../user-browser/internalBrowserApi'
import type { AiSource } from './AiChatMessageRow'
import styles from './AiSourceLinks.module.css'

const MAX_VISIBLE_SOURCES = 3

export default function AiSourceLinks({ sources }: { sources?: AiSource[] }) {
  const [expanded, setExpanded] = useState(false)
  const uniqueSources = uniqueSourcesFor(sources)
  if (!uniqueSources.length) return null

  const internalTabs = isLocalAiBrowserAvailable()
  const visibleSources = expanded ? uniqueSources : uniqueSources.slice(0, MAX_VISIBLE_SOURCES)

  return (
    <section className={styles.panel} aria-label={`回答来源，共 ${uniqueSources.length} 个`}>
      <header className={styles.heading}>
        <span className={styles.headingIcon}><Globe2 size={15} aria-hidden="true" /></span>
        <strong>来源</strong>
        <small>{uniqueSources.length} 个结果</small>
      </header>

      <div className={styles.cards}>
        {visibleSources.map((source) => {
          const identity = siteIdentity(source.url)
          const title = source.title?.trim() || identity.host || source.url
          const content = (
            <>
              <span className={styles.siteMark} data-tone={identity.tone} aria-hidden="true">
                {identity.initial}
              </span>
              <span className={styles.copy}>
                <small>{identity.host || '公开网页'}</small>
                <strong>{title}</strong>
              </span>
            </>
          )

          return (
            <article className={styles.card} key={source.url}>
              {internalTabs ? (
                <button
                  className={styles.main}
                  type="button"
                  title={`在一龙标签页打开：${title}`}
                  aria-label={`在一龙标签页打开 ${title}`}
                  onClick={() => openInternalBrowserLink(source)}
                >
                  {content}
                </button>
              ) : (
                <a
                  className={[styles.main, styles.mainExternal].join(' ')}
                  href={source.url}
                  target="_blank"
                  rel="noreferrer"
                  title={`打开来源：${title}`}
                >
                  {content}
                  <ExternalLink size={14} aria-hidden="true" />
                </a>
              )}

              {internalTabs && (
                <a
                  className={styles.external}
                  href={source.url}
                  target="_blank"
                  rel="noreferrer"
                  title="使用系统浏览器打开"
                  aria-label={`使用系统浏览器打开 ${title}`}
                >
                  <ExternalLink size={15} aria-hidden="true" />
                </a>
              )}
            </article>
          )
        })}
      </div>

      {uniqueSources.length > MAX_VISIBLE_SOURCES && (
        <button className={styles.more} type="button" onClick={() => setExpanded((value) => !value)}>
          {expanded ? '收起来源' : `全部显示（${uniqueSources.length}）`}
          {expanded ? <ChevronUp size={15} aria-hidden="true" /> : <ChevronDown size={15} aria-hidden="true" />}
        </button>
      )}
    </section>
  )
}

function uniqueSourcesFor(sources?: AiSource[]) {
  const seen = new Set<string>()
  return (sources ?? []).filter((source) => {
    const key = normalizedUrl(source.url)
    if (!key || seen.has(key)) return false
    seen.add(key)
    return true
  })
}

function normalizedUrl(url: string) {
  try {
    const parsed = new URL(url)
    parsed.hash = ''
    return parsed.toString()
  } catch {
    return url.trim()
  }
}

function siteIdentity(url: string) {
  try {
    const host = new URL(url).hostname.toLowerCase().replace(/^www\./, '')
    const segments = host.split('.').filter(Boolean)
    const secondLevel = segments[segments.length - 2] || ''
    const suffixIndex = ['co', 'com', 'net', 'org'].includes(secondLevel) ? 3 : 2
    const brand = segments[segments.length - suffixIndex] || segments[0] || 'web'
    return {
      host,
      initial: brand.slice(0, 1).toUpperCase(),
      tone: String(stableTone(host)),
    }
  } catch {
    return { host: '', initial: 'W', tone: '0' }
  }
}

function stableTone(value: string) {
  let total = 0
  for (let index = 0; index < value.length; index += 1) total += value.charCodeAt(index)
  return total % 6
}
