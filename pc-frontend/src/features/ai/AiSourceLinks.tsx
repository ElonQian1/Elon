import { ChevronDown, ChevronUp, ExternalLink, Globe2, PanelTopOpen } from 'lucide-react'
import { useState } from 'react'
import { isLocalAiBrowserAvailable } from '../user-browser/localAiBrowserApi'
import { openInternalBrowserLink } from '../user-browser/internalBrowserApi'
import type { AiSource } from './AiChatMessageRow'
import AiSourceMark from './AiSourceMark'
import { aiSiteIdentity, aiSourceDisplayTitle, normalizedAiSourceUrl } from './aiSourcePresentation'
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
            <AiSourceMark key={source.url} source={source} variant="compact" />
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
              const identity = aiSiteIdentity(source.url)
              const title = aiSourceDisplayTitle(source, identity)
              const content = (
                <>
                  <AiSourceMark source={source} />
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
    const key = normalizedAiSourceUrl(source.url)
    if (!key) continue
    const existingIndex = indexes.get(key)
    if (existingIndex === undefined) {
      indexes.set(key, unique.length)
      unique.push(source)
      continue
    }
    const existing = unique[existingIndex]
    const existingTitle = aiSourceDisplayTitle(existing)
    const nextTitle = aiSourceDisplayTitle(source)
    unique[existingIndex] = {
      ...existing,
      title: nextTitle.length > existingTitle.length ? source.title : existing.title,
      icon_url: existing.icon_url || source.icon_url,
      marker_text: existing.marker_text || source.marker_text,
      citation_id: existing.citation_id || source.citation_id,
      group_size: Math.max(existing.group_size || 1, source.group_size || 1),
    }
  }
  return unique
}
