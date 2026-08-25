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
import { lastMatchingLocalAiUserIndex, normalizeLocalAiResponsePrompt } from './localAiResponseTracking'
import { shouldRequestLocalAiPrivateConversationRefresh } from './localAiPrivateConversationRefreshPolicy'
import {
  RESPONSE_COMPLETION_REFRESH_MS,
  RESPONSE_COMPLETION_SETTLE_MS,
  RESPONSE_REFRESH_DELAYS_MS,
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
  const generation = useRef(0)
  const timer = useRef(0)
  const requestRef = useRef<() => void>(() => {})
  const completionObservedAt = useRef(0)
  const expectedPrompt = useRef('')
  const startedAt = useRef(0)
  const privateRefreshAttempted = useRef(false)
  const snapshotRef = useRef(snapshot)
  snapshotRef.current = snapshot

  const cancel = useCallback(() => {
    generation.current += 1
    window.clearTimeout(timer.current)
    timer.current = 0
    requestRef.current = () => {}
    completionObservedAt.current = 0
    expectedPrompt.current = ''
    startedAt.current = 0
    privateRefreshAttempted.current = false
  }, [])

  const start = useCallback((prompt: string) => {
    cancel()
    const activeProvider = provider
    const normalizedPrompt = normalizeLocalAiResponsePrompt(prompt)
    if (!activeProvider || !ownerKey || !normalizedPrompt) return
    expectedPrompt.current = prompt
    startedAt.current = Date.now()
    const activeGeneration = generation.current
    let delayIndex = 0
    const request = () => {
      if (activeGeneration !== generation.current) return
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
      void requestLocalAiWebSnapshot(activeProvider.id, ownerKey)
        .then(refreshSessionState, () => {})
      if (shouldRequestLocalAiPrivateConversationRefresh({
        providerId: activeProvider.id,
        snapshot: snapshotRef.current,
        expectedPrompt: prompt,
        elapsedMs: Date.now() - startedAt.current,
        attempted: privateRefreshAttempted.current,
      })) {
        privateRefreshAttempted.current = true
        void requestLocalAiCurrentConversationRefresh(activeProvider.id, ownerKey)
          .then(refreshSessionState, () => {})
      }
      const delay = completionObservedAt.current
        ? RESPONSE_COMPLETION_REFRESH_MS
        : RESPONSE_REFRESH_DELAYS_MS[delayIndex++]
      if (delay !== undefined) timer.current = window.setTimeout(request, delay)
    }
    requestRef.current = request
    timer.current = window.setTimeout(
      request,
      RESPONSE_REFRESH_DELAYS_MS[delayIndex++] ?? RESPONSE_COMPLETION_REFRESH_MS,
    )
  }, [cancel, ownerKey, provider, refreshSessionState])

  useEffect(() => {
    const expected = normalizeLocalAiResponsePrompt(expectedPrompt.current)
    if (!expected || !provider || !ownerKey || !snapshot || snapshot.streaming) return
    const userIndex = lastMatchingLocalAiUserIndex(snapshot.messages, expected)
    if (userIndex < 0) return
    const completed = snapshot.messages.slice(userIndex + 1).some((item) => (
      item.role === 'assistant'
      && item.state !== 'streaming'
      && !localAiAssistantExtractionIncomplete(item)
    ))
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
    timer.current = window.setTimeout(
      () => requestRef.current(),
      RESPONSE_COMPLETION_REFRESH_MS,
    )
  }, [cancel, ownerKey, provider, snapshot])

  useEffect(() => cancel, [cancel])

  return {
    expectedResponsePrompt: expectedPrompt,
    startResponseRefresh: start,
    cancelResponseRefresh: cancel,
  }
}
