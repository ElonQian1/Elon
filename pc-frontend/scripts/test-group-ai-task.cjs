const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')
const { test } = require('node:test')
const filename = path.resolve(__dirname, '../src/features/friends/group-ai/groupAiTask.ts')
const compiled = new Module(filename, module)
compiled.filename = filename
compiled.paths = module.paths
compiled._compile(ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 }, fileName: filename,
}).outputText, filename)
const { GroupAiTask, groupAiCommandId, groupAiDocumentReady, completedGroupAiReply, groupAiFailureCode } = compiled.exports
const privateReceipts = require('../../android/app/src/main/assets/chatgpt_web_fresh_text_receipts.js')
const input = { owner: 'owner', group: 'group', title: 'Test group', source: 'one', provider: 'chatgpt',
  selection: { message_ids: ['one', 'two'], message_revisions: { one: 1, two: 3 }, question: 'Summarize' } }
const message = (role, text, state = 'completed') => ({ role, state, content: [{ type: 'markdown', text }] })
const snapshot = (messages = []) => ({ type: 'message_snapshot', url: 'https://chatgpt.com/',
  draft: '', composerReady: true, loginRequired: false, streaming: false, messages })
const documentState = s => ({ loading: false, contextReady: true, semanticCacheStatus: 'live',
  currentUrl: 'https://chatgpt.com/?temporary-chat=true', semanticEvent: s, commandResults: [] })

test('host failures retain fixed diagnostics without exporting raw errors', () => {
  assert.equal(groupAiFailureCode('group_worker_not_trusted', 'opening'), 'group_worker_not_trusted')
  assert.equal(groupAiFailureCode({ code: 'upgrade_required' }, 'opening'), 'desktop_upgrade_required')
  assert.equal(groupAiFailureCode('command permission denied for private-owner', 'opening'), 'desktop_permission_denied')
  assert.equal(groupAiFailureCode(new Error('private content https://example.test/?token=secret'), 'opening'), 'group_opening_failed')
})

function fixture(options = {}) {
  let time = 0, sent = false, owner = true
  let request = { id: 'request', group_id: 'group', trigger_message_id: 'one', state: 'prepared',
    prompt: 'Selected fixture question', engine: 'chatgpt_web', dispatch_permit: false, result_message_id: null,
    attachments: options.attachments || [] }
  let receipt
  const calls = []
  const port = {
    now: () => time, wait: async ms => { time += ms },
    checkOwner: () => { if (!owner) throw new Error('Account changed') },
    async upload(operation, received, files, check) {
      check(); calls.push(['upload', files]);
      if (options.failUpload) throw new Error('Attachment incomplete')
      if (options.cancelUpload) await options.cancelUpload()
      check()
    },
    async prepare(operation, received) {
      calls.push(['prepare', operation, received])
      return { ...request, attachments: options.legacyServer ? undefined : request.attachments }
    },
    async action(operation, received, previous, action, content) {
      calls.push([action, content])
      if (action === 'dispatch') {
        request = { ...request, state: 'dispatched', dispatch_permit: !options.noPermit }
        if (options.loseDispatch) throw new Error('Lost dispatch response')
      }
      if (action === 'uncertain' && request.state !== 'completed') request.state = 'indeterminate'
      if (action === 'complete') {
        assert.equal(content, '**Complete answer**')
        if (options.deliveryOffline) { options.deliveryOffline = false; throw new Error('Offline') }
        request = { ...request, state: 'completed', result_message_id: 'group-result' }
        if (options.loseComplete) { options.loseComplete = false; throw new Error('Lost complete response') }
      }
      return { ...request }
    },
    async host(operation, received, action, value, requestId) {
      calls.push([action, value])
      if (action === 'prepare') receipt = { requestId, action: 'private_protocol_probe', ok: true, detail: '{"code":"ready","stage":"ready"}' }
      if (action === 'send_prompt') { assert.equal(value, request.prompt); sent = true; if (options.ownerChange) owner = false }
      if (action === 'open' && options.cancelOnOpen) await options.cancelOnOpen()
      const s = options.loginRequired ? { ...snapshot(), loginRequired: true }
        : sent && !options.noAnswer ? { ...snapshot([message('user', request.prompt), message('assistant', '**Complete answer**')]), streaming: true, privateStreamState: 'completed' }
          : snapshot()
      return { ...documentState(s), commandResult: receipt }
    },
  }
  const task = new GroupAiTask('operation', input, port, () => {})
  return { task, port, calls, options }
}

test('all selected files are confirmed before authorization and send', async () => {
  const f = fixture({ attachments: [{ attachment_id: 'synthetic-image' }] })
  await f.task.start()
  assert.equal(f.task.progress.phase, 'completed')
  assert.ok(f.calls.findIndex(c => c[0] === 'upload') < f.calls.findIndex(c => c[0] === 'dispatch'))
  assert.equal(f.calls.filter(c => c[0] === 'send_prompt').length, 1)
})

test('attachment failure or cancel never falls back to a text-only analysis', async () => {
  for (const cancel of [false, true]) {
    const f = fixture({ attachments: [{ attachment_id: 'synthetic-image' }], failUpload: !cancel })
    if (cancel) f.options.cancelUpload = () => f.task.cancel()
    await f.task.start()
    assert.equal(f.calls.filter(c => ['dispatch', 'send_prompt', 'complete', 'fallback'].includes(c[0])).length, 0)
  }
})

test('private command id is canonical and signed-long safe', () => {
  const ids = new Set(Array.from({ length: 100 }, () => groupAiCommandId(1789900000000)))
  assert.equal(ids.size, 100)
  for (const id of ids) {
    assert.match(id, /^mcp_[0-9a-z]+$/)
    assert.ok(Number.isSafeInteger(parseInt(id.slice(4), 36)))
    assert.ok(BigInt(parseInt(id.slice(4), 36)) < 2n ** 63n)
    assert.equal(privateReceipts.create().admit(id), '')
  }
})
test('requires a live empty isolated document, not a personal conversation or cache', () => {
  const state = documentState(snapshot())
  assert.equal(groupAiDocumentReady(state, 'chatgpt'), true)
  for (const patch of [{ contextReady: false }, { loading: true }, { semanticCacheStatus: 'cached' }, { currentUrl: 'https://chatgpt.com/' },
    { semanticEvent: snapshot([message('user', 'personal')]) }, { semanticEvent: { ...snapshot(), draft: 'unsent' } },
    { semanticEvent: { ...snapshot(), url: 'https://chatgpt.com/c/other' } }]) {
    assert.equal(groupAiDocumentReady({ ...state, ...patch }, 'chatgpt'), false)
  }
})
test('completed private answer bypasses stale global streaming but never accepts a partial answer', () => {
  const s = { ...snapshot([message('user', 'Question'), message('assistant', '**Complete**')]), streaming: true, privateStreamState: 'completed' }
  assert.equal(completedGroupAiReply(s, 'Question'), '**Complete**')
  assert.equal(completedGroupAiReply({ ...s, privateStreamState: 'streaming' }, 'Question'), null)
  assert.equal(completedGroupAiReply({ ...s, messages: [message('user', 'Question'), message('assistant', 'Partial', 'streaming')] }, 'Question'), null)
  assert.equal(completedGroupAiReply(s, 'Other'), null)
  assert.equal(completedGroupAiReply({ ...s, messages: [...s.messages, message('user', 'New question')] }, 'Question'), null)
})
test('preserves code formatting in the delivered reply', () => {
  const s = snapshot([message('user', 'Q'), { role: 'assistant', state: 'completed', content: [{ type: 'code', language: 'js', text: 'return 1' }] }])
  assert.equal(completedGroupAiReply(s, 'Q'), '```js\nreturn 1\n```')
})
test('selected revisions dispatch once and completed answer is delivered to the same group', async () => {
  const { task, calls } = fixture()
  await task.start()
  assert.equal(task.progress.phase, 'completed')
  assert.deepEqual(calls.find(c => c[0] === 'prepare' && c[2])?.[2].selection, input.selection)
  for (const action of ['dispatch', 'send_prompt', 'complete', 'close']) assert.equal(calls.filter(c => c[0] === action).length, 1)
})
test('lost dispatch permission never replays prompt when checking status', async () => {
  const { task, calls } = fixture({ loseDispatch: true, noAnswer: true })
  await task.start(); await task.resume()
  assert.equal(task.progress.phase, 'uncertain')
  assert.equal(calls.filter(c => c[0] === 'dispatch').length, 1)
  assert.equal(calls.filter(c => c[0] === 'send_prompt').length, 0)
})
test('denied dispatch permission does not send', async () => {
  const { task, calls } = fixture({ noPermit: true })
  await task.start()
  assert.equal(task.progress.phase, 'uncertain')
  assert.equal(calls.filter(c => c[0] === 'send_prompt').length, 0)
})
test('delivery failure retries the answer only, never asks AI twice', async () => {
  const { task, calls } = fixture({ deliveryOffline: true })
  await task.start()
  assert.equal(task.progress.hasAnswer, true)
  await task.resume()
  assert.equal(task.progress.phase, 'completed')
  assert.equal(calls.filter(c => c[0] === 'send_prompt').length, 1)
  assert.equal(calls.filter(c => c[0] === 'complete').length, 2)
})
test('lost completion response is reconciled through server status without redelivery', async () => {
  const { task, calls } = fixture({ loseComplete: true })
  await task.start(); await task.resume()
  assert.equal(task.progress.phase, 'completed')
  assert.equal(calls.filter(c => c[0] === 'complete').length, 1)
  assert.equal(calls.filter(c => c[0] === 'send_prompt').length, 1)
})
test('account change closes the task and prevents delivery', async () => {
  const { task, calls } = fixture({ ownerChange: true })
  await task.start()
  assert.equal(task.progress.phase, 'cancelled')
  assert.equal(calls.filter(c => c[0] === 'complete').length, 0)
  assert.ok(calls.some(c => c[0] === 'close'))
})
test('cancellation during preparation cannot dispatch after delayed open', async () => {
  const f = fixture()
  f.options.cancelOnOpen = () => f.task.cancel()
  await f.task.start()
  assert.equal(f.task.progress.phase, 'cancelled')
  assert.equal(f.calls.filter(c => c[0] === 'dispatch').length, 0)
  assert.ok(f.calls.some(c => c[0] === 'close'))
})
test('login is a recoverable preparation failure, not unsupported capability', async () => {
  const { task, calls } = fixture({ loginRequired: true })
  await task.start()
  assert.equal(task.progress.phase, 'failed')
  assert.match(task.progress.message, /登录/)
  assert.equal(calls.filter(c => c[0] === 'dispatch').length, 0)
})
test('concurrent starts are single-flight', async () => {
  const { task, calls } = fixture()
  await Promise.all([task.start(), task.start()])
  assert.equal(calls.filter(c => c[0] === 'dispatch').length, 1)
})

test('a failed attachment attempt closes its draft before retrying in a fresh host', async () => {
  const f = fixture({ attachments: [{ name: 'fixture.png' }], failUpload: true })
  await f.task.start()
  assert.equal(f.task.progress.phase, 'failed')
  assert.equal(f.calls.filter(c => c[0] === 'close').length, 1)
  assert.equal(f.calls.filter(c => c[0] === 'send_prompt').length, 0)
  f.options.failUpload = false
  await f.task.resume()
  assert.equal(f.task.progress.phase, 'completed')
  assert.equal(f.calls.filter(c => c[0] === 'open').length, 2)
  assert.equal(f.calls.filter(c => c[0] === 'send_prompt').length, 1)
})

test('an old server cannot silently turn a selected attachment request into text only', async () => {
  const f = fixture({ legacyServer: true })
  await f.task.start()
  assert.equal(f.task.progress.phase, 'failed')
  assert.match(f.task.progress.message, /服务器尚未支持附件清单/)
  assert.equal(f.calls.filter(c => ['open', 'dispatch', 'send_prompt'].includes(c[0])).length, 0)
})
