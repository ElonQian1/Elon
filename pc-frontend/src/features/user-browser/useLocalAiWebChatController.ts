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

export default function useLocalAiWebChatController(
  provider: LocalAiWebProvider | undefined,
  ownerKey: string,
  clientState: LocalAiClientState = 'ready',
) {
  const requestedSessionIdentity = provider && ownerKey ? `${provider.id}:${ownerKey}` : ''
  const [sessionEntry, setSessionEntry] = useState<{
    identity: string
    state: LocalAiWebSessionState | null
  }>(() => ({
    identity: requestedSessionIdentity,
    state: provider && ownerKey ? getCachedLocalAiWebSessionState(provider.id, ownerKey) : null,
  }))
  const [draft, setDraft] = useState('')
  const [draftTouched, setDraftTouched] = useState(false)
  const [busyAction, setBusyAction] = useState('')
  const [message, setMessage] = useState('')
  const autoStartKey = useRef('')
  const responseRefreshGeneration = useRef(0)
  const responseRefreshTimer = useRef(0)
  const expectedResponsePrompt = useRef('')
  const visibleSessionState = provider && ownerKey
    ? sessionEntry.identity === requestedSessionIdentity
      && sessionEntry.state?.providerId === provider.id
      ? sessionEntry.state
      : getCachedLocalAiWebSessionState(provider.id, ownerKey)
    : null
  const snapshot = useMemo(
    () => isLocalAiMessageSnapshot(visibleSessionState?.semanticEvent)
      ? visibleSessionState.semanticEvent
      : null,
    [visibleSessionState?.semanticEvent],
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

  function setSessionState(state: LocalAiWebSessionState | null) {
    setSessionEntry({ identity: requestedSessionIdentity, state })
  }

  useEffect(() => {
    setSessionState(provider && ownerKey
      ? getCachedLocalAiWebSessionState(provider.id, ownerKey)
      : null)
    setDraft('')
    setDraftTouched(false)
    setBusyAction('')
    setMessage('')
    autoStartKey.current = ''
    cancelResponseRefresh()
  }, [ownerKey, provider])

  useEffect(() => () => cancelResponseRefresh(), [])

  useEffect(() => {
    if (!provider || !ownerKey) return
    const key = `${provider.id}:${ownerKey}`
    if (autoStartKey.current === key) return
    autoStartKey.current = key
    let active = true
    setBusyAction('prepare_guest_session')
    setMessage(`正在后台连接 ${provider.displayName}；官网允许时可直接使用访客模式。`)
    void openLocalAiWebSession(provider.id, ownerKey, { showWindow: false })
      .then(async () => {
        try {
          const next = await getLocalAiWebSessionState(provider.id, ownerKey)
          if (active) setSessionState(next)
        } catch {
          // 后续有界轮询会恢复状态，不重复创建窗口。
        }
        if (active) setMessage(`${provider.displayName} 已在本机后台连接；登录是历史与增强能力的可选项。`)
      })
      .catch((error) => {
        if (active) setMessage(localAiBrowserErrorMessage(error))
      })
      .finally(() => {
        if (active) setBusyAction('')
      })
    return () => { active = false }
  }, [ownerKey, provider])

  useEffect(() => {
    if (!provider || !ownerKey) return
    let active = true
    let timer = 0
    const poll = async () => {
      try {
        const next = await getLocalAiWebSessionState(provider.id, ownerKey)
        if (active) setSessionState(next)
      } catch (error) {
        if (active) setMessage(localAiBrowserErrorMessage(error))
      } finally {
        if (active) timer = window.setTimeout(() => void poll(), 1_500)
      }
    }
    void poll()
    return () => {
      active = false
      window.clearTimeout(timer)
    }
  }, [ownerKey, provider])

  useEffect(() => {
    if (!draftTouched) setDraft(snapshot?.draft ?? '')
  }, [draftTouched, snapshot?.draft])

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
      await openLocalAiWebSession(provider.id, ownerKey)
      try {
        setSessionState(await getLocalAiWebSessionState(provider.id, ownerKey))
      } catch {
        // The bounded poll recovers a state refresh without reopening the window.
      }
      setMessage(`已显示 ${provider.displayName} 官方窗口；可检查访客能力，也可按官网要求登录或完成人机验证。`)
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
    if (!provider || !ownerKey || busyAction) return null
    if (!provider.adapterActions.includes(action)) {
      setMessage(`${provider.displayName} 当前不支持这个原生动作；可以显示官方窗口继续使用。`)
      return null
    }
    setBusyAction(action)
    if (['new_conversation', 'open_conversation', 'open_project'].includes(action)) {
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
      } else if (action === 'send_prompt') {
        setDraft('')
        setDraftTouched(false)
        setMessage(result?.detail || '消息已交给官方网页发送。')
        startResponseRefresh(value ?? '')
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

  function startResponseRefresh(prompt: string) {
    cancelResponseRefresh()
    if (!provider || !ownerKey || !normalizePrompt(prompt)) return
    expectedResponsePrompt.current = prompt
    const generation = responseRefreshGeneration.current
    let delayIndex = 0
    const request = () => {
      if (generation !== responseRefreshGeneration.current || !provider || !ownerKey) return
      void requestLocalAiWebSnapshot(provider.id, ownerKey).catch(() => {})
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
      if (next.commandResult?.requestId !== requestId) {
        await runLocalAiWebAdapterCommand(provider.id, ownerKey, collectAction)
        next = await waitForLocalAiAdapterResult(provider.id, ownerKey, listAction, requestId) ?? next
      }
      setSessionState(next)
      const result = next.commandResult
      if (result?.requestId !== requestId) {
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
    openOfficial,
    control,
    openCachedConversation,
    run,
    refreshComposerControls,
    refreshFeatureNavigation,
  }
}

const RESPONSE_REFRESH_DELAYS_MS = [400, 800, 1_500, 2_500, 4_000, 6_000, 8_000, 10_000] as const

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
