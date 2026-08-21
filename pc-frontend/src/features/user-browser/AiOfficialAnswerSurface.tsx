import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import type { AiWebChatBackend } from './useAiWebChatBackend'
import {
  AI_BROWSER_SURFACE_CHANGED_EVENT,
  boundsFor,
  hideLocalAiWebSessionEmbedded,
  presentLocalAiWebSessionEmbedded,
  type AiBrowserSurface,
} from './internalBrowserApi'
import {
  localAiAnswerSurfaceKey,
  selectLocalAiAnswerRenderMode,
} from './localAiAnswerSurfacePolicy'
import styles from './AiOfficialAnswerSurface.module.css'

const MAX_PRESENT_ATTEMPTS = 4
const PRESENT_RETRY_BASE_MS = 450

export default function AiOfficialAnswerSurface({ web }: { web: AiWebChatBackend }) {
  const viewportRef = useRef<HTMLDivElement>(null)
  const generationRef = useRef(0)
  const [browserSurface, setBrowserSurface] = useState<AiBrowserSurface>('chat')
  const [failedKey, setFailedKey] = useState('')
  const [presentationAttempt, setPresentationAttempt] = useState(0)
  const snapshot = web.controller.snapshot
  const answerKey = useMemo(
    () => localAiAnswerSurfaceKey(web.provider?.id, snapshot),
    [snapshot, web.provider?.id],
  )
  const renderMode = selectLocalAiAnswerRenderMode({
    ready: web.ready,
    browserSurface,
    busy: Boolean(web.controller.busyAction),
    session: web.controller.sessionState,
    snapshot,
  })
  const shouldPresent = renderMode === 'official_live'
    && Boolean(answerKey && web.officialRequest)
    && failedKey !== answerKey

  useEffect(() => {
    const surfaceChanged = (event: Event) => {
      const next = (event as CustomEvent<AiBrowserSurface>).detail
      if (next === 'chat' || next === 'official' || next === 'source') {
        if (next === 'chat') {
          setFailedKey('')
          setPresentationAttempt(0)
        }
        setBrowserSurface(next)
      }
    }
    window.addEventListener(AI_BROWSER_SURFACE_CHANGED_EVENT, surfaceChanged)
    return () => {
      window.removeEventListener(AI_BROWSER_SURFACE_CHANGED_EVENT, surfaceChanged)
    }
  }, [])

  useEffect(() => {
    setFailedKey('')
    setPresentationAttempt(0)
  }, [answerKey])

  useLayoutEffect(() => {
    const request = web.officialRequest
    if (!shouldPresent || !request) return
    const generation = ++generationRef.current
    let frame = 0
    let retryTimer = 0
    let observer: ResizeObserver | null = null
    let presented = false
    let synchronizing = false
    const synchronize = async () => {
      const viewport = viewportRef.current
      if (!viewport || generation !== generationRef.current || synchronizing || retryTimer) return
      synchronizing = true
      try {
        await presentLocalAiWebSessionEmbedded(request, boundsFor(viewport), { contentOnly: true })
        if (generation !== generationRef.current) {
          await hideLocalAiWebSessionEmbedded(request).catch(() => null)
          return
        }
        presented = true
      } catch {
        if (generation === generationRef.current) {
          if (presentationAttempt + 1 < MAX_PRESENT_ATTEMPTS) {
            const delay = PRESENT_RETRY_BASE_MS * (presentationAttempt + 1)
            retryTimer = window.setTimeout(() => {
              retryTimer = 0
              setPresentationAttempt(presentationAttempt + 1)
            }, delay)
          } else {
            setFailedKey(answerKey)
          }
        }
        await hideLocalAiWebSessionEmbedded(request).catch(() => null)
      } finally {
        synchronizing = false
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
      window.clearTimeout(retryTimer)
      observer?.disconnect()
      if (presented) void hideLocalAiWebSessionEmbedded(request).catch(() => null)
    }
  }, [answerKey, presentationAttempt, shouldPresent, web.officialRequest])

  if (!shouldPresent) return null
  return (
    <section className={styles.surface} aria-label="官网实时回答区域">
      <div className={styles.viewport} ref={viewportRef} data-official-answer-viewport="true">
        <span>正在准备官网回答区域；不可用时自动显示本机缓存。</span>
      </div>
    </section>
  )
}
