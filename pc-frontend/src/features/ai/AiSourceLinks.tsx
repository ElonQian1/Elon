import { ChevronDown, ChevronUp, ExternalLink, Globe2, PanelTopOpen } from 'lucide-react'
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
              <SourceMark source={source} identity={identity} />
              <span className={styles.copy}>
                <small>{identity.host || '公开网页'}</small>
                <strong>{title}</strong>
              </span>
            </>
          )

          return (
            <article className={styles.card} key={source.url}>
              <a
                className={[styles.main, !internalTabs ? styles.mainOnly : ''].join(' ')}
                href={source.url}
                target="_blank"
                rel="noreferrer"
                title={`使用系统浏览器打开：${title}`}
                aria-label={`使用系统浏览器打开 ${title}`}
              >
                {content}
                {!internalTabs && <ExternalLink size={14} aria-hidden="true" />}
              </a>

              {internalTabs && (
                <button
                  className={styles.internal}
                  type="button"
                  title="在一龙内部标签页打开"
                  aria-label={`在一龙标签页打开 ${title}`}
                  onClick={() => openInternalBrowserLink(source)}
                >
                  <PanelTopOpen size={15} aria-hidden="true" />
                </button>
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
  const unique: AiSource[] = []
  const indexes = new Map<string, number>()
  for (const source of sources ?? []) {
    const key = normalizedUrl(source.url)
    if (!key) continue
    const existingIndex = indexes.get(key)
    if (existingIndex === undefined) {
      indexes.set(key, unique.length)
      unique.push(source)
      continue
    }
    if (!unique[existingIndex].icon_url && source.icon_url) {
      unique[existingIndex] = { ...unique[existingIndex], icon_url: source.icon_url }
    }
  }
  return unique
}

function SourceMark({
  source,
  identity,
}: {
  source: AiSource
  identity: ReturnType<typeof siteIdentity>
}) {
  const iconUrl = safeIconUrl(source.icon_url)
  const [failedUrl, setFailedUrl] = useState('')
  const showIcon = Boolean(iconUrl && failedUrl !== iconUrl)
  return (
    <span className={styles.siteMark} data-tone={identity.tone} aria-hidden="true">
      <span>{identity.initial}</span>
      {showIcon && (
        <img
          className={styles.siteLogo}
          src={iconUrl}
          alt=""
          loading="lazy"
          decoding="async"
          referrerPolicy="no-referrer"
          onError={() => setFailedUrl(iconUrl)}
        />
      )}
    </span>
  )
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

function safeIconUrl(value?: string) {
  if (!value) return ''
  try {
    const parsed = new URL(value)
    if (parsed.protocol !== 'https:' || parsed.username || parsed.password) return ''
    return parsed.toString()
  } catch {
    return ''
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
