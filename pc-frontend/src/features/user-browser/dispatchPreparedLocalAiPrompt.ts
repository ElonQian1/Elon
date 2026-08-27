import {
  controlLocalAiWebSession,
  localAiBrowserErrorMessage,
  runLocalAiWebAdapterCommand,
  waitForLocalAiAdapterResult,
  type LocalAiWebProvider,
  type LocalAiWebSessionState,
} from './localAiBrowserApi'
import { requestReturnToAiChat } from './internalBrowserApi'
import type { QueuedLocalAiSend } from './localAiWebChatControllerConfig'

interface DispatchPreparedLocalAiPromptOptions {
  provider: LocalAiWebProvider | undefined
  ownerKey: string
  requestedSessionIdentity: string
  prepared: QueuedLocalAiSend
  restore: (prepared: QueuedLocalAiSend) => void
  onBusyAction: (action: string) => void
  onMessage: (message: string) => void
  onState: (state: LocalAiWebSessionState) => void
  onResponseRefresh: (prompt: string, baselineMatchingUserCount: number) => void
}

export async function dispatchPreparedLocalAiPrompt({
  provider,
  ownerKey,
  requestedSessionIdentity,
  prepared,
  restore,
  onBusyAction,
  onMessage,
  onState,
  onResponseRefresh,
}: DispatchPreparedLocalAiPromptOptions): Promise<LocalAiWebSessionState | null> {
  if (!provider || !ownerKey || prepared.sessionIdentity !== requestedSessionIdentity) {
    restore(prepared)
    return null
  }
  onBusyAction('send_prompt')
  onMessage('')
  try {
    const foregroundRequest = {
      providerId: provider.id,
      providerName: provider.displayName,
      ownerKey,
    }
    // The native chat surface owns focus. Park the official WebView before and
    // after the matching receipt because the page may navigate late after send.
    requestReturnToAiChat(foregroundRequest)
    onState(await controlLocalAiWebSession(provider.id, ownerKey, 'background'))
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
      restore(prepared)
      onMessage('没有收到当前发送的匹配回执；消息没有标记为成功，草稿已保留。')
      return null
    }
    requestReturnToAiChat(foregroundRequest)
    let foregroundState = next
    try {
      foregroundState = await controlLocalAiWebSession(provider.id, ownerKey, 'background')
    } catch {
      // Response polling continues to reassert the same bounded foreground intent.
    }
    onState(foregroundState)
    const result = next.commandResult
    if (result?.action === 'send_prompt' && !result.ok) {
      restore(prepared)
      onMessage(result.detail || '官方网页没有完成发送，草稿已保留；可显示官方窗口后重试。')
    } else {
      onMessage(result?.detail || '消息已交给官方网页发送；正在一龙聊天界面同步回复。')
      onResponseRefresh(prepared.prompt, prepared.pending.baselineMatchingUserCount)
    }
    return next
  } catch (error) {
    restore(prepared)
    onMessage(localAiBrowserErrorMessage(error))
    return null
  } finally {
    onBusyAction('')
  }
}
