const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')
const { test } = require('node:test')
function load(file, dependencies) {
  const filename = path.resolve(__dirname, '../src/features/friends/group-ai', file)
  const m = new Module(filename, module)
  m.require = name => { if (!(name in dependencies)) throw new Error(name); return dependencies[name] }
  m._compile(ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020, jsx: ts.JsxEmit.ReactJSX }, fileName: filename,
  }).outputText, filename)
  return m.exports
}
function fixture() {
  const f = { calls: [], executed: 0, receiptFailures: 0, local: true, pending: ['command'] }
  class Model { invalidate() {} async execute(c) { f.executed++; return { ok: true, action: c.action } } }
  f.bridge = load('groupAiControlBridge.ts', {
    uuid: { v4: () => 'worker' },
    '../../../store/auth': { useAuthStore: { getState: () => ({ user: null, token: null }), subscribe() {} } },
    '../../../api/runtime': { cloudBaseUrl: () => 'http://43.139.149.158:8080', isLocalWorkbench: () => f.local },
    '../../shell/desktopShell': { getDesktopInvoke: () => async (command, args) => {
      f.calls.push({ command, args }); if (f.failNative) throw new Error('old_runtime'); return true
    } },
    '../../codex-control/codexControlApi': { postWinEvent: async () => {} },
    '../../../lib/utils': { safeNodeAdminUrl: () => 'http://127.0.0.1:7799' },
    '../../node/localNodeApi': { probeLocalNode() {}, async nodeApi(url, route, options) {
      f.calls.push({ route, options })
      if (route.endsWith('/pending')) return { command_ids: f.pending }
      if (route.endsWith('/claim')) return { command: { action: 'groups' } }
      if (route.endsWith('/receipt') && f.receiptFailures-- > 0) throw new Error('receipt_lost')
      return {}
    } },
    '../socialChatOperations': { socialRequest() {} },
    './groupAiStore': { getGroupAiTask() {}, startGroupAi() {} },
    './groupAiControlModel': { GroupAiControlError: Error, GroupAiControlModel: Model },
  })
  return f
}
test('local main with no cloud login starts hidden worker and never claims group commands', async () => {
  const f = fixture(); await f.bridge.pollGroupAiCommands()
  assert.equal(f.executed, 0)
  assert.equal(f.calls.some(c => c.route?.endsWith('/claim')), false)
  assert.deepEqual(f.calls.find(c => c.command).args, { cloudBaseUrl: 'http://43.139.149.158:8080', nodeBaseUrl: 'http://127.0.0.1:7799' })
})
test('no pending command creates no worker; failed native start never consumes pending command', async () => {
  const f = fixture(); f.pending = []; await f.bridge.pollGroupAiCommands()
  assert.equal(f.calls.some(c => c.command), false)
  f.pending = ['command']; f.failNative = true; await f.bridge.pollGroupAiCommands()
  assert.equal(f.calls.some(c => c.route?.endsWith('/claim')), false)
})
test('only dedicated worker runs production command once even when receipt fails', async () => {
  const f = fixture(); f.receiptFailures = 1
  await f.bridge.pollGroupAiCommands(true)
  assert.equal(f.executed, 1)
  assert.equal(f.calls.filter(c => c.route?.endsWith('/receipt')).length, 2)
  assert.equal(f.calls.some(c => c.command), false)
})
test('worker imports no Shell, syncs cloud auth changes, and cancels polling on unmount', async () => {
  const callbacks = {}, intervals = new Set()
  let effect, saved = 'old', rehydrations = 0, cleared = 0, polls = 0
  global.localStorage = { getItem: () => saved }
  global.window = { addEventListener: (k, v) => { callbacks[k] = v }, removeEventListener: k => { delete callbacks[k] },
    setInterval: f => { intervals.add(f); return f }, clearInterval: f => intervals.delete(f) }
  const { default: Worker } = load('GroupAiWorker.tsx', {
    react: { useEffect: fn => { effect = fn } },
    '../../../store/auth': { useAuthStore: { getState: () => ({ token: null }), setState: () => { cleared++ }, persist: { rehydrate: async () => { rehydrations++ } } } },
    '../../shell/desktopShell': { getDesktopInvoke: () => () => {} },
    '../../codex-control/codexControlApi': { postWinEvent: async () => {} },
    './groupAiControlBridge': { pollGroupAiCommands: async dedicated => { assert.equal(dedicated, true); polls++ } },
  })
  assert.equal(Worker(), null); const cleanup = effect()
  await new Promise(setImmediate)
  assert.equal(polls, 1)
  saved = 'new'; callbacks.storage({ key: 'elon_auth' }); await new Promise(setImmediate)
  assert.equal(rehydrations, 1)
  saved = null; callbacks.storage({ key: null }); await new Promise(setImmediate)
  assert.equal(cleared, 1)
  cleanup(); assert.equal(intervals.size, 0); assert.equal(callbacks.storage, undefined)
})
test('worker has no personal-session, file, clipboard, show or navigation permissions', () => {
  const root = path.resolve(__dirname, '../../desktop-shell/src-tauri')
  const permissions = fs.readFileSync(path.join(root, 'permissions/group-ai-worker-session.toml'), 'utf8')
  assert.match(permissions, /commands.allow = \["list_local_ai_web_providers", "group_ai_web_session"\]/)
  const native = fs.readFileSync(path.join(root, 'src/group_ai_worker.rs'), 'utf8')
  assert.match(native, /\.visible\(false\)/)
  assert.match(native, /\.focused\(false\)/)
  assert.doesNotMatch(native, /\.set_focus\(|\.show\(|\.navigate\(|\.data_directory\(/)
})
