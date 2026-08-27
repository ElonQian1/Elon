import { useCallback, useEffect, useRef } from 'react'
import {
  controlLocalAiWebSession,
  requestLocalAiCurrentConversationRefresh,
  requestLocalAiWebSnapshot,
  type LocalAiMessageSnapshot,
  type LocalAiWebProvider,
} from './localAiBrowserApi'
import { localAiAssistantExtractionIncomplete } from './localAiAssistantContentQuality'
import { requestReturnToAiChat } from './internalBrowserApi'
import { matchingLocalAiUserIndex, normalizeLocalAiResponsePrompt } from './localAiResponseTracking'
import { shouldRequestLocalAiPrivateConversationRefresh } from './localAiPrivateConversationRefreshPolicy'
import { localAiSnapshotIsStreaming } from './localAiPrivateStreamSignal'
import { LocalAiResponseRefreshFlight } from './localAiResponseRefreshFlight'
import {
  RESPONSE_COMPLETION_SETTLE_MS,
  localAiResponseRefreshDelay,
  localAiResponseRefreshPhase,
  type LocalAiResponseRefreshPhase,
} from './localAiWebChatControllerConfig'

interface LocalAiResponseRefreshOptions {
  provider?: LocalAiWebProvider
  ownerKey: string
  snapshot: LocalAiMessageSnapshot | null
  refreshSessionState: () => void
}

export default function useLocalAiResponseRefresh({
  provider,
  ownerKey,
  snapshot,
  refreshSessionState,
}: LocalAiResponseRefreshOptions) {
  const refreshFlight = useRef(new LocalAiResponseRefreshFlight())
  const timer = useRef(0)
  const requestRef = useRef<() => void>(() => {})
  const scheduleRef = useRef<() => void>(() => {})
  const refreshPhase = useRef<LocalAiResponseRefreshPhase>('initial')
  const delayIndex = useRef(0)
  const completionObservedAt = useRef(0)
  const expectedPrompt = useRef('')
  const baselineMatchingUserCount = useRef(0)
  const startedAt = useRef(0)
  const privateRefreshAttempted = useRef(false)
  const snapshotRef = useRef(snapshot)
  snapshotRef.current = snapshot

  const cancel = useCallback(() => {
    refreshFlight.current.reset()
    window.clearTimeout(timer.current)
    timer.current = 0
    requestRef.current = () => {}
    scheduleRef.current = () => {}
    refreshPhase.current = 'initial'
    delayIndex.current = 0
    completionObservedAt.current = 0
    expectedPrompt.current = ''
    baselineMatchingUserCount.current = 0
    startedAt.current = 0
    privateRefreshAttempted.current = false
  }, [])

  const start = useCallback((prompt: string, promptBaselineMatchingUserCount = 0) => {
    cancel()
    const activeProvider = provider
    const normalizedPrompt = normalizeLocalAiResponsePrompt(prompt)
    if (!activeProvider || !ownerKey || !normalizedPrompt) return
    expectedPrompt.current = prompt
    baselineMatchingUserCount.current = Number.isFinite(promptBaselineMatchingUserCount)
      ? Math.max(0, Math.floor(promptBaselineMatchingUserCount))
      : 0
    startedAt.current = Date.now()
    const activeGeneration = refreshFlight.current.currentGeneration()
    const scheduleNext = (): void => {
      const delay = localAiResponseRefreshDelay(refreshPhase.current, delayIndex.current++)
      window.clearTimeout(timer.current)
      timer.current = delay === undefined ? 0 : window.setTimeout(request, delay)
    }
    const request = (): void => {
      timer.current = 0
      const claim = refreshFlight.current.claim(activeGeneration)
      if (claim !== 'run') return
      requestReturnToAiChat({
        providerId: activeProvider.id,
        providerName: activeProvider.displayName,
        ownerKey,
      })
      // The event above restores the React surface. This command independently
      // parks the real child WebView so late upstream focus/navigation cannot
      // cover the native answer while generation is in progress.
      void controlLocalAiWebSession(activeProvider.id, ownerKey, 'background')
        .then(() => {}, () => {})
      const requests: Promise<unknown>[] = [
        requestLocalAiWebSnapshot(activeProvider.id, ownerKey),
      ]
      if (shouldRequestLocalAiPrivateConversationRefresh({
        providerId: activeProvider.id,
        snapshot: snapshotRef.current,
        expectedPrompt: prompt,
        baselineMatchingUserCount: baselineMatchingUserCount.current,
        elapsedMs: Date.now() - startedAt.current,
        attempted: privateRefreshAttempted.current,
      })) {
        privateRefreshAttempted.current = true
        requests.push(requestLocalAiCurrentConversationRefresh(activeProvider.id, ownerKey))
      }
      void Promise.allSettled(requests).then(() => {
        const settlement = refreshFlight.current.settle(activeGeneration)
        if (settlement === 'stale') return
        refreshSessionState()
        if (settlement === 'rerun') request()
        else scheduleNext()
      })
    }
    requestRef.current = request
    scheduleRef.current = scheduleNext
    scheduleNext()
  }, [cancel, ownerKey, provider, refreshSessionState])

  useEffect(() => {
    const expected = normalizeLocalAiResponsePrompt(expectedPrompt.current)
    if (!expected || !provider || !ownerKey || !snapshot) return
    const userIndex = matchingLocalAiUserIndex(
      snapshot.messages,
      expected,
      baselineMatchingUserCount.current,
    )
    if (userIndex < 0) return
    const assistant = snapshot.messages.slice(userIndex + 1)
      .find((item) => item.role === 'assistant')
    const streaming = Boolean(assistant && localAiSnapshotIsStreaming(snapshot, assistant))
    const completed = Boolean(assistant && !streaming && !localAiAssistantExtractionIncomplete(assistant))
    const nextPhase = localAiResponseRefreshPhase({
      providerId: provider.id,
      current: refreshPhase.current,
      assistantObserved: Boolean(assistant),
      streaming,
      completed,
    })
    if (nextPhase !== refreshPhase.current) {
      refreshPhase.current = nextPhase
      delayIndex.current = 0
      window.clearTimeout(timer.current)
      timer.current = 0
      if (nextPhase === 'streaming_watchdog') scheduleRef.current()
    }
    if (!completed) return

    requestReturnToAiChat({
      providerId: provider.id,
      providerName: provider.displayName,
      ownerKey,
    })
    void controlLocalAiWebSession(provider.id, ownerKey, 'background').then(() => {}, () => {})
    const now = Date.now()
    if (!completionObservedAt.current) completionObservedAt.current = now
    if (now - completionObservedAt.current >= RESPONSE_COMPLETION_SETTLE_MS) {
      cancel()
      return
    }
    window.clearTimeout(timer.current)
    scheduleRef.current()
  }, [cancel, ownerKey, provider, snapshot])

  useEffect(() => cancel, [cancel])

  return {
    expectedResponsePrompt: expectedPrompt,
    startResponseRefresh: start,
    cancelResponseRefresh: cancel,
  }
}
