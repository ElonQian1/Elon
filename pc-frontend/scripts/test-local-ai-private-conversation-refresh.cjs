const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')

const root = path.resolve(__dirname, '..')
const load = (relative, replacements = {}) => {
  const filename = path.join(root, relative)
  const source = fs.readFileSync(filename, 'utf8')
  const output = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 },
    fileName: filename,
  }).outputText
  const compiled = new Module(filename, module)
  compiled.filename = filename
  compiled.paths = module.paths
  const defaultRequire = compiled.require.bind(compiled)
  compiled.require = (id) => replacements[id]?.exports ?? defaultRequire(id)
  compiled._compile(output, filename)
  return compiled
}

const quality = load('src/features/user-browser/localAiAssistantContentQuality.ts')
const tracking = load('src/features/user-browser/localAiResponseTracking.ts')
const privateStreamSignal = load('src/features/user-browser/localAiPrivateStreamSignal.ts')
const policy = load('src/features/user-browser/localAiPrivateConversationRefreshPolicy.ts', {
  './localAiAssistantContentQuality': quality,
  './localAiResponseTracking': tracking,
  './localAiPrivateStreamSignal': privateStreamSignal,
})
const { shouldRequestLocalAiPrivateConversationRefresh: shouldRefresh } = policy.exports

const base = {
  providerId: 'chatgpt',
  expectedPrompt: '新的问题',
  elapsedMs: 2_000,
  attempted: false,
}
assert.equal(shouldRefresh({ ...base, snapshot: snapshot([]) }), false)
assert.equal(shouldRefresh({ ...base, snapshot: snapshot([user('旧的问题')]) }), false)
assert.equal(shouldRefresh({ ...base, elapsedMs: 500, snapshot: snapshot([user('新的问题')]) }), false)
assert.equal(shouldRefresh({ ...base, snapshot: snapshot([user('新的问题')]) }), true)
assert.equal(shouldRefresh({ ...base, attempted: true, snapshot: snapshot([user('新的问题')]) }), false)
assert.equal(shouldRefresh({ ...base, providerId: 'google-ai-mode', snapshot: snapshot([user('新的问题')]) }), false)
assert.equal(shouldRefresh({
  ...base,
  snapshot: snapshot([user('新的问题'), assistant([{ type: 'markdown', text: '正常正文' }])]),
}), false)
assert.equal(shouldRefresh({
  ...base,
  snapshot: snapshot([
    user('新的问题'),
    assistant([{ type: 'interactive', text: 'Bitcoin (BTC)', kind: 'renderer_upgrade_required' }]),
  ]),
}), true)
assert.equal(shouldRefresh({
  ...base,
  elapsedMs: 2_000,
  snapshot: snapshot([user('新的问题'), { ...assistant([]), state: 'streaming' }], true),
}), false)
assert.equal(shouldRefresh({
  ...base,
  elapsedMs: 6_500,
  snapshot: snapshot([user('新的问题'), { ...assistant([]), state: 'streaming' }], true),
}), true)
assert.equal(shouldRefresh({
  ...base,
  snapshot: {
    ...snapshot([user('新的问题')], true),
    privateStreamObserved: true,
    privateStreamRevision: 9,
    privateStreamState: 'completed',
  },
}), true, 'completed private transport must refresh missing official content without a six-second stall')

function snapshot(messages, streaming = false) {
  return {
    type: 'message_snapshot',
    title: 'test',
    url: 'https://chatgpt.com/c/current-conversation',
    draft: '',
    messages,
    authenticated: false,
    composerReady: true,
    streaming,
    currentModel: '',
    capabilities: [],
  }
}
function user(text) {
  return { id: `u-${text}`, role: 'user', state: 'completed', content: [{ type: 'text', text }] }
}
function assistant(content) {
  return { id: 'a-current', role: 'assistant', state: 'completed', content }
}

console.log('PASS local AI conditional private conversation refresh policy')
