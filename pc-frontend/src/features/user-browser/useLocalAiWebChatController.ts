import { useEffect, useMemo, useRef, useState } from 'react'
import {
  controlLocalAiWebSession,
  getCachedLocalAiWebSessionState,
  getLocalAiWebSessionState,
  isLocalAiConversationSnapshot,
  isLocalAiMessageSnapshot,
  localAiBrowserErrorMessage,
  openLocalAiWebResearchDirectory,
  openLocalAiWebSession,
  runLocalAiWebAdapterCommand,
  waitForLocalAiAdapterResult,
  type LocalAiAdapterAction,
  type LocalAiBrowserControlAction,
  type LocalAiWebProvider,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'
import {
  isLocalAiComposerControlsSnapshot,
  isLocalAiFeatureNavigationSnapshot,
  isLocalAiUiManifestSnapshot,
  type LocalAiComposerControlsSnapshot,
} from './localAiBrowserProtocol'
import { deriveLocalAiUserState, type LocalAiClientState } from './localAiUserState'
import useLocalAiSessionPolling from './useLocalAiSessionPolling'
import {
  beginPendingLocalAiResponse,
  beginOptimisticLocalAiSend,
  mergeOptimisticLocalAiMessages,
  pendingLocalAiResponseObserved,
  pendingLocalAiSendObserved,
  type PendingLocalAiResponse,
  type PendingLocalAiSend,
} from './localAiOptimisticSend'
import { requestOfficialAiTab, requestReturnToAiChat } from './internalBrowserApi'
import {
  keepLocalAiNewConversationInNativeForeground,
  requestLocalAiNewConversationNativeForeground,
} from './localAiNewConversationForeground'
import {
  chatGptNewConversationResetControlAction,
  googleNewConversationNeedsReload,
  localAiNewConversationNativeReady,
  selectLocalAiNewConversationPath,
} from './localAiNewConversation'
import useChatGptNewConversationRecovery from './useChatGptNewConversationRecovery'
import useLocalAiNewConversationDeadline from './useLocalAiNewConversationDeadline'
import useLocalAiNewConversationLifecycle from './useLocalAiNewConversationLifecycle'
import { localAiComposerAvailability } from './localAiComposerAvailability'
import {
  BACKGROUND_RECONNECT_MAX_ATTEMPTS,
  GOOGLE_NEW_CONVERSATION_RELOAD_DELAY_MS,
  type QueuedLocalAiSend,
} from './localAiWebChatControllerConfig'
import useLocalAiAccessRecovery, { createLocalAiAccessRetry } from './useLocalAiAccessRecovery'
import useLocalAiResponseRefresh from './useLocalAiResponseRefresh'
import useLocalAiPendingResponseWatchdog from './useLocalAiPendingResponseWatchdog'
import useLocalAiComposerDraft from './useLocalAiComposerDraft'
import { localAiWarmSessionReusable } from './localAiWarmSessionPolicy'
import { resumeLocalAiWebSession } from './resumeLocalAiWebSession'
import useLocalAiCapabilityPrewarm from './useLocalAiCapabilityPrewarm'
import { syncLocalAiDeferredMenu } from './localAiDeferredMenuSync'
import useLocalAiCachedConversationNavigation from './useLocalAiCachedConversationNavigation'
export default function useLocalAiWebChatController(
  provider: LocalAiWebProvider | undefined,
  ownerKey: string,
  clientState: LocalAiClientState = 'ready',
) {
  const providerId = provider?.id ?? ''
  const providerDisplayName = provider?.displayName ?? ''
  const requestedSessionIdentity = providerId && ownerKey ? `${providerId}:${ownerKey}` : ''
  const activeSessionIdentity = useRef(requestedSessionIdentity)
  activeSessionIdentity.current = requestedSessionIdentity
  const [sessionEntry, setSessionEntry] = useState<{
    identity: string
    state: LocalAiWebSessionState | null
  }>(() => ({
    identity: requestedSessionIdentity,
    state: provider && ownerKey ? getCachedLocalAiWebSessionState(provider.id, ownerKey) : null,
  }))
  const { draft, draftRef, draftTouched, setDraft, setDraftTouched } = useLocalAiComposerDraft(providerId, ownerKey)
  const [pendingSends, setPendingSends] = useState<PendingLocalAiSend[]>([])
  const [pendingResponses, setPendingResponses] = useState<PendingLocalAiResponse[]>([])
  const [busyAction, setBusyAction] = useState('')
  const [message, setMessage] = useState('')
  const { baselineId: newConversationBaselineId, begin: beginNewConversationTransition,
    cancel: cancelNewConversationTransition, finish: finishNewConversationTransition,
    queuedSend, queuedSendDispatching, queuedSendRef, recoveryStartedAtMs: newConversationRecoveryStartedAtMs,
    reset: resetNewConversationTransition, setQueuedSend } = useLocalAiNewConversationLifecycle<QueuedLocalAiSend>()
  const newConversationRecoveryExpired = useLocalAiNewConversationDeadline(newConversationRecoveryStartedAtMs)
  const pendingResponseSlow = useLocalAiPendingResponseWatchdog(requestedSessionIdentity, pendingResponses)
  const autoStartKey = useRef('')
  const backgroundReconnectAttempts = useRef(0)
  const backgroundReconnectInFlight = useRef(false)
  const optimisticSendSequence = useRef(0)
  const visibleSessionState = provider && ownerKey
    ? sessionEntry.identity === requestedSessionIdentity
      && sessionEntry.state?.providerId === provider.id
      ? sessionEntry.state
      : getCachedLocalAiWebSessionState(provider.id, ownerKey)
    : null
  const liveSnapshot = useMemo(
    () => isLocalAiMessageSnapshot(visibleSessionState?.semanticEvent)
      ? visibleSessionState.semanticEvent
      : null,
    [visibleSessionState?.semanticEvent],
  )
  const snapshot = newConversationRecoveryStartedAtMs
    || visibleSessionState?.semanticConversationAligned === false
    ? null
    : liveSnapshot
  const refreshSessionState = useLocalAiSessionPolling({
    enabled: Boolean(providerId && ownerKey),
    providerId,
    ownerKey,
    state: visibleSessionState,
    onState: setSessionState,
  })
  const {
    expectedResponsePrompt,
    startResponseRefresh,
    cancelResponseRefresh,
  } = useLocalAiResponseRefresh({ provider, ownerKey, snapshot, refreshSessionState })
  const accessRecovery = useLocalAiAccessRecovery(
    requestedSessionIdentity, snapshot, pendingResponses, expectedResponsePrompt.current,
    setPendingResponses, cancelResponseRefresh, setMessage,
  )
  const visibleMessages = useMemo(
    () => mergeOptimisticLocalAiMessages(
      snapshot?.messages ?? [],
      pendingSends,
      pendingResponses,
      accessRecovery.blocked,
    ),
    [accessRecovery.blocked, pendingResponses, pendingSends, snapshot?.messages],
  )
  const navigationSnapshot = useMemo(
    () => isLocalAiConversationSnapshot(visibleSessionState?.navigationEvent)
      ? visibleSessionState.navigationEvent
      : null,
    [visibleSessionState?.navigationEvent],
  )
  const composerSnapshot = useMemo(
    () => isLocalAiComposerControlsSnapshot(visibleSessionState?.composerEvent)
      ? visibleSessionState.composerEvent
      : null,
    [visibleSessionState?.composerEvent],
  )
  const featureSnapshot = useMemo(
    () => isLocalAiFeatureNavigationSnapshot(visibleSessionState?.featureEvent)
      ? visibleSessionState.featureEvent
      : null,
    [visibleSessionState?.featureEvent],
  )
  const uiManifest = useMemo(
    () => isLocalAiUiManifestSnapshot(visibleSessionState?.uiManifestEvent)
      ? visibleSessionState.uiManifestEvent
      : null,
    [visibleSessionState?.uiManifestEvent],
  )
  const sessionOpen = Boolean(visibleSessionState && visibleSessionState.windowStatus !== 'closed')
  const userState = useMemo(() => {
    const next = deriveLocalAiUserState(clientState, provider, visibleSessionState, liveSnapshot)
    if (!newConversationRecoveryStartedAtMs) return next
    return {
      ...next,
      canNewConversation: false,
      features: next.features.map((feature) => feature.id === 'new_conversation'
        ? { ...feature, active: false }
        : feature),
    }
  }, [clientState, liveSnapshot, newConversationRecoveryStartedAtMs, provider, visibleSessionState])
  const composerAvailability = localAiComposerAvailability({
    clientReady: clientState === 'ready',
    providerAvailable: Boolean(provider),
    sendSupported: Boolean(provider?.adapterActions.includes('send_prompt')),
    directSendReady: userState.canSend,
    newConversationRecoveryActive: Boolean(newConversationRecoveryStartedAtMs),
    queuedSendActive: Boolean(queuedSend),
    sendFlightActive: pendingSends.length > 0 || pendingResponses.length > 0,
    busyAction,
  })
  const capabilityPrewarmBlocked = Boolean(
    busyAction || draft.trim() || pendingSends.length || pendingResponses.length
      || queuedSend || newConversationRecoveryStartedAtMs,
  )

  function setSessionState(state: LocalAiWebSessionState | null) {
    setSessionEntry({ identity: requestedSessionIdentity, state })
  }

  useEffect(() => {
    setSessionState(providerId && ownerKey
      ? getCachedLocalAiWebSessionState(providerId, ownerKey)
      : null)
    setPendingSends([])
    setPendingResponses([])
    resetNewConversationTransition()
    setBusyAction('')
    setMessage('')
    autoStartKey.current = ''
    backgroundReconnectAttempts.current = 0
    backgroundReconnectInFlight.current = false
    cancelResponseRefresh()
  }, [ownerKey, providerId, requestedSessionIdentity])

  useEffect(() => {
    if (!newConversationRecoveryStartedAtMs) return
    const nativeReady = localAiNewConversationNativeReady(
      visibleSessionState,
      liveSnapshot,
      newConversationRecoveryStartedAtMs,
      newConversationBaselineId.current,
    )
    // A page-level new-chat receipt is provisional: guest ChatGPT can briefly show an
    // empty root surface and then restore the previous conversation from its profile.
    // Only the native snapshot proves that the replacement conversation identity,
    // empty message surface, composer and semantic cache all agree.
    const queuedSendReady = nativeReady
    if (nativeReady && !queuedSend) return finishNewConversationTransition()
    if (queuedSendReady && queuedSend && !busyAction && !queuedSendDispatching.current) {
      queuedSendDispatching.current = true
      finishNewConversationTransition()
      setQueuedSend(null)
      void dispatchPreparedPrompt(queuedSend)
        .finally(() => { queuedSendDispatching.current = false })
      return
    }
    // 五个条件里任何一个（尤其是官网迟迟不给可信实时快照）迟迟凑不齐时，不能让
    // 输入框永远显示空白；排队消息只在新会话绑定成功后发送，超时则安全还原草稿。
    if (!newConversationRecoveryExpired) return
    cancelNewConversationTransition(restoreQueuedSend)
    setMessage(queuedSend
      ? '新会话后台连接超时，消息没有误发；草稿已保留，可显示官方页确认后重试。'
      : '新会话后台连接超时；输入仍可继续编辑，如页面显示旧内容可打开官方页确认。')
  }, [busyAction, liveSnapshot, newConversationRecoveryExpired,
    newConversationRecoveryStartedAtMs, providerId, queuedSend, visibleSessionState])

  useEffect(() => {
    if (!newConversationRecoveryStartedAtMs || providerId !== 'google-ai-mode' || !ownerKey) return
    let active = true
    const timer = window.setTimeout(() => {
      void getLocalAiWebSessionState(providerId, ownerKey)
        .then(async (current) => {
          const currentSnapshot = isLocalAiMessageSnapshot(current.semanticEvent)
            ? current.semanticEvent
            : null
          if (!active || !googleNewConversationNeedsReload(current, currentSnapshot)) return
          const next = await controlLocalAiWebSession(providerId, ownerKey, 'reload')
          if (!active) return
          setSessionState(next)
          setMessage('Google AI 模式首次新会话加载较慢，已在后台自动重试一次。')
        })
        .catch(() => {
          // 最终恢复超时仍会释放输入框，不让官网偶发加载失败永久阻塞原生 UI。
        })
    }, GOOGLE_NEW_CONVERSATION_RELOAD_DELAY_MS)
    return () => {
      active = false
      window.clearTimeout(timer)
    }
  }, [newConversationRecoveryStartedAtMs, ownerKey, providerId])

  useChatGptNewConversationRecovery({
    providerId,
    ownerKey,
    startedAtMs: newConversationRecoveryStartedAtMs,
    baselineConversationId: newConversationBaselineId.current,
    suspended: busyAction === 'new_conversation',
    onState: setSessionState,
    onMessage: setMessage,
  })

  useEffect(() => {
    if (!providerId || !ownerKey) return
    const key = requestedSessionIdentity
    if (autoStartKey.current === key) return
    autoStartKey.current = key
    let active = true
    const cachedState = getCachedLocalAiWebSessionState(providerId, ownerKey)
    const warmSession = localAiWarmSessionReusable(cachedState, providerId)
    if (!warmSession) setMessage(`正在后台连接 ${providerDisplayName}；官网允许时可直接使用访客模式。`)
    void resumeLocalAiWebSession(providerId, ownerKey, cachedState)
      .then(({ state, reused }) => {
        if (active && state) setSessionState(state)
        if (active && !reused) setMessage(`${providerDisplayName} 已在本机后台连接；登录是历史与增强能力的可选项。`)
      })
      .catch((error) => {
        if (active) setMessage(localAiBrowserErrorMessage(error))
      })
    return () => { active = false }
  }, [ownerKey, providerDisplayName, providerId, requestedSessionIdentity])

  useEffect(() => {
    if (!providerId || !ownerKey) return
    if (visibleSessionState?.windowStatus !== 'closed') {
      backgroundReconnectAttempts.current = 0
      return
    }
    if (backgroundReconnectInFlight.current
      || backgroundReconnectAttempts.current >= BACKGROUND_RECONNECT_MAX_ATTEMPTS) return
    backgroundReconnectAttempts.current += 1
    backgroundReconnectInFlight.current = true
    let active = true
    void openLocalAiWebSession(providerId, ownerKey, { showWindow: false })
      .then(() => getLocalAiWebSessionState(providerId, ownerKey))
      .then((next) => { if (active) setSessionState(next) })
      .catch(() => {
        // 下一次轮询若仍是 closed，会在次数用尽前再试一次。
      })
      .finally(() => { backgroundReconnectInFlight.current = false })
    return () => { active = false }
  }, [providerId, ownerKey, visibleSessionState?.windowStatus, visibleSessionState?.updatedAtMs])

  useEffect(() => {
    if (!draftTouched) {
      const nextDraft = snapshot?.draft ?? ''
      setDraft(nextDraft)
    }
  }, [draftTouched, snapshot?.draft])

  useEffect(() => {
    if (!snapshot || pendingSends.length === 0) return
    const remaining = pendingSends.filter((pending) => (
      !pendingLocalAiSendObserved(snapshot.messages, pending)
    ))
    if (remaining.length === pendingSends.length) return
    setPendingSends(remaining)
    if (remaining.length === 0 && !snapshot.draft && !draftRef.current) {
      setDraftTouched(false)
    }
  }, [pendingSends, snapshot])

  useEffect(() => {
    if (!snapshot || pendingResponses.length === 0) return
    const remaining = pendingResponses.filter((pending) => (
      !pendingLocalAiResponseObserved(snapshot.messages, pending)
    ))
    if (remaining.length !== pendingResponses.length) setPendingResponses(remaining)
  }, [pendingResponses, snapshot])

  async function openOfficial() {
    if (!provider || !ownerKey || busyAction) return
    setBusyAction('open')
    setMessage('')
    try {
      await openLocalAiWebSession(provider.id, ownerKey, { showWindow: false })
      try {
        setSessionState(await getLocalAiWebSessionState(provider.id, ownerKey))
      } catch {
        // The bounded poll recovers a state refresh without reopening the window.
      }
      requestOfficialAiTab({ providerId: provider.id, providerName: provider.displayName, ownerKey })
      setMessage(`已切换到 ${provider.displayName} 官方原生标签；天气、地图、图标和交互内容由官网直接显示。`)
    } catch (error) {
      setMessage(localAiBrowserErrorMessage(error))
    } finally {
      setBusyAction('')
    }
  }

  async function openResearchDirectory() {
    if (!provider || !ownerKey || busyAction) return
    setBusyAction('research-directory')
    setMessage('')
    try {
      await openLocalAiWebResearchDirectory(provider.id, ownerKey)
      setMessage('已打开当前厂商的本机原始响应研究目录。')
    } catch (error) {
      setMessage(localAiBrowserErrorMessage(error))
    } finally {
      setBusyAction('')
    }
  }

  async function control(action: LocalAiBrowserControlAction) {
    if (!provider || !ownerKey || busyAction) return
    setBusyAction(action)
    setMessage('')
    try {
      setSessionState(await controlLocalAiWebSession(provider.id, ownerKey, action))
      if (action === 'external') {
        setMessage('已打开系统浏览器；系统浏览器不会与一龙本地窗口共享 Cookie。')
      } else if (action === 'background') {
        setMessage(`${provider.displayName} 官方页已收起到本机后台，一龙聊天界面可以继续使用。`)
      }
    } catch (error) {
      setMessage(localAiBrowserErrorMessage(error))
    } finally {
      setBusyAction('')
    }
  }

  const openCachedConversation = useLocalAiCachedConversationNavigation({
    provider,
    ownerKey,
    sessionIdentity: requestedSessionIdentity,
    busyAction,
    isSessionCurrent: (identity) => activeSessionIdentity.current === identity,
    beforeOpen: () => {
    cancelNewConversationTransition(restoreQueuedSend)
    setPendingSends([])
    setPendingResponses([])
    cancelResponseRefresh()
    },
    onBusyAction: setBusyAction,
    onMessage: setMessage,
    onState: setSessionState,
  })

  async function run(action: LocalAiAdapterAction, value?: string, expectedDraft?: string) {
    if (!provider || !ownerKey) return null
    if (!provider.adapterActions.includes(action)) {
      setMessage(`${provider.displayName} 当前不支持这个原生动作；可以显示官方窗口继续使用。`)
      return null
    }
    if (action === 'send_prompt' && composerAvailability.shouldQueue) {
      const pending = preparePendingSend(value ?? '')
      if (!pending) return null
      const queued = {
        prompt: pending.prompt,
        expectedDraft: expectedDraft ?? '',
        pending,
        sessionIdentity: requestedSessionIdentity,
      }
      setQueuedSend(queued)
      setMessage('消息已保存在本机新会话队列；官网在后台完成绑定后会自动发送。')
      return visibleSessionState
    }
    if (action === 'new_conversation' && newConversationRecoveryStartedAtMs) {
      setMessage('当前新会话仍在后台确认；为避免串到上一会话，确认完成前不会重复新建。')
      return visibleSessionState
    }
    if (busyAction || (action === 'send_prompt' && !composerAvailability.canSubmit)) return null
    if (action === 'new_conversation') {
      return startNewConversation()
    }
    if (action === 'send_prompt') {
      const pending = preparePendingSend(value ?? '')
      if (!pending) return null
      return dispatchPreparedPrompt({
        prompt: pending.prompt,
        expectedDraft: expectedDraft ?? '',
        pending,
        sessionIdentity: requestedSessionIdentity,
      })
    }
    setBusyAction(action)
    if (['open_conversation', 'open_project'].includes(action)) {
      cancelNewConversationTransition(restoreQueuedSend)
      cancelResponseRefresh()
    }
    setMessage('')
    try {
      const requestId = await runLocalAiWebAdapterCommand(provider.id, ownerKey, action, value, expectedDraft)
      const next = await waitForLocalAiAdapterResult(provider.id, ownerKey, action, requestId)
      if (!next) {
        setMessage('没有收到当前命令的匹配回执；为避免误判，一龙没有把这次操作标记为成功。请显示官方页检查后重试。')
        return null
      }
      if (next) setSessionState(next)
      const result = next?.commandResult
      if (result?.action === action && !result.ok) {
        setMessage(result.detail || '官方网页没有完成这个动作，请显示官方窗口后重试。')
      } else if (result?.detail) {
        setMessage(result.detail)
      }
      return next
    } catch (error) {
      setMessage(localAiBrowserErrorMessage(error))
      return null
    } finally {
      setBusyAction('')
    }
  }

  function preparePendingSend(prompt: string): PendingLocalAiSend | null {
    if (!provider) return null
    const pending = beginOptimisticLocalAiSend(
      snapshot?.messages ?? [],
      pendingSends,
      prompt,
      `optimistic-${provider.id}-${Date.now()}-${optimisticSendSequence.current++}`,
    )
    if (!pending) return null
    setPendingSends((current) => current.concat(pending))
    setPendingResponses((current) => current.concat(beginPendingLocalAiResponse(pending)))
    setDraft('')
    setDraftTouched(true)
    return pending
  }

  async function dispatchPreparedPrompt(
    prepared: QueuedLocalAiSend,
  ): Promise<LocalAiWebSessionState | null> {
    if (!provider || !ownerKey || prepared.sessionIdentity !== requestedSessionIdentity) {
      restoreQueuedSend(prepared)
      return null
    }
    setBusyAction('send_prompt')
    setMessage('')
    try {
      // Sending belongs to the production native surface. Explicitly cancel any
      // in-flight official-tab presentation and park the child WebView before its
      // page command can navigate or focus itself.
      requestReturnToAiChat({
        providerId: provider.id,
        providerName: provider.displayName,
        ownerKey,
      })
      setSessionState(await controlLocalAiWebSession(provider.id, ownerKey, 'background'))
      const requestId = await runLocalAiWebAdapterCommand(
        provider.id,
        ownerKey,
        'send_prompt',
        prepared.prompt,
        prepared.expectedDraft,
      )
      const next = await waitForLocalAiAdapterResult(
        provider.id,
        ownerKey,
        'send_prompt',
        requestId,
      )
      if (!next) {
        restoreQueuedSend(prepared)
        setMessage('没有收到当前发送的匹配回执；消息没有标记为成功，草稿已保留。')
        return null
      }
      // The official page may accept the prompt by navigating or focusing after the
      // initial background command has completed. Reassert native foreground ownership
      // after the matching receipt so that late WebView focus cannot cover the reply.
      requestReturnToAiChat({
        providerId: provider.id,
        providerName: provider.displayName,
        ownerKey,
      })
      let foregroundState = next
      try {
        foregroundState = await controlLocalAiWebSession(provider.id, ownerKey, 'background')
      } catch {
        // Response polling below continues to reassert the same bounded foreground intent.
      }
      setSessionState(foregroundState)
      const result = next.commandResult
      if (result?.action === 'send_prompt' && !result.ok) {
        restoreQueuedSend(prepared)
        setMessage(result.detail || '官方网页没有完成发送，草稿已保留；可显示官方窗口后重试。')
      } else {
        setMessage(result?.detail || '消息已交给官方网页发送；正在一龙聊天界面同步回复。')
        startResponseRefresh(prepared.prompt)
      }
      return next
    } catch (error) {
      restoreQueuedSend(prepared)
      setMessage(localAiBrowserErrorMessage(error))
      return null
    } finally {
      setBusyAction('')
    }
  }

  function beginLocalNewConversation() {
    const previousDraft = draftRef.current
    beginNewConversationTransition(visibleSessionState?.activeConversationId ?? '')
    setPendingSends([])
    setPendingResponses([])
    setQueuedSend(null)
    queuedSendDispatching.current = false
    cancelResponseRefresh()
    setDraft('')
    setDraftTouched(false)
    setMessage(`已在本机进入 ${provider?.displayName ?? '网页 AI'} 新会话；官网正在后台同步。`)
    return previousDraft
  }

  async function startNewConversation(retryPrompt = ''): Promise<LocalAiWebSessionState | null> {
    if (!provider || !ownerKey) return null
    const path = selectLocalAiNewConversationPath(provider.id, visibleSessionState, snapshot)
    const previousDraft = beginLocalNewConversation()
    requestLocalAiNewConversationNativeForeground(provider, ownerKey)
    const retry = createLocalAiAccessRetry(
      requestedSessionIdentity, retryPrompt,
      `optimistic-${provider.id}-${Date.now()}-${optimisticSendSequence.current++}`,
    )
    if (retry) {
      setPendingSends([retry.pending]); setPendingResponses([retry.response])
      setQueuedSend(retry.queued)
      accessRecovery.dismiss()
    }
    setBusyAction('new_conversation')
    try {
      if (path === 'adapter') {
        try {
          const requestId = await runLocalAiWebAdapterCommand(
            provider.id,
            ownerKey,
            'new_conversation',
          )
          const next = await waitForLocalAiAdapterResult(
            provider.id,
            ownerKey,
            'new_conversation',
            requestId,
          )
          if (next) {
            const result = next.commandResult
            if (result?.action !== 'new_conversation' || result.ok) {
              const background = await keepLocalAiNewConversationInNativeForeground(provider, ownerKey, next)
              setSessionState(background)
              if (result?.action === 'new_conversation' && result.ok) {
                setMessage(`已确认 ${provider.displayName} 空白新会话；首条消息可立即输入，原生会话绑定完成后自动发送。`)
              } else {
                setMessage(`已进入 ${provider.displayName} 新会话；可以立即输入，官网上下文正在后台确认。`)
              }
              return background
            }
            setSessionState(next)
          }
        } catch {
          // 活跃适配器偶发失效时复用同一个本地新会话边界，静默切到首页恢复。
        }
        return openNewConversationHome(previousDraft, '官网新会话动作未能确认，已在后台自动恢复。')
      }
      return openNewConversationHome(previousDraft)
    } finally {
      setBusyAction('')
    }
  }

  function retryLoginBlockedPrompt() {
    return !accessRecovery.prompt.trim() || busyAction ? null : startNewConversation(accessRecovery.prompt)
  }

  async function openNewConversationHome(
    previousDraft: string,
    recoveryMessage = '',
  ): Promise<LocalAiWebSessionState | null> {
    if (!provider || !ownerKey) return null
    try {
      const next = await controlLocalAiWebSession(
        provider.id,
        ownerKey,
        provider.id === 'chatgpt'
          ? chatGptNewConversationResetControlAction(visibleSessionState?.currentUrl)
          : 'home',
      )
      const background = await keepLocalAiNewConversationInNativeForeground(provider, ownerKey, next)
      setSessionState(background)
      setMessage([
        recoveryMessage,
        `已进入 ${provider.displayName} 新会话；可以立即输入，提前发送的消息会在绑定确认后自动提交。`,
      ].filter(Boolean).join(' '))
      return background
    } catch (error) {
      failLocalNewConversation(previousDraft, error)
      return null
    }
  }

  function failLocalNewConversation(previousDraft: string, error: unknown) {
    const queued = queuedSendRef.current
    cancelNewConversationTransition(restoreQueuedSend)
    if (!queued && !draftRef.current && previousDraft) {
      setDraft(previousDraft)
      setDraftTouched(true)
    }
    setMessage(queued
      ? `${localAiBrowserErrorMessage(error)} 消息没有误发，草稿已保留。`
      : `${localAiBrowserErrorMessage(error)} 输入框仍可使用，原草稿已保留。`)
  }

  function rollbackPendingSend(pending: PendingLocalAiSend | null) {
    if (!pending) return
    setPendingSends((current) => current.filter((item) => item.id !== pending.id))
    setPendingResponses((current) => current.filter((item) => item.sendId !== pending.id))
    if (!draftRef.current) {
      setDraft(pending.prompt)
      setDraftTouched(true)
    }
  }

  function restoreQueuedSend(queued: QueuedLocalAiSend) {
    rollbackPendingSend(queued.pending)
  }

  async function refreshComposerControls(section: LocalAiComposerControlsSnapshot['section']) {
    const listAction = section === 'model' ? 'list_model_options' : 'list_composer_tools'
    const collectAction = section === 'model' ? 'collect_model_options' : 'collect_composer_tools'
    return refreshDeferredMenu(listAction, collectAction)
  }

  async function refreshFeatureNavigation() {
    return refreshDeferredMenu('list_navigation', 'collect_navigation')
  }

  async function refreshDeferredMenu(
    listAction: LocalAiAdapterAction,
    collectAction: LocalAiAdapterAction,
  ) {
    if (!provider || !ownerKey || busyAction) return
    if (!provider.adapterActions.includes(listAction) || !provider.adapterActions.includes(collectAction)) {
      setMessage(`${provider.displayName} 当前没有这个官网菜单。`)
      return
    }
    setBusyAction(listAction)
    setMessage('正在读取官网可见选项…')
    try {
      const next = await syncLocalAiDeferredMenu({
        providerId: provider.id,
        ownerKey,
        sessionIdentity: requestedSessionIdentity,
        listAction,
        collectAction,
      })
      if (!next) {
        setMessage('官网菜单没有返回匹配回执；已保留现有选项，不会把旧菜单当成当前结果。')
        return
      }
      setSessionState(next)
      const result = next.commandResult
      if (result?.action !== listAction && result?.action !== collectAction) {
        setMessage('官网菜单没有返回匹配回执；已保留现有选项，不会把旧菜单当成当前结果。')
      } else if (!result.ok) {
        setMessage(result.detail || '官网菜单尚未返回可用选项。')
      } else {
        setMessage('已同步官网当前可见选项。')
      }
    } catch (error) {
      setMessage(localAiBrowserErrorMessage(error))
    } finally {
      setBusyAction('')
    }
  }

  useLocalAiCapabilityPrewarm({
    provider,
    ownerKey,
    sessionIdentity: requestedSessionIdentity,
    sessionState: visibleSessionState,
    snapshot: liveSnapshot,
    foregroundBlocked: capabilityPrewarmBlocked,
    onState: setSessionState,
  })

  return {
    sessionIdentity: requestedSessionIdentity,
    sessionState: visibleSessionState,
    snapshot,
    visibleMessages,
    navigationSnapshot,
    composerSnapshot,
    featureSnapshot,
    uiManifest,
    userState,
    sessionOpen,
    draft,
    setDraft: (value: string) => {
      setDraft(value)
      setDraftTouched(true)
    },
    busyAction,
    message,
    canEditDraft: composerAvailability.canEdit,
    canSubmitDraft: composerAvailability.canSubmit,
    queuedSendActive: Boolean(queuedSend),
    pendingResponseSlow,
    newConversationRecoveryActive: Boolean(newConversationRecoveryStartedAtMs),
    loginRecoveryPrompt: accessRecovery.prompt,
    retryLoginBlockedPrompt,
    dismissLoginRecovery: accessRecovery.dismiss,
    openOfficial,
    openResearchDirectory,
    control,
    openCachedConversation,
    run,
    refreshComposerControls,
    refreshFeatureNavigation,
  }
}
