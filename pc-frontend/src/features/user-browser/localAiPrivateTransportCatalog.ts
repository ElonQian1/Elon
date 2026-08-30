import type {
  LocalAiAdapterAction,
  LocalAiMessageSnapshot,
  LocalAiPrivateRichRecovery,
  LocalAiPrivateTransportHealth,
  LocalAiWebProvider,
} from './localAiBrowserApi'

export interface LocalAiPrivateTransportCapability {
  id: string
  label: string
  requestMode: string
  fallback: string
  runtimeEnabled: boolean
  activation: 'preset_then_background_verify'
}

export function localAiPrivateRichRecoveryStatusCopy(
  recovery: LocalAiPrivateRichRecovery | undefined,
): string | null {
  if (!recovery || recovery.version !== 1) return null
  const kinds = recovery.richKinds.length ? `（${recovery.richKinds.join(' / ')}）` : ''
  if (recovery.active) {
    const binding = recovery.turnBound
      ? '回答轮次已绑定'
      : recovery.conversationBound ? '会话已绑定，等待回答身份补齐' : '等待会话身份补齐'
    const reconciled = recovery.placeholderReconciled ? '；已用真实卡片替换官网占位' : ''
    return `富内容恢复已接纳 ${recovery.acceptedCount} 次${kinds}，${binding}${reconciled}。`
  }
  if (['stale_generation', 'route_mismatch', 'identity_mismatch', 'detached_incomplete'].includes(
    recovery.lastOutcome,
  )) {
    return `最近富内容结果已安全拒绝（${richRecoveryOutcomeCopy(recovery.lastOutcome)}）；未串入其他会话。`
  }
  if (recovery.lastOutcome === 'empty') {
    return '本轮私有响应尚未解析出可渲染富内容；继续使用正文和官网回退。'
  }
  return null
}

export function localAiPrivateStreamStatusCopy(
  provider: LocalAiWebProvider | undefined,
  snapshot: LocalAiMessageSnapshot | null | undefined,
): string | null {
  if (!provider || snapshot?.privateStreamObserved !== true) return null
  if (snapshot.privateStreamState === 'streaming') {
    return `${provider.displayName} 私有回复通道正在接收本轮内容；原生界面保持可用，官网页继续在后台生成。`
  }
  if (snapshot.privateStreamState === 'completed') {
    return `${provider.displayName} 私有回复完成信号已到达；正文与富内容已进入原生结算流程。`
  }
  return `${provider.displayName} 私有回复通道已验证；当前没有进行中的回答。`
}

function richRecoveryOutcomeCopy(outcome: LocalAiPrivateRichRecovery['lastOutcome']): string {
  if (outcome === 'stale_generation') return '旧发送代次'
  if (outcome === 'route_mismatch') return '页面会话不一致'
  if (outcome === 'identity_mismatch') return '回答身份不一致'
  if (outcome === 'detached_incomplete') return '离线恢复身份不完整'
  return '结构无效'
}

interface CapabilityDefinition {
  id: string
  label: string
  requestMode: string
  fallback: string
  requiredActions?: LocalAiAdapterAction[]
  androidParityIds?: readonly string[]
}

const SHARED_CONTINUITY: CapabilityDefinition = {
  id: 'win_web_ai_background_navigation_continuity_v1',
  label: '后台导航与宿主恢复连续性',
  requestMode: 'preserve_inflight_navigation_and_resume_adapter_snapshot',
  fallback: 'official_webview_bounded_recovery',
  androidParityIds: ['android_web_ai_background_navigation_continuity_v1'],
}

const SHARED_REFRESH_SINGLE_FLIGHT: CapabilityDefinition = {
  id: 'win_web_ai_snapshot_refresh_single_flight_v1',
  label: '官网快照单飞行刷新',
  requestMode: 'coalesced_official_snapshot_poll',
  fallback: 'official_webview_bounded_recovery',
  requiredActions: ['snapshot'],
}

const SHARED_SEND_COORDINATOR: CapabilityDefinition = {
  id: 'win_web_ai_unified_send_coordinator_v1',
  label: '单所有者发送、稳定回执与跨会话隔离',
  requestMode: 'stable_request_id_single_owner_generation_gated_official_page_transport',
  fallback: 'official_page_reconciliation_without_automatic_write_replay',
  requiredActions: ['send_prompt'],
  androidParityIds: ['android_web_ai_unified_send_coordinator_v1'],
}

const SHARED_PROVIDER_SESSION_PREWARM: CapabilityDefinition = {
  id: 'win_web_ai_provider_session_prewarm_v1',
  label: '厂商会话后台预热与切换复用',
  requestMode: 'delayed_single_flight_hidden_webview_warmup',
  fallback: 'selected_provider_foreground_resume',
  requiredActions: ['snapshot'],
}

const SHARED_CONVERSATION_BODY_CACHE: CapabilityDefinition = {
  id: 'win_web_ai_per_conversation_rich_snapshot_cache_v1',
  label: '独立会话正文与富内容缓存',
  requestMode: 'dpapi_per_conversation_cache_first',
  fallback: 'official_webview_navigation',
  requiredActions: ['open_conversation'],
}

const SHARED_NATIVE_STREAM_REFRESH: CapabilityDefinition = {
  id: 'win_web_ai_native_stream_refresh_v1',
  label: '私有流原生事件即时刷新',
  requestMode: 'coalesced_native_session_update_event',
  fallback: 'bounded_session_and_dom_watchdog',
}

const CATALOG: Record<string, CapabilityDefinition[]> = {
  chatgpt: [
    {
      id: 'win_chatgpt_private_conversation_project_directory_v1',
      label: '私有会话与项目目录观察',
      requestMode: 'passive_official_response_observer',
      fallback: 'official_dom_directory',
      requiredActions: ['list_conversations', 'open_conversation'],
      androidParityIds: ['android_chatgpt_private_conversation_project_directory_v1'],
    },
    {
      id: 'win_chatgpt_private_conversation_prefetch_v1',
      label: '当前会话同源预取',
      requestMode: 'authenticated_same_origin_get',
      fallback: 'official_webview_navigation',
      requiredActions: ['open_conversation'],
      androidParityIds: ['android_chatgpt_private_conversation_prefetch_v1'],
    },
    {
      id: 'win_chatgpt_guest_private_conversation_refresh_v1',
      label: '游客会话富内容补齐',
      requestMode: 'observed_same_origin_guest_get_with_bounded_endpoint_fallback',
      fallback: 'official_stream_and_dom_snapshot',
      requiredActions: ['open_conversation', 'snapshot'],
    },
    {
      id: 'win_chatgpt_private_send_dispatch_observer_v1',
      label: '发送请求被动确认',
      requestMode: 'passive_official_request_observer',
      fallback: 'official_dom_send_confirmation',
      requiredActions: ['send_prompt'],
      androidParityIds: ['android_chatgpt_private_send_dispatch_observer_v1'],
    },
    {
      id: 'win_chatgpt_same_origin_text_transaction_v1',
      label: '同源私有文本写事务与官网回退',
      requestMode: 'versioned_single_flight_same_origin_text_post_when_reusable',
      fallback: 'immediate_official_page_transaction_without_write_replay',
      requiredActions: ['send_prompt'],
      androidParityIds: ['android_chatgpt_same_origin_text_transaction_v1'],
    },
    {
      id: 'win_chatgpt_private_stream_observer_v1',
      label: '私有流与完成态结算',
      requestMode: 'passive_official_response_clone',
      fallback: 'official_dom_stream_snapshot',
      requiredActions: ['snapshot', 'stop_generation'],
      androidParityIds: [
        'android_chatgpt_private_stream_observer_v1',
        'android_chatgpt_private_stream_completion_settlement_v1',
      ],
    },
    {
      id: 'win_chatgpt_private_stream_send_binding_v1',
      label: '新会话首轮私有流绑定',
      requestMode: 'send_ledger_revision_gated_private_stream_binding',
      fallback: 'official_dom_prompt_confirmation',
      requiredActions: ['snapshot', 'send_prompt'],
    },
    {
      id: 'win_chatgpt_private_rich_turn_settlement_v1',
      label: '富内容异步解压与当前回答结算',
      requestMode: 'observed_widget_generation_and_conversation_bound_settlement',
      fallback: 'captured_response_recovery_and_official_dom',
      requiredActions: ['snapshot', 'send_prompt'],
    },
    {
      id: 'win_chatgpt_private_rich_placeholder_reconciliation_v1',
      label: '富内容占位与真实卡片对账',
      requestMode: 'title_bound_private_rich_placeholder_reconciliation',
      fallback: 'preserve_unrelated_official_interactive_content',
      requiredActions: ['snapshot'],
    },
    {
      id: 'win_chatgpt_realtime_voice_private_transcript_refresh_v1',
      label: '语音结束后转写单飞行刷新',
      requestMode: 'serial_authenticated_same_origin_get_then_snapshot',
      fallback: 'official_dom_snapshot',
      requiredActions: ['invoke_ui_control'],
      androidParityIds: ['android_chatgpt_realtime_voice_private_transcript_refresh_v1'],
    },
    {
      id: 'win_chatgpt_realtime_voice_background_surface_v1',
      label: '后台官网语音与原生控制面连续性',
      requestMode: 'official_webrtc_background_webview_and_native_control_surface',
      fallback: 'official_webview_realtime_voice',
      requiredActions: ['invoke_ui_control'],
      androidParityIds: ['android_chatgpt_realtime_voice_background_overlay_v1'],
    },
    {
      id: 'win_chatgpt_realtime_voice_data_channel_transcript_v1',
      label: '私有语音数据通道状态与实时转写',
      requestMode: 'passive_webview2_official_webrtc_data_channel_state_and_delta_transcript',
      fallback: 'private_conversation_refresh_and_official_dom_snapshot',
      requiredActions: ['invoke_ui_control', 'snapshot'],
      androidParityIds: ['android_chatgpt_realtime_voice_data_channel_transcript_v1'],
    },
    SHARED_CONTINUITY,
    SHARED_REFRESH_SINGLE_FLIGHT,
    SHARED_SEND_COORDINATOR,
    SHARED_CONVERSATION_BODY_CACHE,
    SHARED_NATIVE_STREAM_REFRESH,
    SHARED_PROVIDER_SESSION_PREWARM,
  ],
  'google-ai-mode': [
    {
      id: 'win_google_private_conversation_directory_v1',
      label: '私有会话目录观察',
      requestMode: 'passive_official_response_observer',
      fallback: 'local_directory_cache_and_official_page',
      requiredActions: ['list_conversations', 'open_conversation'],
      androidParityIds: ['android_google_web_private_conversation_directory_v1'],
    },
    {
      id: 'win_google_conversation_snapshot_cache_v1',
      label: '会话快照缓存先显',
      requestMode: 'passive_official_snapshot_cache',
      fallback: 'official_webview_navigation',
      requiredActions: ['open_conversation'],
      androidParityIds: ['android_google_web_conversation_snapshot_cache_v1'],
    },
    {
      id: 'win_google_private_reply_observer_v1',
      label: '回复流完成态观察',
      requestMode: 'passive_completion_signal',
      fallback: 'official_dom_reply_snapshot',
      requiredActions: ['snapshot', 'send_prompt'],
      androidParityIds: ['android_google_web_private_reply_observer_v1'],
    },
    SHARED_CONTINUITY,
    SHARED_REFRESH_SINGLE_FLIGHT,
    SHARED_SEND_COORDINATOR,
    SHARED_CONVERSATION_BODY_CACHE,
    SHARED_NATIVE_STREAM_REFRESH,
    SHARED_PROVIDER_SESSION_PREWARM,
  ],
}

export interface LocalAiAndroidProductionParity {
  androidId: string
  winId: string
}

export interface LocalAiAndroidProductionParityGap {
  androidId: string
  reason: string
}

const ANDROID_PRODUCTION_PARITY_GAPS: readonly LocalAiAndroidProductionParityGap[] = [
  {
    androidId: 'android_chatgpt_interaction_preset_cache_v1',
    reason: 'Win 当前只有会话级内存预设与后台预热；持久预设缓存和单次官网动作对账尚待独立验收。',
  },
  {
    androidId: 'android_chatgpt_web_private_voice_native_relay_v1',
    reason: 'Win 当前由后台 WebView2 持有单一官网 WebRTC；原生媒体所有权仍需独立验收。',
  },
]

/**
 * Executable APK -> Win production parity contract. Android-only presentation
 * concepts (for example an overlay) map to the equivalent desktop surface,
 * never to copied mobile UI code.
 */
export function localAiAndroidProductionParity(): LocalAiAndroidProductionParity[] {
  const pairs = Object.values(CATALOG).flatMap((definitions) => definitions.flatMap(
    (definition) => (definition.androidParityIds ?? []).map((androidId) => ({
      androidId,
      winId: definition.id,
    })),
  ))
  return [...new Map(pairs.map((pair) => [pair.androidId, pair])).values()]
}

/** New APK defaults must be mapped or remain in this explicit, tested gap list. */
export function localAiAndroidProductionParityGaps(): LocalAiAndroidProductionParityGap[] {
  return ANDROID_PRODUCTION_PARITY_GAPS.map((gap) => ({ ...gap }))
}

export function localAiPrivateTransportCapabilities(
  provider: LocalAiWebProvider | undefined,
): LocalAiPrivateTransportCapability[] {
  if (!provider) return []
  const actions = new Set(provider.adapterActions)
  return (CATALOG[provider.id] ?? []).map((definition) => ({
    id: definition.id,
    label: definition.label,
    requestMode: definition.requestMode,
    fallback: definition.fallback,
    runtimeEnabled: (definition.requiredActions ?? []).every((action) => actions.has(action)),
    activation: 'preset_then_background_verify',
  }))
}

export function localAiPrivateTransportStatusCopy(
  provider: LocalAiWebProvider | undefined,
  health?: LocalAiPrivateTransportHealth,
  nowMs = Date.now(),
): { copy: string; detail: string } | null {
  const capabilities = localAiPrivateTransportCapabilities(provider)
  if (!capabilities.length) return null
  const enabled = capabilities.filter((capability) => capability.runtimeEnabled)
  const detail = enabled.map((capability) => capability.label).join('、')
  if (provider?.id === 'chatgpt' && health?.version === 1
      && health.sampledAtMs > 0
      && health.sampledAtMs <= nowMs + 5_000
      && nowMs - health.sampledAtMs <= 2 * 60 * 1000) {
    if (health.cooldownRemainingMs > 0) {
      return {
        copy: `私有会话预取正在冷却（约 ${Math.ceil(health.cooldownRemainingMs / 1000)} 秒，${privateOutcomeCopy(health.lastOutcome)}）；当前立即回退官网，不阻塞聊天。`,
        detail,
      }
    }
    if (health.prefetchReady) {
      const latency = health.privateLatencyMs > 0 ? `，最近约 ${health.privateLatencyMs}ms` : ''
      return {
        copy: `私有会话预取实时可用${latency}；已成功 ${health.successes} 次，失败自动回退官网。`,
        detail,
      }
    }
    if (health.prefetchEnabled) {
      return {
        copy: '私有会话预取已启用，正在后台等待官网会话上下文；当前使用缓存或官网语义，不阻塞输入。',
        detail,
      }
    }
  }
  return {
    copy: `私有加速预设 ${enabled.length}/${capabilities.length} 项已载入；无需等待官网扫描，运行健康在后台异步核验。`,
    detail,
  }
}

function privateOutcomeCopy(outcome: LocalAiPrivateTransportHealth['lastOutcome']): string {
  if (outcome === 'timeout') return '上次请求超时'
  if (outcome === 'auth') return '官网登录状态待恢复'
  if (outcome === 'context') return '官网请求上下文待恢复'
  if (outcome === 'parse' || outcome === 'empty') return '官网响应结构待适配'
  if (outcome === 'http' || outcome === 'network' || outcome === 'official_error') return '官网网络暂不可用'
  return '自动保护'
}
