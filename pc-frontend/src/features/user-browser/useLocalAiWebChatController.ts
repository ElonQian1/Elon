import { useEffect, useMemo, useRef, useState } from 'react'
import {
  controlLocalAiWebSession,
  getCachedLocalAiWebSessionState,
  getLocalAiWebSessionState,
  isLocalAiConversationSnapshot,
  isLocalAiMessageSnapshot,
  localAiBrowserErrorMessage,
  openLocalAiCachedConversation,
  openLocalAiWebSession,
  requestLocalAiWebSnapshot,
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
  beginOptimisticLocalAiSend,
  mergeOptimisticLocalAiMessages,
  pendingLocalAiSendObserved,
  type PendingLocalAiSend,
} from './localAiOptimisticSend'
import { requestOfficialAiTab, requestReturnToAiChat } from './internalBrowserApi'
import {
  googleNewConversationNeedsReload,
  localAiNewConversationNativeReady,
  selectLocalAiNewConversationPath,
} from './localAiNewConversation'
import { localAiComposerAvailability } from './localAiComposerAvailability'

export default function useLocalAiWebChatController(
  provider: LocalAiWebProvider | undefined,
  ownerKey: string,
  clientState: LocalAiClientState = 'ready',
) {
  const providerId = provider?.id ?? ''
  const providerDisplayName = provider?.displayName ?? ''
  const requestedSessionIdentity = providerId && ownerKey ? `${providerId}:${ownerKey}` : ''
  const [sessionEntry, setSessionEntry] = useState<{
    identity: string
    state: LocalAiWebSessionState | null
  }>(() => ({
    identity: requestedSessionIdentity,
    state: provider && ownerKey ? getCachedLocalAiWebSessionState(provider.id, ownerKey) : null,
  }))
  const [draft, setDraft] = useState('')
  const [draftTouched, setDraftTouched] = useState(false)
  const [pendingSends, setPendingSends] = useState<PendingLocalAiSend[]>([])
  const [queuedSend, setQueuedSend] = useState<QueuedLocalAiSend | null>(null)
  const [busyAction, setBusyAction] = useState('')
  const [message, setMessage] = useState('')
  const [newConversationRecoveryStartedAtMs, setNewConversationRecoveryStartedAtMs] = useState(0)
  const autoStartKey = useRef('')
  const backgroundReconnectAttempts = useRef(0)
  const backgroundReconnectInFlight = useRef(false)
  const responseRefreshGeneration = useRef(0)
  const responseRefreshTimer = useRef(0)
  const expectedResponsePrompt = useRef('')
  const optimisticSendSequence = useRef(0)
  const queuedSendDispatching = useRef(false)
  const queuedSendRef = useRef<QueuedLocalAiSend | null>(null)
  const newConversationBaselineId = useRef('')
  const draftRef = useRef('')
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
  const snapshot = newConversationRecoveryStartedAtMs ? null : liveSnapshot
  const visibleMessages = useMemo(
    () => mergeOptimisticLocalAiMessages(snapshot?.messages ?? [], pendingSends),
    [pendingSends, snapshot?.messages],
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
  const userState = useMemo(
    () => deriveLocalAiUserState(clientState, provider, visibleSessionState, snapshot),
    [clientState, provider, snapshot, visibleSessionState],
  )
  const composerAvailability = localAiComposerAvailability({
    clientReady: clientState === 'ready',
    providerAvailable: Boolean(provider && ownerKey),
    sendSupported: Boolean(provider?.adapterActions.includes('send_prompt')),
    directSendReady: userState.canSend,
    newConversationRecoveryActive: Boolean(newConversationRecoveryStartedAtMs),
    queuedSendActive: Boolean(queuedSend),
    busyAction,
  })

  function setSessionState(state: LocalAiWebSessionState | null) {
    setSessionEntry({ identity: requestedSessionIdentity, state })
  }

  const refreshSessionState = useLocalAiSessionPolling({
    enabled: Boolean(providerId && ownerKey),
    providerId,
    ownerKey,
    state: visibleSessionState,
    onState: setSessionState,
  })

  useEffect(() => {
    setSessionState(providerId && ownerKey
      ? getCachedLocalAiWebSessionState(providerId, ownerKey)
      : null)
    setDraft('')
    draftRef.current = ''
    setDraftTouched(false)
    setPendingSends([])
    setQueuedSend(null)
    queuedSendRef.current = null
    queuedSendDispatching.current = false
    newConversationBaselineId.current = ''
    setBusyAction('')
    setMessage('')
    setNewConversationRecoveryStartedAtMs(0)
    autoStartKey.current = ''
    backgroundReconnectAttempts.current = 0
    backgroundReconnectInFlight.current = false
    cancelResponseRefresh()
  }, [ownerKey, providerId, requestedSessionIdentity])

  useEffect(() => () => cancelResponseRefresh(), [])

  useEffect(() => {
    if (!newConversationRecoveryStartedAtMs) return
    const nativeReady = localAiNewConversationNativeReady(
      visibleSessionState,
      liveSnapshot,
      newConversationRecoveryStartedAtMs,
      newConversationBaselineId.current,
    )
    if (nativeReady && !queuedSend) {
      newConversationBaselineId.current = ''
      setNewConversationRecoveryStartedAtMs(0)
      return
    }
    if (nativeReady && queuedSend && !busyAction && !queuedSendDispatching.current) {
      queuedSendDispatching.current = true
      newConversationBaselineId.current = ''
      setNewConversationRecoveryStartedAtMs(0)
      setQueuedSend(null)
      queuedSendRef.current = null
      void dispatchPreparedPrompt(queuedSend)
        .finally(() => { queuedSendDispatching.current = false })
      return
    }
    // 五个条件里任何一个（尤其是官网迟迟不给可信实时快照）迟迟凑不齐时，不能让
    // 输入框永远显示空白；排队消息只在新会话绑定成功后发送，超时则安全还原草稿。
    if (Date.now() - newConversationRecoveryStartedAtMs < NEW_CONVERSATION_RECOVERY_TIMEOUT_MS) return
    newConversationBaselineId.current = ''
    setNewConversationRecoveryStartedAtMs(0)
    if (queuedSend) {
      restoreQueuedSend(queuedSend)
      setQueuedSend(null)
      queuedSendRef.current = null
      setMessage('新会话后台连接超时，消息没有误发；草稿已保留，可显示官方页确认后重试。')
    } else {
      setMessage('新会话后台连接超时；输入仍可继续编辑，如页面显示旧内容可打开官方页确认。')
    }
  }, [busyAction, liveSnapshot, newConversationRecoveryStartedAtMs, queuedSend, visibleSessionState])

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

  useEffect(() => {
    if (!providerId || !ownerKey) return
    const key = requestedSessionIdentity
    if (autoStartKey.current === key) return
    autoStartKey.current = key
    let active = true
    setMessage(`正在后台连接 ${providerDisplayName}；官网允许时可直接使用访客模式。`)
    void openLocalAiWebSession(providerId, ownerKey, { showWindow: false })
      .then(async () => {
        try {
          const next = await getLocalAiWebSessionState(providerId, ownerKey)
          if (active) setSessionState(next)
        } catch {
          // 后续共享状态同步会恢复，不重复创建窗口。
        }
        if (active) setMessage(`${providerDisplayName} 已在本机后台连接；登录是历史与增强能力的可选项。`)
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
      draftRef.current = nextDraft
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
    const expected = normalizePrompt(expectedResponsePrompt.current)
    if (!expected || !snapshot || snapshot.streaming) return
    const messages = snapshot.messages
    const userIndex = lastMatchingUserIndex(messages, expected)
    if (userIndex < 0) return
    if (messages.slice(userIndex + 1).some((item) => item.role === 'assistant' && item.state !== 'streaming')) {
      cancelResponseRefresh()
    }
  }, [snapshot])

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

  async function openCachedConversation(conversationId: string) {
    if (!provider || !ownerKey || busyAction) return
    setBusyAction('open_cached_conversation')
    setPendingSends([])
    cancelResponseRefresh()
    setMessage('正在从本机缓存恢复会话，并在后台连接官方上下文…')
    try {
      setSessionState(await openLocalAiCachedConversation(provider.id, ownerKey, conversationId))
      setMessage('已立即恢复本机会话缓存；官方页面正在后台同步最新内容。')
    } catch (error) {
      setMessage(localAiBrowserErrorMessage(error))
    } finally {
      setBusyAction('')
    }
  }

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
      queuedSendRef.current = queued
      setQueuedSend(queued)
      setMessage('消息已保存在本机新会话队列；官网在后台完成绑定后会自动发送。')
      return visibleSessionState
    }
    if (busyAction) return null
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
    draftRef.current = ''
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
      setSessionState(next)
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
    newConversationBaselineId.current = visibleSessionState?.activeConversationId ?? ''
    setNewConversationRecoveryStartedAtMs(Date.now())
    setPendingSends([])
    setQueuedSend(null)
    queuedSendRef.current = null
    queuedSendDispatching.current = false
    cancelResponseRefresh()
    draftRef.current = ''
    setDraft('')
    setDraftTouched(false)
    setMessage(`已在本机进入 ${provider?.displayName ?? '网页 AI'} 新会话；官网正在后台同步。`)
    return previousDraft
  }

  async function startNewConversation(): Promise<LocalAiWebSessionState | null> {
    if (!provider || !ownerKey) return null
    const path = selectLocalAiNewConversationPath(provider.id, visibleSessionState, snapshot)
    const previousDraft = beginLocalNewConversation()
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
            setSessionState(next)
            const result = next.commandResult
            if (result?.action !== 'new_conversation' || result.ok) {
              setMessage(`已进入 ${provider.displayName} 新会话；可以立即输入，官网上下文正在后台确认。`)
              return next
            }
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

  async function openNewConversationHome(
    previousDraft: string,
    recoveryMessage = '',
  ): Promise<LocalAiWebSessionState | null> {
    if (!provider || !ownerKey) return null
    try {
      const next = await controlLocalAiWebSession(provider.id, ownerKey, 'home')
      setSessionState(next)
      setMessage([
        recoveryMessage,
        `已进入 ${provider.displayName} 新会话；可以立即输入，提前发送的消息会在绑定确认后自动提交。`,
      ].filter(Boolean).join(' '))
      return next
    } catch (error) {
      failLocalNewConversation(previousDraft, error)
      return null
    }
  }

  function failLocalNewConversation(previousDraft: string, error: unknown) {
    newConversationBaselineId.current = ''
    setNewConversationRecoveryStartedAtMs(0)
    const queued = queuedSendRef.current
    if (queued) {
      restoreQueuedSend(queued)
      queuedSendRef.current = null
      setQueuedSend(null)
    } else if (!draftRef.current && previousDraft) {
      draftRef.current = previousDraft
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
    if (!draftRef.current) {
      draftRef.current = pending.prompt
      setDraft(pending.prompt)
      setDraftTouched(true)
    }
  }

  function restoreQueuedSend(queued: QueuedLocalAiSend) {
    rollbackPendingSend(queued.pending)
  }

  function startResponseRefresh(prompt: string) {
    cancelResponseRefresh()
    if (!provider || !ownerKey || !normalizePrompt(prompt)) return
    expectedResponsePrompt.current = prompt
    const generation = responseRefreshGeneration.current
    let delayIndex = 0
    const request = () => {
      if (generation !== responseRefreshGeneration.current || !provider || !ownerKey) return
      void requestLocalAiWebSnapshot(provider.id, ownerKey)
        .then(refreshSessionState, () => {})
      const delay = RESPONSE_REFRESH_DELAYS_MS[delayIndex++]
      if (delay !== undefined) responseRefreshTimer.current = window.setTimeout(request, delay)
    }
    responseRefreshTimer.current = window.setTimeout(request, RESPONSE_REFRESH_DELAYS_MS[delayIndex++] ?? 400)
  }

  function cancelResponseRefresh() {
    responseRefreshGeneration.current += 1
    window.clearTimeout(responseRefreshTimer.current)
    responseRefreshTimer.current = 0
    expectedResponsePrompt.current = ''
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
      const requestId = await runLocalAiWebAdapterCommand(provider.id, ownerKey, listAction)
      await new Promise((resolve) => window.setTimeout(resolve, 180))
      let next = await getLocalAiWebSessionState(provider.id, ownerKey)
      // 首轮 180ms 内常常还没等到 list 回执；second 命令是 collect，回执的 action 也是
      // collect，必须换成它自己的 action/requestId 去等，否则会一直等一个不会出现的匹配。
      let resultAction: LocalAiAdapterAction = listAction
      let resultRequestId = requestId
      if (next.commandResult?.requestId !== requestId) {
        resultAction = collectAction
        resultRequestId = await runLocalAiWebAdapterCommand(provider.id, ownerKey, collectAction)
        next = await waitForLocalAiAdapterResult(provider.id, ownerKey, resultAction, resultRequestId) ?? next
      }
      setSessionState(next)
      const result = next.commandResult
      if (result?.action !== resultAction || result?.requestId !== resultRequestId) {
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

  return {
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
      draftRef.current = value
      setDraft(value)
      setDraftTouched(true)
    },
    busyAction,
    message,
    canEditDraft: composerAvailability.canEdit,
    canSubmitDraft: composerAvailability.canSubmit,
    queuedSendActive: Boolean(queuedSend),
    newConversationRecoveryActive: Boolean(newConversationRecoveryStartedAtMs),
    openOfficial,
    control,
    openCachedConversation,
    run,
    refreshComposerControls,
    refreshFeatureNavigation,
  }
}

interface QueuedLocalAiSend {
  prompt: string
  expectedDraft: string
  pending: PendingLocalAiSend
  sessionIdentity: string
}

const RESPONSE_REFRESH_DELAYS_MS = [400, 800, 1_500, 2_500, 4_000, 6_000, 8_000, 10_000] as const
// 后台会话轮询本身已经按 15 秒节奏检查一次是否仍然 closed，这里只再限一个总次数上限，
// 避免真正打不开时无休止地重建 WebView2。
const BACKGROUND_RECONNECT_MAX_ATTEMPTS = 3
// 新建会话后等待官网给出可信实时快照的上限；超过就不再无限期把输入框清空。
const GOOGLE_NEW_CONVERSATION_RELOAD_DELAY_MS = 2_000
const NEW_CONVERSATION_RECOVERY_TIMEOUT_MS = 24_000

function normalizePrompt(value: string): string {
  return value.trim().replace(/\s+/g, ' ')
}

function visibleMessageText(message: { content: Array<{ type: string; text?: string }> }): string {
  return message.content
    .filter((part) => part.type === 'text' || part.type === 'markdown')
    .map((part) => part.text ?? '')
    .join('\n')
}

function lastMatchingUserIndex(
  messages: Array<{ role: string; content: Array<{ type: string; text?: string }> }>,
  expected: string,
): number {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index]
    if (message.role === 'user' && normalizePrompt(visibleMessageText(message)) === expected) return index
  }
  return -1
}
