const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const catalogFilename = path.resolve(
  __dirname,
  '../src/features/user-browser/localAiPrivateTransportCatalog.ts',
)
const catalogSource = fs.readFileSync(catalogFilename, 'utf8')
const output = ts.transpileModule(catalogSource, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
  fileName: catalogFilename,
}).outputText
const compiled = new Module(catalogFilename, module)
compiled.filename = catalogFilename
compiled.paths = module.paths
compiled._compile(output, catalogFilename)

const {
  localAiAndroidProductionParity,
  localAiAndroidProductionParityGaps,
  localAiPrivateRichRecoveryStatusCopy,
  localAiPrivateStreamStatusCopy,
  localAiPrivateTransportCapabilities,
  localAiPrivateTransportStatusCopy,
} = compiled.exports

const chatgpt = provider('chatgpt', [
  'snapshot', 'send_prompt', 'stop_generation', 'list_conversations',
  'open_conversation', 'invoke_ui_control',
  'list_model_options', 'collect_model_options', 'select_model_option',
  'list_composer_tools', 'collect_composer_tools', 'select_composer_tool',
  'list_navigation', 'collect_navigation', 'select_navigation',
])
const google = provider('google-ai-mode', [
  'snapshot', 'send_prompt', 'list_conversations', 'open_conversation',
])

const chatCapabilities = localAiPrivateTransportCapabilities(chatgpt)
assert.equal(chatCapabilities.length, 19)
assert.equal(chatCapabilities.every((capability) => capability.runtimeEnabled), true)
assert.equal(chatCapabilities.every((capability) => (
  capability.activation === 'preset_then_background_verify'
)), true)

const googleCapabilities = localAiPrivateTransportCapabilities(google)
assert.equal(googleCapabilities.length, 9)
assert.equal(googleCapabilities.every((capability) => capability.runtimeEnabled), true)

const incompleteGoogle = localAiPrivateTransportCapabilities(provider('google-ai-mode', [
  'snapshot', 'send_prompt',
]))
assert.equal(incompleteGoogle.filter((capability) => capability.runtimeEnabled).length, 6)

const copy = localAiPrivateTransportStatusCopy(chatgpt)
assert.match(copy.copy, /19\/19/)
assert.match(copy.copy, /无需等待官网扫描/)
assert.match(copy.detail, /私有流与完成态结算/)
assert.match(copy.detail, /同源私有文本写事务与官网回退/)
assert.match(copy.detail, /官网快照单飞行刷新/)
assert.match(copy.detail, /单所有者发送、稳定回执与跨会话隔离/)
assert.match(copy.detail, /独立会话正文与富内容缓存/)
assert.match(copy.detail, /私有流原生事件即时刷新/)
assert.match(copy.detail, /后台导航与宿主恢复连续性/)
assert.match(copy.detail, /游客会话富内容补齐/)
assert.match(copy.detail, /新会话首轮私有流绑定/)
assert.match(copy.detail, /富内容异步解压与当前回答结算/)
assert.match(copy.detail, /富内容占位与真实卡片对账/)
assert.match(copy.detail, /模型、工具与功能预设缓存/)
assert.match(copy.detail, /后台官网语音与原生控制面连续性/)
assert.match(copy.detail, /私有语音数据通道状态与实时转写/)
assert.match(copy.detail, /厂商会话后台预热与切换复用/)
const firstTurnBinding = chatCapabilities.find((capability) => (
  capability.id === 'win_chatgpt_private_stream_send_binding_v1'
))
assert.equal(firstTurnBinding.requestMode, 'send_ledger_revision_gated_private_stream_binding')
assert.equal(firstTurnBinding.fallback, 'official_dom_prompt_confirmation')
const hostContinuity = chatCapabilities.find((capability) => (
  capability.id === 'win_web_ai_background_navigation_continuity_v1'
))
const sendCoordinator = chatCapabilities.find((capability) => (
  capability.id === 'win_web_ai_unified_send_coordinator_v1'
))
assert.equal(
  sendCoordinator.requestMode,
  'stable_request_id_single_owner_generation_gated_official_page_transport',
)
assert.equal(
  hostContinuity.requestMode,
  'preserve_inflight_navigation_and_resume_adapter_snapshot',
)
const richTurnSettlement = chatCapabilities.find((capability) => (
  capability.id === 'win_chatgpt_private_rich_turn_settlement_v1'
))
assert.equal(
  richTurnSettlement.requestMode,
  'observed_widget_generation_and_conversation_bound_settlement',
)
assert.equal(richTurnSettlement.fallback, 'captured_response_recovery_and_official_dom')
const richPlaceholderReconciliation = chatCapabilities.find((capability) => (
  capability.id === 'win_chatgpt_private_rich_placeholder_reconciliation_v1'
))
assert.equal(
  richPlaceholderReconciliation.requestMode,
  'title_bound_private_rich_placeholder_reconciliation',
)
assert.equal(
  richPlaceholderReconciliation.fallback,
  'preserve_unrelated_official_interactive_content',
)
const voiceDataChannel = chatCapabilities.find((capability) => (
  capability.id === 'win_chatgpt_realtime_voice_data_channel_transcript_v1'
))
assert.equal(
  voiceDataChannel.requestMode,
  'passive_webview2_official_webrtc_data_channel_state_and_delta_transcript',
)
assert.equal(
  voiceDataChannel.fallback,
  'private_conversation_refresh_and_official_dom_snapshot',
)

const richAccepted = localAiPrivateRichRecoveryStatusCopy(richRecovery({
  active: true,
  conversationBound: true,
  richKinds: ['finance'],
  acceptedCount: 2,
  placeholderReconciled: true,
  lastOutcome: 'accepted',
}))
assert.match(richAccepted, /已接纳 2 次/)
assert.match(richAccepted, /finance/)
assert.match(richAccepted, /等待回答身份补齐/)
assert.match(richAccepted, /替换官网占位/)

const richRejected = localAiPrivateRichRecoveryStatusCopy(richRecovery({
  rejectedCount: 1,
  lastOutcome: 'stale_generation',
}))
assert.match(richRejected, /旧发送代次/)
assert.match(richRejected, /未串入其他会话/)

const live = localAiPrivateTransportStatusCopy(chatgpt, health({
  prefetchReady: true,
  privateLatencyMs: 428,
  successes: 7,
}), 10_000)
assert.match(live.copy, /实时可用/)
assert.match(live.copy, /428ms/)
assert.match(live.copy, /成功 7 次/)

const cooling = localAiPrivateTransportStatusCopy(chatgpt, health({
  cooldownRemainingMs: 31_100,
  lastOutcome: 'timeout',
}), 10_000)
assert.match(cooling.copy, /约 32 秒/)
assert.match(cooling.copy, /上次请求超时/)
assert.match(cooling.copy, /立即回退官网/)

const awaitingContext = localAiPrivateTransportStatusCopy(chatgpt, health(), 10_000)
assert.match(awaitingContext.copy, /等待官网会话上下文/)
assert.match(awaitingContext.copy, /不阻塞输入/)

const stale = localAiPrivateTransportStatusCopy(chatgpt, health({ sampledAtMs: 1 }), 200_000)
assert.match(stale.copy, /19\/19/)

const androidCatalog = fs.readFileSync(path.resolve(
  __dirname,
  '../../android/app/src/main/kotlin/com/elon/app/chatgptweb/WebAiPrivateTransportCatalog.kt',
), 'utf8')
const androidProductionIds = [...androidCatalog.matchAll(
  /Entry\(\s*id = "([^"]+)"[\s\S]*?productionDefault = (true|false),[\s\S]*?\n\s*\),/g,
)]
  .filter((match) => match[2] === 'true')
  .map((match) => match[1])
const parity = localAiAndroidProductionParity()
const parityGaps = localAiAndroidProductionParityGaps()
assert.equal(androidProductionIds.length, 16)
assert.deepEqual(
  [...new Set([
    ...parity.map((item) => item.androidId),
    ...parityGaps.map((item) => item.androidId),
  ])].sort(),
  [...androidProductionIds].sort(),
  'every APK production private capability must map to Win or remain an explicit tested gap',
)
assert.deepEqual(
  parityGaps.map((item) => item.androidId).sort(),
  [
    'android_chatgpt_web_private_voice_native_relay_v1',
  ],
)
assert.equal(parityGaps.every((item) => item.reason.trim().length > 12), true)
assert.equal(parity.some((item) => (
  item.androidId === 'android_chatgpt_interaction_preset_cache_v1'
  && item.winId === 'win_chatgpt_interaction_preset_cache_v1'
)), true)
assert.equal(parity.some((item) => (
  item.androidId === 'android_chatgpt_realtime_voice_data_channel_transcript_v1'
  && item.winId === 'win_chatgpt_realtime_voice_data_channel_transcript_v1'
)), true)
const winCapabilityIds = new Set([...chatCapabilities, ...googleCapabilities].map((item) => item.id))
for (const mapping of parity) {
  assert.equal(winCapabilityIds.has(mapping.winId), true, `${mapping.androidId} maps to missing ${mapping.winId}`)
}

assert.equal(localAiPrivateStreamStatusCopy(chatgpt, null), null)
assert.equal(localAiPrivateStreamStatusCopy(chatgpt, {
  privateStreamObserved: false,
  privateStreamState: 'streaming',
}), null)
assert.match(localAiPrivateStreamStatusCopy(chatgpt, {
  privateStreamObserved: true,
  privateStreamState: 'streaming',
}), /正在接收本轮内容/)
assert.match(localAiPrivateStreamStatusCopy(google, {
  privateStreamObserved: true,
  privateStreamState: 'completed',
}), /Google AI 模式 私有回复完成信号已到达/)
assert.match(localAiPrivateStreamStatusCopy(google, {
  privateStreamObserved: true,
  privateStreamState: 'idle',
}), /私有回复通道已验证/)

const capabilityHook = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/useLocalAiBrowserCapability.ts',
), 'utf8')
const providerPresets = fs.readFileSync(path.resolve(
  __dirname,
  '../src/features/user-browser/localAiWebProviders.ts',
), 'utf8')
assert.match(capabilityHook, /desktopDetected \? 'ready' : 'desktop_required'/)
assert.match(capabilityHook, /desktopDetected \? localAiWebProviderPresets\(\) : \[\]/)
assert.match(capabilityHook, /void verify\(true\)/)
assert.match(capabilityHook, /if \(!preservePreset\)/)
assert.match(providerPresets, /'google-ai-mode'[\s\S]*?'list_conversations'[\s\S]*?'open_conversation'/)

process.stdout.write('PASS Win private transport preset-first catalog\n')

function provider(id, adapterActions) {
  return {
    id,
    displayName: id === 'chatgpt' ? 'ChatGPT' : 'Google AI 模式',
    adapterActions,
    adapterVersion: 1,
    desktopRuntimeVersion: 2,
  }
}

function health(overrides = {}) {
  return {
    version: 1,
    prefetchEnabled: true,
    prefetchReady: false,
    officialFresh: false,
    cooldownRemainingMs: 0,
    officialLatencyMs: 0,
    privateLatencyMs: 0,
    successes: 0,
    failures: 0,
    consecutiveFailures: 0,
    lastOutcome: 'none',
    attemptBudgetMs: 700,
    sampledAtMs: 10_000,
    ...overrides,
  }
}

function richRecovery(overrides = {}) {
  return {
    version: 1,
    generation: 3,
    active: false,
    detached: false,
    conversationBound: false,
    turnBound: false,
    messageBound: false,
    richKinds: [],
    acceptedCount: 0,
    rejectedCount: 0,
    lastOutcome: 'none',
    placeholderReconciled: false,
    sampledAtMs: 10_000,
    ...overrides,
  }
}
