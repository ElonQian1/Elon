import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import type { AiWebChatBackend } from './useAiWebChatBackend'
import {
  AI_BROWSER_SURFACE_CHANGED_EVENT,
  boundsFor,
  hideLocalAiWebSessionEmbedded,
  presentLocalAiWebSessionEmbedded,
  REQUEST_RETURN_TO_AI_CHAT_EVENT,
  type AiBrowserSurface,
} from './internalBrowserApi'
import {
  localAiAnswerSurfaceKey,
  selectLocalAiAnswerRenderMode,
} from './localAiAnswerSurfacePolicy'
import styles from './AiOfficialAnswerSurface.module.css'

export default function AiOfficialAnswerSurface({ web }: { web: AiWebChatBackend }) {
  const viewportRef = useRef<HTMLDivElement>(null)
  const generationRef = useRef(0)
  const [browserSurface, setBrowserSurface] = useState<AiBrowserSurface>('chat')
  const [dismissedKey, setDismissedKey] = useState('')
  const [failedKey, setFailedKey] = useState('')
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
    && dismissedKey !== answerKey
    && failedKey !== answerKey

  useEffect(() => {
    const surfaceChanged = (event: Event) => {
      const next = (event as CustomEvent<AiBrowserSurface>).detail
      if (next === 'chat' || next === 'official' || next === 'source') setBrowserSurface(next)
    }
    const returnToNative = () => {
      if (answerKey) setDismissedKey(answerKey)
    }
    window.addEventListener(AI_BROWSER_SURFACE_CHANGED_EVENT, surfaceChanged)
    window.addEventListener(REQUEST_RETURN_TO_AI_CHAT_EVENT, returnToNative)
    return () => {
      window.removeEventListener(AI_BROWSER_SURFACE_CHANGED_EVENT, surfaceChanged)
      window.removeEventListener(REQUEST_RETURN_TO_AI_CHAT_EVENT, returnToNative)
    }
  }, [answerKey])

  useLayoutEffect(() => {
    const request = web.officialRequest
    if (!shouldPresent || !request) return
    const generation = ++generationRef.current
    let frame = 0
    let observer: ResizeObserver | null = null
    let presented = false
    const synchronize = async () => {
      const viewport = viewportRef.current
      if (!viewport || generation !== generationRef.current) return
      try {
        await presentLocalAiWebSessionEmbedded(request, boundsFor(viewport), { contentOnly: true })
        if (generation !== generationRef.current) {
          await hideLocalAiWebSessionEmbedded(request).catch(() => null)
          return
        }
        presented = true
      } catch {
        if (generation === generationRef.current) setFailedKey(answerKey)
        await hideLocalAiWebSessionEmbedded(request).catch(() => null)
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
      if (presented) void hideLocalAiWebSessionEmbedded(request).catch(() => null)
    }
  }, [answerKey, shouldPresent, web.officialRequest])

  if (!shouldPresent) return null
  return (
    <section className={styles.surface} aria-label="官网实时回答区域">
      <div className={styles.viewport} ref={viewportRef} data-official-answer-viewport="true">
        <span>正在准备官网回答区域；不可用时自动显示本机缓存。</span>
      </div>
    </section>
  )
}
