const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

function compile(relativePath) {
  const target = path.resolve(__dirname, '..', relativePath)
  const result = ts.transpileModule(fs.readFileSync(target, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
    fileName: target,
  }).outputText
  const instance = new Module(target, module)
  instance.filename = target
  instance.paths = module.paths
  instance._compile(result, target)
  return instance
}

const privateStreamSignal = compile('src/features/user-browser/localAiPrivateStreamSignal.ts')

const filename = path.resolve(__dirname, '../src/features/user-browser/localAiProviderActivity.ts')
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
const defaultRequire = compiled.require.bind(compiled)
compiled.require = (id) => id === './localAiPrivateStreamSignal'
  ? privateStreamSignal.exports
  : defaultRequire(id)
compiled._compile(output, filename)
const { initializeLocalAiProviderActivity, updateLocalAiProviderActivity } = compiled.exports

const idle = initializeLocalAiProviderActivity(state({ id: 'answer-1', streaming: false, updatedAt: 10 }))
assert.equal(idle.phase, 'idle')
assert.equal(idle.unread, false, 'existing cached answers must not appear unread on startup')

const streaming = updateLocalAiProviderActivity(
  idle,
  state({ id: 'answer-2', streaming: true, updatedAt: 20 }),
  false,
)
assert.equal(streaming.phase, 'streaming')
assert.equal(streaming.label, '后台回答中')

const completed = updateLocalAiProviderActivity(
  streaming,
  state({ id: 'answer-2', streaming: false, updatedAt: 30 }),
  false,
)
assert.equal(completed.phase, 'completed')
assert.equal(completed.unread, true)
assert.equal(completed.label, '新回答')

const read = updateLocalAiProviderActivity(
  completed,
  state({ id: 'answer-2', streaming: false, updatedAt: 30 }),
  true,
)
assert.equal(read.phase, 'idle')
assert.equal(read.unread, false, 'selecting the provider must mark its answer as read')

const selectedStreaming = updateLocalAiProviderActivity(
  read,
  state({ id: 'answer-3', streaming: true, updatedAt: 40 }),
  true,
)
assert.equal(selectedStreaming.label, '正在回答')
assert.equal(selectedStreaming.unread, false)

const privateCompleted = updateLocalAiProviderActivity(
  selectedStreaming,
  state({ id: 'answer-3', streaming: true, updatedAt: 45, privateStreamState: 'completed' }),
  false,
)
assert.equal(privateCompleted.phase, 'completed', 'private completion must override a stale DOM streaming flag')

const attention = updateLocalAiProviderActivity(
  selectedStreaming,
  { ...state({ id: 'answer-3', streaming: false, updatedAt: 50 }), windowStatus: 'blocked' },
  false,
)
assert.equal(attention.phase, 'attention')
assert.equal(attention.unread, false)

const hookSource = fs.readFileSync(
  path.resolve(__dirname, '../src/features/user-browser/useLocalAiProviderActivity.ts'),
  'utf8',
)
const backendSource = fs.readFileSync(
  path.resolve(__dirname, '../src/features/user-browser/useAiWebChatBackend.ts'),
  'utf8',
)
const sidebarSource = fs.readFileSync(
  path.resolve(__dirname, '../src/features/user-browser/AiWebChatSidebar.tsx'),
  'utf8',
)
assert.match(hookSource, /getLocalAiWebSessionState\(providerId, ownerKey\)/)
assert.match(hookSource, /providerId !== selectedProviderId/)
assert.match(hookSource, /state\?\.providerId === providerId \? state : null/)
assert.match(hookSource, /IDLE_BACKGROUND_DELAY_MS = 8_000/)
assert.doesNotMatch(hookSource, /requestLocalAiWebSnapshot|openLocalAiWebSession/)
assert.match(backendSource, /providerActivities = useLocalAiProviderActivity/)
assert.match(sidebarSource, /providerActivities\[provider\.id\]/)

process.stdout.write('PASS local AI background provider activity contract\n')

function state({ id, streaming, updatedAt, privateStreamState }) {
  return {
    providerId: 'chatgpt',
    windowLabel: 'local-ai-chatgpt-test',
    windowStatus: 'ready',
    windowVisible: false,
    currentUrl: 'https://chatgpt.com/c/test',
    currentHost: 'chatgpt.com',
    loading: false,
    rendererStatus: 'active',
    cacheStatus: 'live',
    semanticCacheStatus: 'live',
    navigationCacheStatus: 'live',
    cacheUpdatedAtMs: updatedAt,
    semanticUpdatedAtMs: updatedAt,
    updatedAtMs: updatedAt,
    semanticEvent: {
      type: 'message_snapshot',
      title: 'test',
      url: 'https://chatgpt.com/c/test',
      draft: '',
      messages: [{ id, role: 'assistant', state: streaming ? 'streaming' : 'completed', content: [] }],
      authenticated: false,
      composerReady: true,
      streaming,
      privateStreamObserved: Boolean(privateStreamState),
      privateStreamRevision: privateStreamState ? 1 : 0,
      privateStreamState: privateStreamState || 'idle',
      currentModel: '',
      capabilities: [],
    },
  }
}
