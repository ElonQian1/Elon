import { ChevronDown, ChevronUp, ExternalLink, Globe2, PanelTopOpen } from 'lucide-react'
import { useState } from 'react'
import { isLocalAiBrowserAvailable } from '../user-browser/localAiBrowserApi'
import { openInternalBrowserLink } from '../user-browser/internalBrowserApi'
import type { AiSource } from './AiChatMessageRow'
import styles from './AiSourceLinks.module.css'

const MAX_VISIBLE_SOURCES = 3

export default function AiSourceLinks({ sources }: { sources?: AiSource[] }) {
  const [expanded, setExpanded] = useState(false)
  const [showAll, setShowAll] = useState(false)
  const uniqueSources = uniqueSourcesFor(sources)
  if (!uniqueSources.length) return null

  const internalTabs = isLocalAiBrowserAvailable()
  const visibleSources = showAll ? uniqueSources : uniqueSources.slice(0, MAX_VISIBLE_SOURCES)

  return (
    <section className={styles.sourceSection} aria-label={`回答来源，共 ${uniqueSources.length} 个`}>
      <button
        className={styles.summary}
        type="button"
        aria-expanded={expanded}
        onClick={() => setExpanded((value) => !value)}
      >
        <span className={styles.logoStack} aria-hidden="true">
          {uniqueSources.slice(0, 3).map((source) => (
            <SourceMark key={source.url} source={source} identity={siteIdentity(source.url)} compact />
          ))}
        </span>
        <strong>来源</strong>
        <small>{uniqueSources.length}</small>
        {expanded ? <ChevronUp size={14} aria-hidden="true" /> : <ChevronDown size={14} aria-hidden="true" />}
      </button>

      {expanded && (
        <div className={styles.panel}>
          <header className={styles.heading}>
            <span className={styles.headingIcon}><Globe2 size={15} aria-hidden="true" /></span>
            <strong>参考来源</strong>
            <small>{uniqueSources.length} 个结果</small>
          </header>

          <div className={styles.cards}>
            {visibleSources.map((source) => {
              const identity = siteIdentity(source.url)
              const title = sourceDisplayTitle(source, identity)
              const content = (
                <>
                  <SourceMark source={source} identity={identity} />
                  <span className={styles.copy}>
                    <strong>{title}</strong>
                    <small>{identity.host || '公开网页'}</small>
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
            <button className={styles.more} type="button" onClick={() => setShowAll((value) => !value)}>
              {showAll ? '收起部分来源' : `全部显示（${uniqueSources.length}）`}
              {showAll ? <ChevronUp size={15} aria-hidden="true" /> : <ChevronDown size={15} aria-hidden="true" />}
            </button>
          )}
        </div>
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
  compact = false,
}: {
  source: AiSource
  identity: ReturnType<typeof siteIdentity>
  compact?: boolean
}) {
  const [failedUrls, setFailedUrls] = useState<string[]>([])
  const iconUrl = sourceIconCandidates(source, identity.host)
    .find((candidate) => !failedUrls.includes(candidate)) ?? ''
  return (
    <span
      className={[styles.siteMark, compact ? styles.siteMarkCompact : ''].join(' ')}
      data-tone={identity.tone}
      aria-hidden="true"
    >
      <span>{identity.initial}</span>
      {iconUrl && (
        <img
          className={styles.siteLogo}
          src={iconUrl}
          alt=""
          loading="lazy"
          decoding="async"
          referrerPolicy="no-referrer"
          onError={() => setFailedUrls((values) => values.includes(iconUrl) ? values : [...values, iconUrl])}
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

function sourceIconCandidates(source: AiSource, host: string) {
  const candidates: string[] = []
  const official = safeIconUrl(source.icon_url)
  if (official && !isIncompleteGoogleFavicon(official)) candidates.push(official)
  if (host) candidates.push(googleFaviconUrl(host))
  try {
    const origin = new URL(source.url).origin
    if (origin.startsWith('https://')) candidates.push(`${origin}/favicon.ico`)
  } catch {
    // Invalid source URLs are already filtered before this presentation boundary.
  }
  return [...new Set(candidates)]
}

function isIncompleteGoogleFavicon(value: string) {
  try {
    const parsed = new URL(value)
    return parsed.hostname === 'www.google.com'
      && parsed.pathname === '/s2/favicons'
      && !parsed.searchParams.has('domain')
      && !parsed.searchParams.has('domain_url')
  } catch {
    return false
  }
}

function googleFaviconUrl(host: string) {
  const icon = new URL('https://www.google.com/s2/favicons')
  icon.searchParams.set('domain', host)
  icon.searchParams.set('sz', '64')
  return icon.toString()
}

function sourceDisplayTitle(source: AiSource, identity: ReturnType<typeof siteIdentity>) {
  const raw = source.title?.replace(/\s+/g, ' ').trim() ?? ''
  const withoutLeadingUrl = raw.replace(/^https?:\/\/\S+\s*/i, '').trim()
  const readable = withoutLeadingUrl.replace(/\s*\+(\d+)$/, ' +$1')
  return readable && !/^https?:\/\//i.test(readable)
    ? readable
    : identity.brand || identity.host || '公开网页'
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
      brand: brandLabel(brand, host),
      tone: String(stableTone(host)),
    }
  } catch {
    return { host: '', initial: 'W', brand: '公开网页', tone: '0' }
  }
}

function brandLabel(brand: string, host: string) {
  const knownBrands: Record<string, string> = {
    barrons: "Barron's",
    marketwatch: 'MarketWatch',
    reuters: 'Reuters',
    youtube: 'YouTube',
  }
  if (host.endsWith('sina.com.cn')) return '新浪财经'
  return knownBrands[brand] ?? `${brand.slice(0, 1).toUpperCase()}${brand.slice(1)}`
}

function stableTone(value: string) {
  let total = 0
  for (let index = 0; index < value.length; index += 1) total += value.charCodeAt(index)
  return total % 6
}
