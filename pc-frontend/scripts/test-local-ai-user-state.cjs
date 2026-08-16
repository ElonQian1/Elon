const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const filename = path.resolve(__dirname, '../src/features/user-browser/localAiUserState.ts')
const source = fs.readFileSync(filename, 'utf8')
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
const { deriveLocalAiUserState } = compiled.exports

const sharedActions = ['snapshot', 'send_prompt', 'stop_generation', 'new_conversation']
const chatgpt = provider('chatgpt', 'manual_web', [
  ...sharedActions,
  'list_conversations',
  'start_google_login',
])
const google = provider('google-ai-mode', 'guest_web_system_login', sharedActions)
const readySession = {
  providerId: 'chatgpt',
  windowLabel: 'local-ai-chatgpt-test',
  windowStatus: 'ready',
  windowVisible: false,
  currentUrl: '',
  currentHost: 'chatgpt.com',
  loading: false,
  rendererStatus: 'active',
  cacheStatus: 'live',
  semanticCacheStatus: 'live',
  navigationCacheStatus: 'empty',
  contextReady: true,
  cacheUpdatedAtMs: 1,
  updatedAtMs: 1,
}

assert.equal(deriveLocalAiUserState('checking', chatgpt, null, null).phase, 'client_checking')
assert.equal(deriveLocalAiUserState('upgrade_required', chatgpt, null, null).phase, 'client_unavailable')
assert.equal(deriveLocalAiUserState('ready', chatgpt, null, null).phase, 'official_closed')

const login = deriveLocalAiUserState('ready', chatgpt, readySession, snapshot({
  authenticated: false,
  loginRequired: true,
  pageKind: 'auth',
}))
assert.equal(login.phase, 'login_required')
assert.equal(login.canSend, false)
assert.equal(login.canStartGoogleLogin, true)

const unavailable = deriveLocalAiUserState('ready', google, {
  ...readySession,
  providerId: google.id,
  currentHost: 'google.com',
}, snapshot({ pageKind: 'unsupported' }))
assert.equal(unavailable.phase, 'provider_unavailable')
assert.equal(unavailable.degraded, true)
assert.equal(unavailable.fallbackRecommended, true)

const guest = deriveLocalAiUserState('ready', google, {
  ...readySession,
  providerId: google.id,
  currentHost: 'google.com',
}, snapshot({
  composerReady: true,
  pageKind: 'ai_mode',
  capabilities: ['citations', 'new_conversation'],
}))
assert.equal(guest.phase, 'ready_guest')
assert.equal(guest.canSend, true)
assert.equal(guest.canNewConversation, true)
assert.equal(guest.canConversationHistory, false)

const authenticated = deriveLocalAiUserState('ready', chatgpt, readySession, snapshot({
  authenticated: true,
  composerReady: true,
  pageKind: 'conversation',
  capabilities: ['new_conversation', 'conversation_history'],
}))
assert.equal(authenticated.phase, 'ready_authenticated')
assert.equal(authenticated.canSend, true)
assert.equal(authenticated.canConversationHistory, true)
assert.equal(authenticated.canStartGoogleLogin, false)

const cached = deriveLocalAiUserState('ready', chatgpt, {
  ...readySession,
  cacheStatus: 'cached',
  semanticCacheStatus: 'cached',
}, snapshot({
  authenticated: true,
  composerReady: true,
  pageKind: 'conversation',
  capabilities: ['new_conversation', 'conversation_history'],
}))
assert.equal(cached.canSend, false, 'cached semantic state must never unlock live writes')
assert.equal(cached.canConversationHistory, false)

const switchingContext = deriveLocalAiUserState('ready', chatgpt, {
  ...readySession,
  contextReady: false,
}, snapshot({
  authenticated: true,
  composerReady: true,
  pageKind: 'conversation',
  capabilities: ['new_conversation', 'conversation_history'],
}))
assert.equal(switchingContext.canSend, false, 'conversation transition must block writes until the target context is live')

const chatgptGuest = deriveLocalAiUserState('ready', chatgpt, readySession, snapshot({
  composerReady: true,
  pageKind: 'home',
  capabilities: ['new_conversation'],
}))
assert.equal(chatgptGuest.phase, 'ready_guest')
assert.equal(chatgptGuest.canSend, true)
assert.equal(chatgptGuest.canConversationHistory, false)

const streaming = deriveLocalAiUserState('ready', chatgpt, readySession, snapshot({
  authenticated: true,
  composerReady: true,
  streaming: true,
  pageKind: 'conversation',
  capabilities: ['new_conversation', 'conversation_history'],
}))
assert.equal(streaming.phase, 'streaming')
assert.equal(streaming.canStop, true)

process.stdout.write('PASS local AI user-state and capability degradation matrix\n')

function provider(id, loginMode, adapterActions) {
  return {
    id,
    displayName: id === 'chatgpt' ? 'ChatGPT' : 'Google AI 模式',
    startHost: id === 'chatgpt' ? 'chatgpt.com' : 'google.com/aimode',
    loginMode,
    profileScope: 'local_owner_provider',
    rendererProtocol: 'yilong.ai.ui.v1',
    rendererStatus: 'active',
    adapterActions,
  }
}

function snapshot(overrides = {}) {
  return {
    type: 'message_snapshot',
    title: '',
    url: '',
    draft: '',
    messages: [],
    authenticated: false,
    composerReady: false,
    streaming: false,
    currentModel: '',
    capabilities: [],
    ...overrides,
  }
}
