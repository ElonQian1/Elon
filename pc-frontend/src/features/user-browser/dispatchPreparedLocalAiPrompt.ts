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
import { localAiSendReceiptDecision } from './localAiSendReceiptPolicy'

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
  isCurrent: () => boolean
  isGenerationCurrent: () => boolean
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
  isCurrent,
  isGenerationCurrent,
}: DispatchPreparedLocalAiPromptOptions): Promise<LocalAiWebSessionState | null> {
  if (!isCurrent()) return null
  if (!provider || !ownerKey || prepared.sessionIdentity !== requestedSessionIdentity) {
    restore(prepared)
    return null
  }
  onBusyAction('send_prompt')
  onMessage('')
  let commandQueued = false
  let requestId = ''
  const reconcileUncertainSend = () => {
    if (!isCurrent()) return
    onMessage('发送结果暂未确认。为避免重复发送，一龙不会自动重放；正在从官方会话对账，可打开官方页核对。')
    onResponseRefresh(prepared.prompt, prepared.pending.baselineMatchingUserCount)
  }
  try {
    const foregroundRequest = {
      providerId: provider.id,
      providerName: provider.displayName,
      ownerKey,
    }
    // The native chat surface owns focus. Park the official WebView before and
    // after the matching receipt because the page may navigate late after send.
    requestReturnToAiChat(foregroundRequest)
    const parked = await controlLocalAiWebSession(provider.id, ownerKey, 'background')
    if (!isCurrent()) return null
    onState(parked)
    requestId = await runLocalAiWebAdapterCommand(
      provider.id,
      ownerKey,
      'send_prompt',
      prepared.prompt,
      prepared.expectedDraft,
    )
    if (!isCurrent()) return null
    commandQueued = true
    const next = await waitForLocalAiAdapterResult(
      provider.id,
      ownerKey,
      'send_prompt',
      requestId,
    )
    if (!isCurrent()) return null
    const decision = localAiSendReceiptDecision({
      commandQueued,
      requestId,
      receipt: next?.commandResult,
    })
    if (decision === 'reconcile' || !next) {
      reconcileUncertainSend()
      return null
    }
    requestReturnToAiChat(foregroundRequest)
    let foregroundState = next
    try {
      foregroundState = await controlLocalAiWebSession(provider.id, ownerKey, 'background')
    } catch {
      // Response polling continues to reassert the same bounded foreground intent.
    }
    if (!isCurrent()) return null
    onState(foregroundState)
    const result = next?.commandResult
    if (decision === 'rejected') {
      restore(prepared)
      onMessage(result?.detail || '官方网页明确拒绝了发送，草稿已恢复；可显示官方窗口后重试。')
    } else {
      onMessage(result?.detail || '消息已交给官方网页发送；正在一龙聊天界面同步回复。')
      onResponseRefresh(prepared.prompt, prepared.pending.baselineMatchingUserCount)
    }
    return next
  } catch (error) {
    if (!isCurrent()) return null
    const decision = localAiSendReceiptDecision({ commandQueued, requestId })
    if (decision === 'reconcile') {
      reconcileUncertainSend()
    } else {
      restore(prepared)
      onMessage(localAiBrowserErrorMessage(error))
    }
    return null
  } finally {
    // A provider/conversation boundary invalidates the old generation. Its late
    // finally block must not clear the busy state owned by a newer operation.
    if (isGenerationCurrent()) onBusyAction('')
  }
}
