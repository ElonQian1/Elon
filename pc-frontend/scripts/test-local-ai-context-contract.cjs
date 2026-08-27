const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const root = path.resolve(__dirname, '..', '..')
const state = read('desktop-shell/src-tauri/src/local_ai_browser/state.rs')
const shell = read('desktop-shell/src-tauri/src/local_ai_browser.rs')
const snapshotCache = read('desktop-shell/src-tauri/src/local_ai_browser/snapshot_cache.rs')
const context = read('desktop-shell/src-tauri/src/local_ai_browser/state/context.rs')
const merger = read('desktop-shell/src-tauri/src/local_ai_browser/semantic_context.rs')
const chatgptWindow = read('desktop-shell/src-tauri/src/local_ai_browser/semantic_context/chatgpt_window.rs')
const directory = read('desktop-shell/src-tauri/src/local_ai_browser/conversation_directory.rs')
const adapter = read('desktop-shell/src-tauri/src/local_ai_browser/adapter.rs')
const googleAdapter = read('desktop-shell/src-tauri/src/local_ai_browser/google_ai_mode.rs')
const api = read('pc-frontend/src/features/user-browser/localAiBrowserApi.ts')
const controller = read('pc-frontend/src/features/user-browser/useLocalAiWebChatController.ts')
const responseRefresh = read('pc-frontend/src/features/user-browser/useLocalAiResponseRefresh.ts')
const responseRefreshConfigPath = path.join(
  root,
  'pc-frontend/src/features/user-browser/localAiWebChatControllerConfig.ts',
)
const directoryAutoSyncPath = path.join(
  root,
  'pc-frontend/src/features/user-browser/localAiDirectoryAutoSync.ts',
)
const realtimeVoicePath = path.join(
  root,
  'pc-frontend/src/features/user-browser/localAiRealtimeVoice.ts',
)
const realtimeVoiceHangupPath = path.join(
  root,
  'pc-frontend/src/features/user-browser/localAiRealtimeVoiceHangup.ts',
)
const realtimeVoiceActivationPath = path.join(
  root,
  'pc-frontend/src/features/user-browser/localAiRealtimeVoiceActivation.ts',
)
const realtimeVoiceTranscriptRefreshPath = path.join(
  root,
  'pc-frontend/src/features/user-browser/localAiRealtimeVoiceTranscriptRefresh.ts',
)
const realtimeVoiceControl = read('pc-frontend/src/features/user-browser/useLocalAiRealtimeVoiceControl.ts')
const responseTracking = read('pc-frontend/src/features/user-browser/localAiResponseTracking.ts')
const backend = read('pc-frontend/src/features/user-browser/useAiWebChatBackend.ts')
const userState = read('pc-frontend/src/features/user-browser/localAiUserState.ts')
const controls = read('pc-frontend/src/features/user-browser/AiWebComposerControls.tsx')
const sidebar = read('pc-frontend/src/features/user-browser/AiWebChatSidebar.tsx')

assert.match(state, /active_conversation_id/)
assert.match(state, /context_ready/)
assert.match(state, /context_status/)
assert.match(state, /require_bound_context/)
assert.match(state, /record_adapter_event_with_context/)
assert.match(state, /record_adapter_event_with_context_and_url/)
assert.match(state, /mark_command_pending_with_value/)
assert.match(shell, /if action != "snapshot"/)
assert.match(shell, /if action == "send_prompt"[\s\S]*require_bound_context/)
assert.match(shell, /normalize_restorable_url\(provider\.id, cached\.as_str\(\)\)/)
assert.match(context, /preserve_conversation_on_navigation/)
assert.match(context, /context_binding_status/)
assert.match(context, /boundary != "send_prompt"/)
assert.match(context, /pending_send_snapshot_ignored/)
assert.match(merger, /has_last_user_text/)
assert.match(context, /"bound"/)
assert.match(context, /stale_message_snapshot_ignored/)
assert.match(context, /merge_message_snapshot/)
assert.match(merger, /GOOGLE_HISTORY_LIMIT/)
assert.match(chatgptWindow, /HISTORY_LIMIT/)
assert.match(chatgptWindow, /messages_have_stable_ids/)
assert.match(chatgptWindow, /is_position_only_id/)
assert.match(merger, /chatgpt_window::merge/)
assert.match(merger, /same_conversation/)
assert.match(merger, /page_context_key/)
assert.match(adapter, /page_context_key/)
assert.match(googleAdapter, /page_context_key/)
assert.match(snapshotCache, /matches!\(key\.as_ref\(\), "udm" \| "aep"\)/)
assert.doesNotMatch(`${adapter}\n${googleAdapter}`, /page_context_key:\s*Some\(raw_url/)

assert.match(api, /activeConversationId\?:/)
assert.match(api, /contextReady\?:/)
assert.match(api, /contextStatus\?:/)
assert.match(api, /requestLocalAiWebSnapshot/)
assert.match(responseRefresh, /localAiResponseRefreshDelay/)
assert.match(responseRefresh, /localAiResponseRefreshPhase/)
assert.match(responseRefresh, /streaming_watchdog/)
assert.match(responseRefresh, /RESPONSE_COMPLETION_SETTLE_MS/)
assert.match(responseRefresh, /requestLocalAiWebSnapshot/)
assert.match(responseRefresh, /matchingLocalAiUserIndex/)
assert.match(responseRefresh, /baselineMatchingUserCount/)
assert.match(responseTracking, /lastMatchingLocalAiUserIndex/)
assert.match(responseTracking, /matchingLocalAiUserIndex/)
assert.match(responseTracking, /latestLocalAiAssistantForUserTurn/)
assert.doesNotMatch(responseTracking, /findLastIndex/)
assert.match(controller, /requestedSessionIdentity/)
assert.match(controller, /sessionEntry\.identity === requestedSessionIdentity/)
assert.match(backend, /contextTurnCount/)
assert.match(backend, /contextSummary/)
assert.match(backend, /localAiHistoryWindow/)
assert.match(sidebar, /historyWindow\.label/)
assert.match(sidebar, /localAiDirectoryAutoSyncKey/)
assert.match(sidebar, /web\.controller\.sessionIdentity/)
assert.match(controls, /MENU_CACHE_TTL_MS/)
assert.match(controls, /menuNeedsRefresh/)
assert.match(controls, /findLocalAiRealtimeVoiceControls/)
assert.match(controls, /realtimeVoiceControl\.run\('end'/)
assert.match(controls, /'正在连接' : '实时语音'/)
assert.match(controls, /activationStatus === 'confirming'/)
assert.match(controls, /官网实时语音已连接/)
assert.match(controls, /正在确认挂断/)
assert.match(controls, /再次挂断/)
assert.match(controls, /'结束语音'/)
assert.match(backend, /官方网页尚未恢复到对应会话/)
assert.match(userState, /context_restoring/)
assert.match(userState, /缓存会话与当前官方页面不一致/)
assert.match(realtimeVoiceControl, /LocalAiRealtimeVoiceTranscriptRefreshFlight/)
assert.match(realtimeVoiceControl, /requestLocalAiCurrentConversationRefresh/)
assert.match(realtimeVoiceControl, /requestLocalAiWebSnapshot/)
assert.match(realtimeVoiceControl, /controlLocalAiWebSession/)
assert.match(realtimeVoiceControl, /snapshot_ui_manifest/)
assert.match(realtimeVoiceControl, /startActivationConfirmation/)
assert.match(realtimeVoiceControl, /endedOnOfficialSurface/)
assert.match(realtimeVoiceControl, /hangupStatus !== 'confirming'/)
assert.match(controls, /官网语音可能仍在通话，请再次挂断或打开官方页确认/)
assert.match(directory, /official_partial/)
assert.match(directory, /is_project_conversation/)
assert.match(api, /localAiAdapterResultAttempts/)
assert.doesNotMatch(api, /attempt < 12/)

const responseRefreshConfig = loadTypeScriptModule(responseRefreshConfigPath)
assert.deepEqual(responseRefreshConfig.RESPONSE_REFRESH_DELAYS_MS, [400, 800, 1_500, 2_500, 4_000, 6_000, 8_000, 10_000])
assert.deepEqual(responseRefreshConfig.RESPONSE_STREAMING_WATCHDOG_DELAYS_MS, [6_000, 12_000, 20_000, 30_000])
assert.equal(responseRefreshConfig.localAiResponseRefreshDelay('initial', 0), 400)
assert.equal(responseRefreshConfig.localAiResponseRefreshDelay('streaming_watchdog', 0), 6_000)
assert.equal(responseRefreshConfig.localAiResponseRefreshDelay('streaming_watchdog', 4), undefined)
assert.equal(responseRefreshConfig.localAiResponseRefreshDelay('completed', 99), 600)
assert.equal(responseRefreshConfig.localAiResponseRefreshPhase({
  providerId: 'google-ai-mode', current: 'initial', assistantObserved: true,
  streaming: true, completed: false,
}), 'streaming_watchdog')
assert.equal(responseRefreshConfig.localAiResponseRefreshPhase({
  providerId: 'chatgpt', current: 'initial', assistantObserved: true,
  streaming: true, completed: false,
}), 'initial')
assert.equal(responseRefreshConfig.localAiResponseRefreshPhase({
  providerId: 'google-ai-mode', current: 'streaming_watchdog', assistantObserved: true,
  streaming: false, completed: true,
}), 'completed')

const transcriptRefresh = loadTypeScriptModule(realtimeVoiceTranscriptRefreshPath)
assert.deepEqual(
  transcriptRefresh.LOCAL_AI_REALTIME_VOICE_TRANSCRIPT_REFRESH_GAPS_MS,
  [250, 750, 1_500],
)

const { localAiDirectoryAutoSyncKey } = loadTypeScriptModule(directoryAutoSyncPath)
assert.equal(localAiDirectoryAutoSyncKey({
  sessionIdentity: 'chatgpt:owner-a', windowLabel: 'chatgpt-window', sessionOpen: true,
}), 'chatgpt:owner-a:chatgpt-window')
assert.equal(localAiDirectoryAutoSyncKey({
  sessionIdentity: 'chatgpt:owner-b', windowLabel: 'chatgpt-window', sessionOpen: true,
}), 'chatgpt:owner-b:chatgpt-window')
assert.equal(localAiDirectoryAutoSyncKey({
  sessionIdentity: 'chatgpt:owner-a', windowLabel: 'chatgpt-window', sessionOpen: false,
}), '')

const { findLocalAiRealtimeVoiceControls } = loadTypeScriptModule(realtimeVoicePath)
const baseControl = {
  region: 'composer', role: 'button', enabled: true, selected: false,
}
const idleVoice = findLocalAiRealtimeVoiceControls([
  { ...baseControl, id: 'voice-start', semantic: 'voice_mode', label: 'Start voice mode' },
])
assert.equal(idleVoice.start.id, 'voice-start')
assert.equal(idleVoice.active, false)
const activeVoice = findLocalAiRealtimeVoiceControls([
  { ...baseControl, id: 'mute', semantic: 'voice_mute', label: 'Mute microphone', region: 'overlay' },
  { ...baseControl, id: 'end', semantic: 'close', label: 'End voice call', region: 'overlay' },
])
assert.equal(activeVoice.mute.id, 'mute')
assert.equal(activeVoice.end.id, 'end')
assert.equal(activeVoice.active, true)
const genericDialog = findLocalAiRealtimeVoiceControls([
  { ...baseControl, id: 'close', semantic: 'close', label: '关闭', region: 'overlay' },
])
assert.equal(genericDialog.end, undefined)
assert.equal(genericDialog.active, false)

const hangup = loadTypeScriptModule(realtimeVoiceHangupPath)
assert.deepEqual(hangup.LOCAL_AI_REALTIME_VOICE_HANGUP_WATCHDOG_DELAYS_MS, [
  1_000, 1_000, 2_000, 3_000, 5_000, 8_000, 15_000, 25_000, 30_000, 30_000,
])
let hangupObservation = hangup.beginLocalAiRealtimeVoiceHangupObservation()
let hangupResult = hangup.observeLocalAiRealtimeVoiceHangup(hangupObservation, {
  conversationPage: true, manifestHealthy: true, controlsTruncated: false,
  startAvailable: true, voiceActive: false,
}, 1_000)
assert.equal(hangupResult.confirmed, false)
hangupResult = hangup.observeLocalAiRealtimeVoiceHangup(hangupResult.observation, {
  conversationPage: true, manifestHealthy: true, controlsTruncated: false,
  startAvailable: true, voiceActive: false,
}, 3_000)
assert.equal(hangupResult.confirmed, true)
hangupResult = hangup.observeLocalAiRealtimeVoiceHangup(hangupResult.observation, {
  conversationPage: true, manifestHealthy: false, controlsTruncated: false,
  startAvailable: true, voiceActive: false,
}, 4_000)
assert.equal(hangupResult.confirmed, false)
assert.deepEqual(hangupResult.observation, { stableSinceMs: 0, stableObservations: 0 })
assert.equal(hangup.shouldRefreshLocalAiRealtimeVoiceHangupControls(0), true)
assert.equal(hangup.shouldRefreshLocalAiRealtimeVoiceHangupControls(2), false)

const activation = loadTypeScriptModule(realtimeVoiceActivationPath)
assert.deepEqual(activation.LOCAL_AI_REALTIME_VOICE_ACTIVATION_WATCHDOG_DELAYS_MS, [
  500, 1_000, 2_000, 4_000, 8_000, 12_000,
])
assert.equal(activation.localAiRealtimeVoiceActivationConfirmed({
  manifestHealthy: true, controlsTruncated: false, voiceActive: true,
}), true)
assert.equal(activation.localAiRealtimeVoiceActivationConfirmed({
  manifestHealthy: false, controlsTruncated: false, voiceActive: true,
}), false)
assert.equal(activation.localAiRealtimeVoiceActivationConfirmed({
  manifestHealthy: true, controlsTruncated: true, voiceActive: true,
}), false)
assert.equal(activation.shouldRefreshLocalAiRealtimeVoiceActivationControls(0), true)
assert.equal(activation.shouldRefreshLocalAiRealtimeVoiceActivationControls(2), false)

process.stdout.write('PASS local AI stable context and response refresh contract\n')

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), 'utf8')
}

function loadTypeScriptModule(filename) {
  const output = ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
    fileName: filename,
  }).outputText
  const compiled = new Module(filename, module)
  compiled.filename = filename
  compiled.paths = module.paths
  compiled._compile(output, filename)
  return compiled.exports
}
