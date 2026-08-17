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
  const openedSourceUrlRef = useRef('')
  const sourceOpenedAtRef = useRef(0)
  const sourceFallbackOpenedRef = useRef(false)

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
      sourceOpenedAtRef.current = Date.now()
      sourceFallbackOpenedRef.current = false
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
    const synchronize = async () => {
      const host = hostRef.current
      if (!host || generation !== generationRef.current) return
      setError('')
      try {
        const bounds = boundsFor(host)
        if (surface === 'official' && official) {
          await controlInternalBrowserTab('hide').catch(() => null)
          await presentLocalAiWebSessionEmbedded(official, bounds)
          setStatus(`官网原生内容 · ${official.providerName} 的天气、地图、图片、图标和交互按官网原样显示`)
        } else if (surface === 'source' && source) {
          if (official) await hideLocalAiWebSessionEmbedded(official).catch(() => null)
          const requestedUrl = new URL(source.url).toString()
          const shouldOpen = openedSourceUrlRef.current !== requestedUrl
          const next = shouldOpen
            ? await openInternalBrowserTab(source, bounds)
            : await controlInternalBrowserTab('show').then(async () => {
                const shown = await getInternalBrowserTabState()
                if (!shown) throw new Error('内部网页标签恢复失败。')
                await resizeInternalBrowserTab(bounds)
                return shown
              })
          if (shouldOpen) openedSourceUrlRef.current = requestedUrl
          if (generation === generationRef.current) setSourceState(next)
          setStatus(next.loading
            ? `正在加载 ${next.currentHost || '来源网页'}…`
            : `${next.currentHost || '来源网页'} · 隔离临时会话，不共享 AI 官网登录状态`)
        }
      } catch (cause) {
        if (generation === generationRef.current) setError(messageFor(cause))
      }
    }
    frame = window.requestAnimationFrame(() => {
      void synchronize()
      const host = hostRef.current
      if (host) {
        observer = new ResizeObserver(() => {
          window.cancelAnimationFrame(frame)
          frame = window.requestAnimationFrame(() => { void synchronize() })
        })
        observer.observe(host)
      }
    })
    return () => {
      window.cancelAnimationFrame(frame)
      observer?.disconnect()
    }
  }, [official, source, surface])

  useEffect(() => {
    if (surface !== 'source' || !source) return
    let active = true
    const refresh = async () => {
      try {
        const next = await getInternalBrowserTabState()
        if (!active) return
        setSourceState(next)
        if (next.lastError || (next.loading && Date.now() - sourceOpenedAtRef.current > 20_000)) {
          const reason = next.lastError || '页面加载失败或超时'
          setError(`${reason}，已改用系统浏览器打开。`)
          if (!sourceFallbackOpenedRef.current) {
            sourceFallbackOpenedRef.current = true
            await controlInternalBrowserTab('external')
          }
        } else {
          setStatus(next.loading
            ? `正在加载 ${next.currentHost || '来源网页'}…`
            : `${next.currentHost || '来源网页'} · 已加载，可随时改用系统浏览器`)
        }
      } catch (cause) {
        if (active) setError(messageFor(cause))
      }
    }
    void refresh()
    const timer = window.setInterval(() => { void refresh() }, 700)
    return () => {
      active = false
      window.clearInterval(timer)
    }
  }, [source, surface])

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
      openedSourceUrlRef.current = ''
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
        <span className={styles.notice} data-error={error ? 'true' : undefined}>
          {error || status || (surface === 'official' ? '正在准备官网原生内容…' : '正在打开来源网页…')}
        </span>
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
        <p className={styles.status} data-error={error ? 'true' : undefined}>
          {error || status || '正在打开网页…'}
          {error && <button type="button" onClick={() => void control('external')}>使用系统浏览器</button>}
        </p>
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
