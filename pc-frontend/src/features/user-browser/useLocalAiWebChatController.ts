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
  const [sessionState, setSessionState] = useState<LocalAiWebSessionState | null>(() => (
    provider && ownerKey ? getCachedLocalAiWebSessionState(provider.id, ownerKey) : null
  ))
  const [draft, setDraft] = useState('')
  const [draftTouched, setDraftTouched] = useState(false)
  const [busyAction, setBusyAction] = useState('')
  const [message, setMessage] = useState('')
  const autoStartKey = useRef('')
  const visibleSessionState = provider && ownerKey
    ? sessionState?.providerId === provider.id
      ? sessionState
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

  useEffect(() => {
    setSessionState(provider && ownerKey
      ? getCachedLocalAiWebSessionState(provider.id, ownerKey)
      : null)
    setDraft('')
    setDraftTouched(false)
    setBusyAction('')
    setMessage('')
    autoStartKey.current = ''
  }, [ownerKey, provider])

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
