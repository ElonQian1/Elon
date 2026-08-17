import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { ArrowLeft, ArrowRight, ExternalLink, House, RefreshCw, X } from 'lucide-react'
import {
  boundsFor,
  controlInternalBrowserTab,
  controlOfficialAiTab,
  getInternalBrowserTabState,
  hideLocalAiWebSessionEmbedded,
  OPEN_INTERNAL_BROWSER_LINK_EVENT,
  OPEN_OFFICIAL_AI_TAB_EVENT,
  openInternalBrowserTab,
  presentLocalAiWebSessionEmbedded,
  resizeInternalBrowserTab,
  type InternalBrowserLinkRequest,
  type InternalBrowserTabState,
  type OfficialAiTabRequest,
} from './internalBrowserApi'
import styles from './AiBrowserExperience.module.css'

type Surface = 'chat' | 'official' | 'source'

export default function AiBrowserExperience() {
  const [surface, setSurface] = useState<Surface>('chat')
  const [official, setOfficial] = useState<OfficialAiTabRequest | null>(null)
  const [source, setSource] = useState<InternalBrowserLinkRequest | null>(null)
  const [sourceState, setSourceState] = useState<InternalBrowserTabState | null>(null)
  const [status, setStatus] = useState('')
  const [error, setError] = useState('')
  const hostRef = useRef<HTMLDivElement>(null)
  const generationRef = useRef(0)

  useEffect(() => {
    const openOfficial = (event: Event) => {
      const detail = (event as CustomEvent<OfficialAiTabRequest>).detail
      if (!detail?.providerId || !detail.ownerKey) return
      setOfficial(detail)
      setSurface('official')
    }
    const openSource = (event: Event) => {
      const detail = (event as CustomEvent<InternalBrowserLinkRequest>).detail
      if (!detail?.url) return
      setSource(detail)
      setSurface('source')
    }
    window.addEventListener(OPEN_OFFICIAL_AI_TAB_EVENT, openOfficial)
    window.addEventListener(OPEN_INTERNAL_BROWSER_LINK_EVENT, openSource)
    return () => {
      window.removeEventListener(OPEN_OFFICIAL_AI_TAB_EVENT, openOfficial)
      window.removeEventListener(OPEN_INTERNAL_BROWSER_LINK_EVENT, openSource)
    }
  }, [])

  useLayoutEffect(() => {
    const generation = ++generationRef.current
    if (surface === 'chat') {
      void hideSurfaces(official)
      return
    }
    let frame = 0
    let observer: ResizeObserver | null = null
    const synchronize = async (open = false) => {
      const host = hostRef.current
      if (!host || generation !== generationRef.current) return
      setError('')
      try {
        const bounds = boundsFor(host)
        if (surface === 'official' && official) {
          await controlInternalBrowserTab('hide').catch(() => null)
          await presentLocalAiWebSessionEmbedded(official, bounds)
          setStatus(`${official.providerName} 官方原生页面`)
        } else if (surface === 'source' && source) {
          if (official) await hideLocalAiWebSessionEmbedded(official).catch(() => null)
          const next = open || sourceState?.currentUrl !== new URL(source.url).toString()
            ? await openInternalBrowserTab(source, bounds)
            : await resizeInternalBrowserTab(bounds).then(() => getInternalBrowserTabState())
          if (generation === generationRef.current) setSourceState(next)
          setStatus('来源网页使用隔离临时会话，不共享 AI 官网登录状态。')
        }
      } catch (cause) {
        if (generation === generationRef.current) setError(messageFor(cause))
      }
    }
    frame = window.requestAnimationFrame(() => {
      void synchronize(sourceState == null)
      const host = hostRef.current
      if (host) {
        observer = new ResizeObserver(() => {
          window.cancelAnimationFrame(frame)
          frame = window.requestAnimationFrame(() => { void synchronize(false) })
        })
        observer.observe(host)
      }
    })
    return () => {
      window.cancelAnimationFrame(frame)
      observer?.disconnect()
    }
  }, [official, source, sourceState?.currentUrl, surface])

  useEffect(() => () => { void hideSurfaces(official) }, [official])

  if (surface === 'chat') return null

  async function control(action: 'back' | 'forward' | 'reload' | 'home' | 'external') {
    setError('')
    try {
      if (surface === 'official' && official) {
        if (action === 'forward') return
        const officialAction = action === 'home' ? 'home' : action
        await controlOfficialAiTab(official, officialAction)
      } else if (surface === 'source') {
        if (action === 'home') return
        setSourceState(await controlInternalBrowserTab(action))
      }
    } catch (cause) {
      setError(messageFor(cause))
    }
  }

  function closeActive() {
    if (surface === 'source') {
      void controlInternalBrowserTab('close').catch(() => null)
      setSource(null)
      setSourceState(null)
      setSurface(official ? 'official' : 'chat')
      return
    }
    if (official) void hideLocalAiWebSessionEmbedded(official).catch(() => null)
    setOfficial(null)
    setSurface(source ? 'source' : 'chat')
  }

  return (
    <section className={styles.surface} aria-label="一龙内部网页标签">
      <div className={styles.bar}>
        <div className={styles.tabs} role="tablist" aria-label="AI 与网页标签">
          <button className={styles.tab} type="button" role="tab" onClick={() => setSurface('chat')}>聊天</button>
          {official && <button className={styles.tab} data-active={surface === 'official'} type="button" role="tab" onClick={() => setSurface('official')} title={`${official.providerName} 官方页`}>
            {official.providerName} 官方页
          </button>}
          {source && <button className={styles.tab} data-active={surface === 'source'} type="button" role="tab" onClick={() => setSurface('source')} title={source.title}>
            {sourceState?.title || source.title || '来源网页'}
          </button>}
        </div>
        <div className={styles.actions} aria-label="网页控制">
          <button className={styles.action} type="button" title="后退" onClick={() => void control('back')}><ArrowLeft size={15} /></button>
          <button className={styles.action} type="button" title="前进" disabled={surface === 'official'} onClick={() => void control('forward')}><ArrowRight size={15} /></button>
          {surface === 'official' && <button className={styles.action} type="button" title="返回官网首页" onClick={() => void control('home')}><House size={15} /></button>}
          <button className={styles.action} type="button" title="刷新" onClick={() => void control('reload')}><RefreshCw size={15} /></button>
          <button className={styles.action} type="button" title="使用系统浏览器打开" onClick={() => void control('external')}><ExternalLink size={15} /></button>
          <button className={styles.action} type="button" title="关闭当前标签" onClick={closeActive}><X size={16} /></button>
        </div>
      </div>
      <div className={styles.host} ref={hostRef}>
        <p className={styles.status} data-error={error ? 'true' : undefined}>{error || status || '正在打开网页…'}</p>
      </div>
    </section>
  )
}

async function hideSurfaces(official: OfficialAiTabRequest | null) {
  await controlInternalBrowserTab('hide').catch(() => null)
  if (official) await hideLocalAiWebSessionEmbedded(official).catch(() => null)
}

function messageFor(cause: unknown) {
  return cause instanceof Error && cause.message ? cause.message : '内部网页标签打开失败，请使用系统浏览器。'
}
