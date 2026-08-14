import { useEffect, useMemo, useRef, useState } from 'react'
import {
  controlLocalAiWebSession,
  getLocalAiWebSessionState,
  isLocalAiConversationSnapshot,
  isLocalAiMessageSnapshot,
  localAiBrowserErrorMessage,
  openLocalAiWebSession,
  runLocalAiWebAdapterCommand,
  waitForLocalAiAdapterResult,
  type LocalAiAdapterAction,
  type LocalAiBrowserControlAction,
  type LocalAiWebProvider,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'

export default function useLocalAiWebChatController(
  provider: LocalAiWebProvider | undefined,
  ownerKey: string,
) {
  const [sessionState, setSessionState] = useState<LocalAiWebSessionState | null>(null)
  const [draft, setDraft] = useState('')
  const [draftTouched, setDraftTouched] = useState(false)
  const [busyAction, setBusyAction] = useState('')
  const [message, setMessage] = useState('')
  const autoStartKey = useRef('')
  const snapshot = useMemo(
    () => isLocalAiMessageSnapshot(sessionState?.semanticEvent) ? sessionState.semanticEvent : null,
    [sessionState?.semanticEvent],
  )
  const navigationSnapshot = useMemo(
    () => isLocalAiConversationSnapshot(sessionState?.navigationEvent)
      ? sessionState.navigationEvent
      : null,
    [sessionState?.navigationEvent],
  )
  const sessionOpen = Boolean(sessionState && sessionState.windowStatus !== 'closed')

  useEffect(() => {
    setSessionState(null)
    setDraft('')
    setDraftTouched(false)
    setBusyAction('')
    setMessage('')
    autoStartKey.current = ''
  }, [provider?.id, ownerKey])

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

  async function run(action: LocalAiAdapterAction, value?: string, expectedDraft?: string) {
    if (!provider || !ownerKey || busyAction) return
    setBusyAction(action)
    setMessage('')
    try {
      await runLocalAiWebAdapterCommand(provider.id, ownerKey, action, value, expectedDraft)
      const next = await waitForLocalAiAdapterResult(provider.id, ownerKey, action)
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
    } catch (error) {
      setMessage(localAiBrowserErrorMessage(error))
    } finally {
      setBusyAction('')
    }
  }

  return {
    sessionState,
    snapshot,
    navigationSnapshot,
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
    run,
  }
}
