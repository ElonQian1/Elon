import { useEffect, useMemo, useState } from 'react'
import type { AiMessage, AiSource } from '../ai/AiChatMessageRow'
import { hasVisibleAiMessageContent, shouldKeepAiWebMessage } from '../ai/aiMessageVisibility'
import type { AiStructuredPart } from '../ai/AiStructuredContent'
import type { AiHomeMode } from '../ai/AiHomeModeSwitch'
import {
  DEFAULT_LOCAL_AI_PROVIDER_ID,
  LOCAL_AI_PROVIDER_FALLBACKS,
} from './localAiWebProviders'
import useLocalAiBrowserCapability from './useLocalAiBrowserCapability'
import useLocalAiWebChatController from './useLocalAiWebChatController'
import { localAiHistoryWindow } from './localAiHistoryWindow'
import { shouldRenderNativeStructuredPart } from './localAiStructuredPartPolicy'
import type { LocalAiStructuredContentPart } from './localAiBrowserProtocol'
import { isYilongRichContent } from './richContentProtocol'
import { localAiRendererCompatibility } from './localAiRendererCompatibility'
import { localAiAssistantExtractionIncomplete } from './localAiAssistantContentQuality'
import useLocalAiProviderActivity from './useLocalAiProviderActivity'
import {
  localAiStreamingStatus,
  localAiStreamingTarget,
} from './localAiStreamingPresentation'

const PROVIDER_STORAGE_KEY = 'elon.pc.aiChatProvider'

export default function useAiWebChatBackend(mode: AiHomeMode, ownerKey: string) {
  const capability = useLocalAiBrowserCapability()
  const [providerId, setProviderId] = useState(readProviderPreference)
  const providers = useMemo(() => (
    capability.providers.length
      ? capability.providers
      : Object.values(LOCAL_AI_PROVIDER_FALLBACKS)
  ), [capability.providers])
  const provider = providers.find((item) => item.id === providerId) || providers[0]
  const controller = useLocalAiWebChatController(
    mode === 'chat' && capability.state === 'ready' ? provider : undefined,
    mode === 'chat' ? ownerKey : '',
    capability.state,
  )
  const streamingTarget = useMemo(() => localAiStreamingTarget(
    controller.visibleMessages,
    controller.snapshot?.streaming === true,
  ), [controller.snapshot?.streaming, controller.visibleMessages])

  useEffect(() => {
    if (!provider || provider.id === providerId) return
    setProviderId(provider.id)
  }, [provider, providerId])

  const messages = useMemo<AiMessage[]>(() => {
    const compatibilityByMessage = new Map<string, ReturnType<typeof localAiRendererCompatibility>>()
    for (const item of controller.visibleMessages) {
      if (item.role !== 'assistant' || item.state !== 'completed') continue
      compatibilityByMessage.set(item.id, localAiRendererCompatibility(
        item.content.filter((part): part is LocalAiStructuredContentPart => (
          !['text', 'markdown', 'citation'].includes(part.type)
        )),
        localAiAssistantExtractionIncomplete(item),
      ))
    }
    const latestAffectedAssistantId = [...controller.visibleMessages]
      .reverse()
      .find((item) => compatibilityByMessage.get(item.id))?.id
    const mapped = controller.visibleMessages.flatMap((item): AiMessage[] => {
      const sources = item.content
        .filter((part): part is Extract<typeof part, { type: 'citation' }> => part.type === 'citation' && Boolean(part.url))
        .map<AiSource>((part) => ({
          title: part.text || publicHost(part.url!),
          url: part.url!,
          icon_url: part.iconUrl,
          marker_text: part.markerText,
          citation_id: part.citationId,
          group_size: part.groupSize,
        }))
      const content = item.content
        .filter((part): part is Extract<typeof part, { type: 'text' | 'markdown' }> => (
          part.type === 'text' || part.type === 'markdown'
        ))
        .map((part) => part.text)
        .filter(Boolean)
        .join('\n\n')
      const contentFormat = item.content.some((part) => part.type === 'markdown')
        ? 'markdown' as const
        : 'plain' as const
      const hasVisibleContent = hasVisibleAiMessageContent(content)
      const structuredParts = item.content
        .filter((part): part is LocalAiStructuredContentPart => (
          !['text', 'markdown', 'citation'].includes(part.type)
        ))
        .filter(shouldRenderNativeStructuredPart)
        .map<AiStructuredPart>((part) => {
          const richContent = 'richContent' in part && isYilongRichContent(part.richContent)
            ? part.richContent
            : undefined
          return {
            type: part.type as AiStructuredPart['type'],
            label: part.text,
            kind: 'kind' in part ? part.kind : undefined,
            language: 'language' in part ? part.language : undefined,
            mediaType: 'mediaType' in part ? part.mediaType : undefined,
            targetHost: 'targetHost' in part ? part.targetHost : undefined,
            lineCount: 'lineCount' in part ? part.lineCount : undefined,
            rowCount: 'rowCount' in part ? part.rowCount : undefined,
            columnCount: 'columnCount' in part ? part.columnCount : undefined,
            richContent,
          }
        })
        .filter((part) => part.type !== 'rich_card' || Boolean(part.richContent))
      const effectiveState = item.id === streamingTarget?.messageId ? 'streaming' : item.state
      if (!shouldKeepAiWebMessage({
        content,
        state: effectiveState,
        sourceCount: sources.length,
        structuredCount: structuredParts.length,
      })) {
        return []
      }
      return [{
        id: `web:${provider?.id || 'ai'}:${item.id}`,
        role: item.role,
        content: hasVisibleContent ? content : '',
        content_format: contentFormat,
        assistant_provider_id: provider?.id,
        tool_used: item.role === 'assistant' && provider?.id === 'google-ai-mode' ? 'web_search' : null,
        sources,
        structured_parts: structuredParts,
        renderer_compatibility: item.id === latestAffectedAssistantId
          ? compatibilityByMessage.get(item.id)
          : undefined,
      }]
    })
    if (streamingTarget?.synthetic) {
      mapped.push({
        id: `web:${provider?.id || 'ai'}:${streamingTarget.messageId}`,
        role: 'assistant',
        content: '',
        content_format: 'plain',
        assistant_provider_id: provider?.id,
        tool_used: null,
        sources: [],
        structured_parts: [],
      })
    }
    return mapped
  }, [controller.visibleMessages, provider?.id, streamingTarget])
  const ready = capability.state === 'ready' && Boolean(ownerKey && provider)
  const providerActivities = useLocalAiProviderActivity({
    enabled: mode === 'chat' && ready,
    providers,
    selectedProviderId: provider?.id,
    ownerKey,
    selectedState: controller.sessionState,
  })
  const canEdit = capability.state === 'ready' && Boolean(provider) && controller.canEditDraft
  const canCompose = ready && controller.canSubmitDraft
  const contextTurnCount = (controller.snapshot?.messages ?? [])
    .filter((item) => item.role === 'user').length
  const contextReady = controller.sessionState?.contextReady !== false
  const contextStatus = controller.sessionState?.contextStatus
  const historyWindow = useMemo(
    () => localAiHistoryWindow(controller.snapshot),
    [controller.snapshot],
  )
  const officialRequest = useMemo(() => (
    provider && ownerKey
      ? { providerId: provider.id, providerName: provider.displayName, ownerKey }
      : undefined
  ), [ownerKey, provider])

  function selectProvider(id: string) {
    setProviderId(id)
    try { window.localStorage.setItem(PROVIDER_STORAGE_KEY, id) } catch {}
  }

  return {
    capability,
    providers,
    providerActivities,
    provider,
    selectProvider,
    controller,
    officialRequest,
    userState: controller.userState,
    messages,
    ready,
    canEdit,
    canCompose,
    title: controller.snapshot?.title || '新对话',
    streamingMessageId: streamingTarget
      ? `web:${provider?.id || 'ai'}:${streamingTarget.messageId}`
      : null,
    streamingStatus: localAiStreamingStatus({
      officialStatus: controller.snapshot?.streamingStatus,
      pendingSlow: controller.pendingResponseSlow,
      providerName: provider?.displayName,
    }),
    contextReady,
    contextStatus,
    contextTurnCount,
    historyWindow,
    contextSummary: controller.newConversationRecoveryActive
      ? controller.queuedSendActive
        ? '消息已保存在本机新会话队列；官网绑定确认后会自动发送。'
        : '新会话已在本机就绪，可以立即输入；官网页面正在后台异步同步。'
      : !contextReady
      ? contextStatus === 'cached'
        ? '已立即显示本地缓存；官方网页尚未恢复到对应会话，发送已暂停。'
        : contextStatus === 'unbound'
          ? '缓存内容与当前官方页面不属于同一会话，发送已暂停。'
          : '正在恢复官方会话；目标上下文确认前已暂停发送。'
      : contextTurnCount > 0
        ? `已确认当前官方会话绑定，并同步 ${contextTurnCount} 轮可见上下文。`
        : '新会话与当前官方页面已绑定，第一条消息会进入这个会话。',
    accessMode: canCompose
      ? controller.snapshot?.authenticated ? 'account' as const : 'guest' as const
      : 'unavailable' as const,
    status: controller.userState.title,
    message: controller.sessionState?.lastError
      || controller.message
      || capability.message
      || (controller.userState.degraded ? controller.userState.detail : ''),
    modelButtonCopy: {
      source: '网页 AI',
      detail: provider?.displayName || '选择厂商',
      title: `聊天来源：${provider?.displayName || '网页 AI'}；点击切换或管理官方会话`,
    },
  }
}

export type AiWebChatBackend = ReturnType<typeof useAiWebChatBackend>

function readProviderPreference() {
  try {
    const stored = window.localStorage.getItem(PROVIDER_STORAGE_KEY) || ''
    if (stored in LOCAL_AI_PROVIDER_FALLBACKS) return stored
  } catch {}
  return DEFAULT_LOCAL_AI_PROVIDER_ID
}

function publicHost(url: string) {
  try { return new URL(url).hostname }
  catch { return url }
}
