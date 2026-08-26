const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(__dirname, '../src/features/user-browser/localAiNewConversation.ts')
const source = fs.readFileSync(filename, 'utf8')
const controllerSource = fs.readFileSync(
  path.resolve(__dirname, '../src/features/user-browser/useLocalAiWebChatController.ts'),
  'utf8',
)
const welcomeSource = fs.readFileSync(
  path.resolve(__dirname, '../src/features/ai/AiChatWelcome.tsx'),
  'utf8',
)
const foregroundSource = fs.readFileSync(
  path.resolve(__dirname, '../src/features/user-browser/localAiNewConversationForeground.ts'),
  'utf8',
)
const chatGptRecoverySource = fs.readFileSync(
  path.resolve(__dirname, '../src/features/user-browser/useChatGptNewConversationRecovery.ts'),
  'utf8',
)
const controllerConfigSource = fs.readFileSync(
  path.resolve(__dirname, '../src/features/user-browser/localAiWebChatControllerConfig.ts'),
  'utf8',
)
const output = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.CommonJS,
    target: ts.ScriptTarget.ES2020,
    esModuleInterop: true,
  },
  fileName: filename,
}).outputText
const compiled = new Module(filename, module)
compiled.filename = filename
compiled.paths = module.paths
compiled._compile(output, filename)

const {
  chatGptNewConversationRecoveryAction,
  chatGptNewConversationResetControlAction,
  googleNewConversationNeedsReload,
  localAiNewConversationCanDispatchQueuedSend,
  localAiNewConversationContextReady,
  localAiNewConversationNativeReady,
  selectLocalAiNewConversationPath,
} = compiled.exports

assert.equal(localAiNewConversationCanDispatchQueuedSend('chatgpt', false, true), true)
assert.equal(localAiNewConversationCanDispatchQueuedSend('chatgpt', false, false), false)
assert.equal(localAiNewConversationCanDispatchQueuedSend('google-ai-mode', false, true), false)
assert.equal(localAiNewConversationCanDispatchQueuedSend('google-ai-mode', true, false), true)

assert.equal(
  chatGptNewConversationResetControlAction('https://chatgpt.com/'),
  'new_conversation_reload',
)
assert.equal(
  chatGptNewConversationResetControlAction('https://chatgpt.com/?temporary-chat=true'),
  'new_conversation_reload',
)
assert.equal(
  chatGptNewConversationResetControlAction('https://chatgpt.com/c/old-conversation'),
  'new_conversation_home',
)
assert.equal(
  chatGptNewConversationResetControlAction('not a url'),
  'new_conversation_home',
)
assert.equal(chatGptNewConversationResetControlAction(null), 'new_conversation_home')

const liveSession = {
  windowStatus: 'ready',
  loading: false,
  rendererStatus: 'active',
  semanticCacheStatus: 'live',
  contextReady: true,
}
const liveSnapshot = { composerReady: true }

assert.equal(selectLocalAiNewConversationPath('chatgpt', null, null), 'home')
assert.equal(selectLocalAiNewConversationPath('chatgpt', {
  ...liveSession,
  rendererStatus: 'connecting',
  semanticCacheStatus: 'cached',
  contextReady: false,
}, { composerReady: false }), 'home')
assert.equal(selectLocalAiNewConversationPath('chatgpt', {
  ...liveSession,
  contextReady: false,
}, liveSnapshot), 'adapter')
assert.equal(selectLocalAiNewConversationPath('chatgpt', {
  ...liveSession,
  semanticCacheStatus: 'cached',
}, liveSnapshot), 'adapter')
assert.equal(selectLocalAiNewConversationPath('chatgpt', {
  ...liveSession,
  loading: true,
}, null), 'adapter')
assert.equal(selectLocalAiNewConversationPath('chatgpt', liveSession, { composerReady: false }), 'adapter')
assert.equal(selectLocalAiNewConversationPath('chatgpt', liveSession, liveSnapshot), 'adapter')
assert.equal(selectLocalAiNewConversationPath('google-ai-mode', liveSession, liveSnapshot), 'adapter')
assert.equal(selectLocalAiNewConversationPath('google-ai-mode', {
  ...liveSession,
  semanticCacheStatus: 'cached',
  contextReady: false,
}, { composerReady: false }), 'adapter')
assert.equal(selectLocalAiNewConversationPath('google-ai-mode', {
  ...liveSession,
  semanticCacheStatus: 'cached',
  contextReady: false,
}, liveSnapshot), 'adapter')
assert.equal(googleNewConversationNeedsReload(liveSession, liveSnapshot), false)
assert.equal(googleNewConversationNeedsReload({
  ...liveSession,
  semanticCacheStatus: 'cached',
}, liveSnapshot), true)
assert.equal(googleNewConversationNeedsReload(liveSession, { composerReady: false }), true)
const bindingSession = {
  ...liveSession,
  activeConversationId: 'new-conversation',
  cacheUpdatedAtMs: 2_000,
  semanticUpdatedAtMs: 2_000,
  updatedAtMs: 2_000,
}
assert.equal(localAiNewConversationContextReady(
  bindingSession,
  { messages: [] },
  1_000,
  'old-conversation',
), true)
assert.equal(localAiNewConversationContextReady(
  { ...bindingSession, activeConversationId: 'old-conversation' },
  { messages: [] },
  1_000,
  'old-conversation',
), false)
assert.equal(localAiNewConversationContextReady(
  { ...bindingSession, updatedAtMs: 999 },
  { messages: [] },
  1_000,
  'old-conversation',
), false)
assert.equal(localAiNewConversationContextReady(
  { ...bindingSession, cacheUpdatedAtMs: 999, updatedAtMs: 2_000 },
  { messages: [] },
  1_000,
  'old-conversation',
), false)
assert.equal(localAiNewConversationContextReady(
  { ...bindingSession, cacheUpdatedAtMs: 2_000, semanticUpdatedAtMs: 999, updatedAtMs: 2_000 },
  { messages: [] },
  1_000,
  'old-conversation',
), false)
assert.equal(localAiNewConversationContextReady(
  bindingSession,
  null,
  1_000,
  'old-conversation',
), false)
assert.equal(localAiNewConversationNativeReady(
  bindingSession,
  { messages: [], composerReady: false, authenticated: false, loginRequired: false },
  1_000,
  'old-conversation',
), false)
assert.equal(chatGptNewConversationRecoveryAction(
  { ...bindingSession, currentUrl: 'https://chatgpt.com/c/old-conversation' },
  { messages: [], composerReady: false, authenticated: false, loginRequired: false },
  1_000,
  'old-conversation',
  5_000,
), 'home')
assert.equal(chatGptNewConversationRecoveryAction(
  { ...bindingSession, currentUrl: 'https://chatgpt.com/' },
  { messages: [], composerReady: false, authenticated: false, loginRequired: false },
  1_000,
  'old-conversation',
  5_000,
), 'reload')
assert.equal(chatGptNewConversationRecoveryAction(
  { ...bindingSession, currentUrl: 'https://chatgpt.com/' },
  { messages: [], composerReady: true, authenticated: false, loginRequired: false },
  1_000,
  'old-conversation',
  2_800,
), null)
assert.equal(localAiNewConversationNativeReady(
  bindingSession,
  { messages: [], composerReady: true, authenticated: false, loginRequired: false },
  1_000,
  'old-conversation',
  2_500,
), false)
assert.equal(localAiNewConversationNativeReady(
  bindingSession,
  { messages: [], composerReady: true, authenticated: false, loginRequired: false },
  1_000,
  'old-conversation',
  2_800,
), true)
assert.equal(localAiNewConversationNativeReady(
  { ...bindingSession, loading: true },
  { messages: [], composerReady: true, authenticated: false, loginRequired: false },
  1_000,
  'old-conversation',
), false)
assert.equal(localAiNewConversationNativeReady(
  bindingSession,
  { messages: [], composerReady: true, authenticated: false, loginRequired: true },
  1_000,
  'old-conversation',
), false)
assert.match(controllerSource, /GOOGLE_NEW_CONVERSATION_RELOAD_DELAY_MS/)
assert.match(controllerSource, /providerId !== 'google-ai-mode'/)
assert.match(controllerSource, /googleNewConversationNeedsReload\(current, currentSnapshot\)/)
assert.match(controllerSource, /controlLocalAiWebSession\(providerId, ownerKey, 'reload'\)/)
assert.match(controllerSource, /useChatGptNewConversationRecovery\(/)
assert.match(chatGptRecoverySource, /CHATGPT_NEW_CONVERSATION_RECOVERY_DELAYS_MS/)
assert.match(chatGptRecoverySource, /providerId !== 'chatgpt'/)
assert.match(chatGptRecoverySource, /chatGptNewConversationRecoveryAction\(/)
assert.match(chatGptRecoverySource, /chatGptNewConversationResetControlAction/)
assert.match(chatGptRecoverySource, /runLocalAiWebAdapterCommand\([\s\S]*?'new_conversation'/)
assert.match(chatGptRecoverySource, /waitForLocalAiAdapterResult\([\s\S]*?'new_conversation'/)
assert.match(chatGptRecoverySource, /callbacks\.current\.onPageBoundaryConfirmed\(\)/)
assert.match(chatGptRecoverySource, /chatGptNewConversationResetControlAction\(current\.currentUrl\)/)
assert.match(chatGptRecoverySource, /current\.loading \|\| current\.rendererStatus !== 'active'/)
assert.match(chatGptRecoverySource, /if \(!startedAtMs \|\| providerId !== 'chatgpt' \|\| !ownerKey \|\| suspended\) return/)
assert.match(chatGptRecoverySource, /CHATGPT_NEW_CONVERSATION_RECOVERY_DELAYS_MS[\s\S]*?\.filter[\s\S]*?\.map/)
assert.match(controllerConfigSource, /\[6_000, 12_000, 18_000\]/)
assert.match(controllerSource, /if \(action === 'new_conversation'\) \{\s*return startNewConversation\(\)/)
assert.match(controllerSource, /function beginLocalNewConversation\(\)/)
assert.match(controllerSource, /setNewConversationRecoveryStartedAtMs\(Date\.now\(\)\)/)
assert.match(controllerSource, /newConversationBaselineId\.current = visibleSessionState\?\.activeConversationId \?\? ''/)
assert.match(controllerSource, /visibleSessionState\?\.semanticConversationAligned === false/)
assert.match(controllerSource, /deriveLocalAiUserState\(clientState, provider, visibleSessionState, liveSnapshot\)/)
assert.match(controllerSource, /localAiNewConversationNativeReady\(/)
assert.match(controllerSource, /localAiNewConversationCanDispatchQueuedSend\(/)
assert.match(controllerSource, /newConversationPageConfirmed/)
assert.match(controllerSource, /setNewConversationPageConfirmed\(true\)/)
assert.match(controllerSource, /onPageBoundaryConfirmed: \(\) => setNewConversationPageConfirmed\(true\)/)
assert.match(controllerSource, /const path = selectLocalAiNewConversationPath\(/)
assert.match(controllerSource, /if \(path === 'adapter'\)/)
assert.match(controllerSource, /waitForLocalAiAdapterResult\([\s\S]*?'new_conversation'/)
assert.match(controllerSource, /return openNewConversationHome\(/)
assert.match(
  controllerSource,
  /startNewConversation[\s\S]*?requestLocalAiNewConversationNativeForeground\(provider, ownerKey\)[\s\S]*?runLocalAiWebAdapterCommand/,
)
assert.match(
  controllerSource,
  /result\?\.action !== 'new_conversation' \|\| result\.ok[\s\S]*?keepLocalAiNewConversationInNativeForeground\(provider, ownerKey, next\)/,
)
assert.match(
  foregroundSource,
  /requestReturnToAiChat\([\s\S]*?controlLocalAiWebSession\(provider\.id, ownerKey, 'background'\)/,
)
assert.match(
  controllerSource,
  /provider\.id === 'chatgpt'[\s\S]*?chatGptNewConversationResetControlAction\(visibleSessionState\?\.currentUrl\)[\s\S]*?: 'home'[\s\S]*?keepLocalAiNewConversationInNativeForeground\(provider, ownerKey, next\)/,
)
assert.doesNotMatch(
  controllerSource,
  /action === 'new_conversation'[\s\S]{0,160}selectLocalAiNewConversationPath[\s\S]{0,80}recoverNewConversation/,
)
assert.match(controllerSource, /消息已保存在本机新会话队列/)
assert.match(controllerSource, /dispatchPreparedPrompt\(queuedSend\)/)
assert.match(controllerSource, /restoreQueuedSend\(queuedSend\)/)
assert.match(welcomeSource, /!web\.controller\.newConversationRecoveryActive/)

process.stdout.write('PASS local AI new-conversation recovery policy\n')
