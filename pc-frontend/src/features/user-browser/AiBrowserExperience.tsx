import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { ArrowLeft, ArrowRight, ExternalLink, House, LockKeyhole, RefreshCw, X } from 'lucide-react'
import {
  boundsFor,
  announceAiBrowserSurface,
  controlInternalBrowserTab,
  controlOfficialAiTab,
  getInternalBrowserTabState,
  hideLocalAiWebSessionEmbedded,
  OPEN_INTERNAL_BROWSER_LINK_EVENT,
  OPEN_OFFICIAL_AI_TAB_EVENT,
  openInternalBrowserTab,
  presentLocalAiWebSessionEmbedded,
  resizeInternalBrowserTab,
  REQUEST_RETURN_TO_AI_CHAT_EVENT,
  type InternalBrowserLinkRequest,
  type InternalBrowserTabState,
  type AiBrowserSurface,
  type OfficialAiTabRequest,
} from './internalBrowserApi'
import styles from './AiBrowserExperience.module.css'

export default function AiBrowserExperience() {
  const [surface, setSurface] = useState<AiBrowserSurface>('chat')
  const [official, setOfficial] = useState<OfficialAiTabRequest | null>(null)
  const [source, setSource] = useState<InternalBrowserLinkRequest | null>(null)
  const [sourceState, setSourceState] = useState<InternalBrowserTabState | null>(null)
  const [status, setStatus] = useState('')
  const [error, setError] = useState('')
  const [transitioning, setTransitioning] = useState(false)
  const viewportRef = useRef<HTMLDivElement>(null)
  const generationRef = useRef(0)
  const transitionRef = useRef(false)
  const officialRef = useRef<OfficialAiTabRequest | null>(null)
  const openedSourceUrlRef = useRef('')
  const sourceOpenedAtRef = useRef(0)
  const sourceFallbackOpenedRef = useRef(false)
  officialRef.current = official

  const activateSurface = useCallback((next: AiBrowserSurface) => {
    announceAiBrowserSurface(next)
    setSurface(next)
  }, [])

  const runTransition = useCallback(async (work: () => Promise<void>) => {
    if (transitionRef.current) return
    transitionRef.current = true
    setTransitioning(true)
    setError('')
    try {
      await work()
    } catch (cause) {
      setError(messageFor(cause))
    } finally {
      transitionRef.current = false
      setTransitioning(false)
    }
  }, [])

  const switchSurface = useCallback(async (next: AiBrowserSurface) => {
    if (next === surface) return
    if (next === 'chat') {
      // Chat is a foreground intent, not merely another queued transition. Invalidate
      // an older async `present` immediately and hide both child surfaces even when a
      // previous transition has not finished yet.
      generationRef.current += 1
      activateSurface('chat')
      const activeOfficial = officialRef.current
      try {
        if (openedSourceUrlRef.current) {
          await controlInternalBrowserTab('hide').catch(() => null)
        }
        if (activeOfficial) await hideOfficialSurface(activeOfficial)
      } catch (cause) {
        setError(messageFor(cause))
      }
      return
    }
    await runTransition(async () => {
      if (surface === 'official' && official) await hideOfficialSurface(official)
      if (surface === 'source' && openedSourceUrlRef.current) {
        const hidden = await controlInternalBrowserTab('hide')
        if (hidden?.visible) throw new Error('来源网页未能收起，请重试。')
      }
      activateSurface(next)
    })
  }, [activateSurface, official, runTransition, surface])

  useEffect(() => {
    const openOfficial = (event: Event) => {
      const detail = (event as CustomEvent<OfficialAiTabRequest>).detail
      if (!detail?.providerId || !detail.ownerKey) return
      setOfficial(detail)
      void switchSurface('official')
    }
    const openSource = (event: Event) => {
      const detail = (event as CustomEvent<InternalBrowserLinkRequest>).detail
      if (!detail?.url) return
      setSource(detail)
      sourceOpenedAtRef.current = Date.now()
      sourceFallbackOpenedRef.current = false
      void switchSurface('source')
    }
    const returnToChat = (event: Event) => {
      const request = (event as CustomEvent<OfficialAiTabRequest | undefined>).detail
      if (surface === 'chat' && request) {
        void runTransition(() => hideOfficialSurface(request))
        return
      }
      void switchSurface('chat')
    }
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && surface !== 'chat') {
        event.preventDefault()
        void switchSurface('chat')
      }
    }
    window.addEventListener(OPEN_OFFICIAL_AI_TAB_EVENT, openOfficial)
    window.addEventListener(OPEN_INTERNAL_BROWSER_LINK_EVENT, openSource)
    window.addEventListener(REQUEST_RETURN_TO_AI_CHAT_EVENT, returnToChat)
    window.addEventListener('keydown', handleKeyDown)
    return () => {
      window.removeEventListener(OPEN_OFFICIAL_AI_TAB_EVENT, openOfficial)
      window.removeEventListener(OPEN_INTERNAL_BROWSER_LINK_EVENT, openSource)
      window.removeEventListener(REQUEST_RETURN_TO_AI_CHAT_EVENT, returnToChat)
      window.removeEventListener('keydown', handleKeyDown)
    }
  }, [runTransition, surface, switchSurface])

  useLayoutEffect(() => {
    const generation = ++generationRef.current
    if (surface === 'chat') return
    let frame = 0
    let observer: ResizeObserver | null = null
    const synchronize = async () => {
      const viewport = viewportRef.current
      if (!viewport || generation !== generationRef.current) return
      setError('')
      try {
        const bounds = boundsFor(viewport)
        if (surface === 'official' && official) {
          await presentLocalAiWebSessionEmbedded(official, bounds)
          if (generation !== generationRef.current) {
            await hideOfficialSurface(official)
            return
          }
          setStatus(`官网原生内容 · ${official.providerName} 的天气、地图、图片、图标和交互按官网原样显示`)
        } else if (surface === 'source' && source) {
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
      const viewport = viewportRef.current
      if (viewport) {
        observer = new ResizeObserver(() => {
          window.cancelAnimationFrame(frame)
          frame = window.requestAnimationFrame(() => { void synchronize() })
        })
        observer.observe(viewport)
      }
    })
    return () => {
      if (generationRef.current === generation) generationRef.current += 1
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

  useEffect(() => () => {
    void hideSurfacesOnUnmount(officialRef.current, Boolean(openedSourceUrlRef.current))
  }, [])

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

  async function closeActive() {
    await runTransition(async () => {
      if (surface === 'source') {
        if (openedSourceUrlRef.current) await controlInternalBrowserTab('close')
        openedSourceUrlRef.current = ''
        setSource(null)
        setSourceState(null)
        activateSurface(official ? 'official' : 'chat')
        return
      }
      if (official) await hideOfficialSurface(official)
      setOfficial(null)
      activateSurface(source ? 'source' : 'chat')
    })
  }

  const siteName = surface === 'official'
    ? official?.providerId === 'chatgpt' ? 'chatgpt.com' : 'google.com/aimode'
    : sourceState?.currentHost || hostFor(source?.url) || '来源网页'
  const siteDetail = surface === 'official'
    ? '官方网页 · 本机会话 Profile'
    : '来源网页 · 临时隔离标签'

  const content = (
    <section className={styles.surface} aria-label="一龙内部网页标签" aria-busy={transitioning}>
      <div className={styles.tabStrip}>
        <div className={styles.tabs} role="tablist" aria-label="AI 与网页标签">
          <button className={styles.tab} type="button" role="tab" aria-selected={false} disabled={transitioning} onClick={() => void switchSurface('chat')}>
            <span className={styles.tabIcon}>龙</span><span className={styles.tabLabel}>一龙聊天</span>
          </button>
          {official && <button className={styles.tab} data-active={surface === 'official'} type="button" role="tab" aria-selected={surface === 'official'} disabled={transitioning} onClick={() => void switchSurface('official')} title={`${official.providerName} 官方页`}>
            <span className={styles.tabIcon} data-provider={official.providerId}>{official.providerId === 'chatgpt' ? '◎' : 'G'}</span>
            <span className={styles.tabLabel}>{official.providerName} 官方页</span>
          </button>}
          {source && <button className={styles.tab} data-active={surface === 'source'} type="button" role="tab" aria-selected={surface === 'source'} disabled={transitioning} onClick={() => void switchSurface('source')} title={source.title}>
            <span className={styles.tabIcon}>↗</span>
            <span className={styles.tabLabel}>{sourceState?.title || source.title || '来源网页'}</span>
          </button>}
        </div>
        <button className={styles.closeTab} type="button" title="关闭当前标签" disabled={transitioning} onClick={() => void closeActive()}><X size={15} /></button>
      </div>
      <div className={styles.navigationBar}>
        <div className={styles.actions} aria-label="网页导航">
          <button className={styles.action} type="button" title="后退" disabled={transitioning} onClick={() => void control('back')}><ArrowLeft size={15} /></button>
          <button className={styles.action} type="button" title="前进" disabled={transitioning || surface === 'official'} onClick={() => void control('forward')}><ArrowRight size={15} /></button>
          <button className={styles.action} type="button" title="刷新" disabled={transitioning} onClick={() => void control('reload')}><RefreshCw size={15} /></button>
          {surface === 'official' && <button className={styles.action} type="button" title="返回官网首页" disabled={transitioning} onClick={() => void control('home')}><House size={15} /></button>}
        </div>
        <div className={styles.siteIdentity} title={siteDetail}>
          <LockKeyhole size={13} aria-hidden="true" />
          <strong>{siteName}</strong>
          <span>{siteDetail}</span>
        </div>
        <span className={styles.notice} data-error={error ? 'true' : undefined}>
          {error || status || (surface === 'official' ? '正在准备官网原生内容…' : '正在打开来源网页…')}
        </span>
        <button className={styles.action} type="button" title="使用系统浏览器打开" disabled={transitioning} onClick={() => void control('external')}><ExternalLink size={15} /></button>
      </div>
      <div className={styles.viewport} data-browser-viewport="native" ref={viewportRef}>
        <p className={styles.status} data-error={error ? 'true' : undefined}>
          {error || status || '正在打开网页…'}
          {error && <button type="button" onClick={() => void control('external')}>使用系统浏览器</button>}
        </p>
      </div>
    </section>
  )

  const portalTarget = document.querySelector<HTMLElement>('[data-ai-surface="production-home"]')
  return portalTarget ? createPortal(content, portalTarget) : content
}

async function hideOfficialSurface(official: OfficialAiTabRequest) {
  const state = await hideLocalAiWebSessionEmbedded(official)
  if (state.windowVisible) throw new Error(`${official.providerName} 官方页未能收起，请重试。`)
}

async function hideSurfacesOnUnmount(official: OfficialAiTabRequest | null, sourceOpened: boolean) {
  if (sourceOpened) await controlInternalBrowserTab('hide').catch(() => null)
  if (official) await hideLocalAiWebSessionEmbedded(official).catch(() => null)
}

function messageFor(cause: unknown) {
  return cause instanceof Error && cause.message ? cause.message : '内部网页标签打开失败，请使用系统浏览器。'
}

function hostFor(value: string | undefined) {
  if (!value) return ''
  try { return new URL(value).hostname }
  catch { return '' }
}
