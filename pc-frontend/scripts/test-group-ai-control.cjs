const assert = require('node:assert/strict')
const fs = require('node:fs')
const Module = require('node:module')
const path = require('node:path')
const ts = require('typescript')
const { test } = require('node:test')
const filename = path.resolve(__dirname, '../src/features/friends/group-ai/groupAiControlModel.ts')
const compiled = new Module(filename, module)
compiled.filename = filename
compiled.paths = module.paths
compiled._compile(ts.transpileModule(fs.readFileSync(filename, 'utf8'), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 }, fileName: filename,
}).outputText, filename)
const { GroupAiControlModel } = compiled.exports
function fixture() {
  const f = { owner: 'owner', count: 0, calls: [], groups: [{ id: 'g', name: 'Fixture' }],
    messages: [{ id: 'image', revision: 2, content: 'Fixture only', created_at: '2026-01-01', attachments: [{ kind: 'image' }] }], task: null }
  f.model = new GroupAiControlModel({ owner: () => f.owner, uuid: () => 'binding-' + (++f.count),
    async read(url) {
      f.calls.push(url)
      if (f.onRead) await f.onRead(url)
      if (url === '/api/me/groups') return { groups: f.groups }
      if (url.endsWith('/ai-sources')) return { group_id: 'g', message_id: 'result', sources: f.messages.filter(m => m.id === 'image') }
      return { messages: f.messages }
    },
    checkIdentity: async () => {}, task: () => f.task,
    start(input, operation) {
      f.calls.push('start')
      f.task = { input, operation, stage: 'preparing', progress: { phase: 'preparing', busy: true }, answer: '',
        dispatched: false, stopped: false, observedMessageCount: 0,
        resume: async () => { f.calls.push('resume') }, cancel: async () => { f.calls.push('cancel') } }
      return f.task
    },
  })
  f.start = { command_id: 'operation', action: 'start', owner_binding: 'binding-1', group_id: 'g',
    message_ids: ['image'], message_revisions: { image: 2 }, question: 'Describe objects', confirmed: true }
  return f
}
test('discovers owned groups/messages without marking read or exposing attachment URLs', async () => {
  const f = fixture()
  const groups = await f.model.execute({ action: 'groups' })
  assert.equal(groups.owner_binding, 'binding-1')
  const messages = await f.model.execute({ action: 'messages', owner_binding: groups.owner_binding, group_id: 'g' })
  assert.equal(messages.messages[0].has_image, true)
  assert.equal(messages.messages[0].revision, 2)
  assert.ok(f.calls.some(v => v.includes('preserve_unread=true')))
  assert.equal(JSON.stringify(messages).includes('url'), false)
  assert.equal(f.task, null)
})
test('same production start receives exact selection/revisions and sharing remains off', async () => {
  const f = fixture()
  await f.model.execute({ action: 'groups' })
  const receipt = await f.model.execute(f.start)
  assert.equal(receipt.task_id, 'operation')
  assert.deepEqual(f.task.input.selection, { message_ids: ['image'], message_revisions: { image: 2 }, question: 'Describe objects', allow_continue: false })
  assert.equal(f.task.input.provider, 'chatgpt')
  assert.equal(receipt.delivery_verified, undefined)
})
test('rejects stale owner and a switch during directory read before dispatch', async () => {
  for (const during of [false, true]) {
    const f = fixture(); await f.model.execute({ action: 'groups' })
    if (during) f.onRead = async () => { f.owner = 'other' }
    else f.owner = 'other'
    await assert.rejects(f.model.execute(f.start), /account_changed|owner_binding_stale/)
    assert.equal(f.task, null)
  }
})
test('rejects recalled, edited, missing or unconfirmed selection and foreign group', async () => {
  for (const kind of ['recalled', 'edited', 'missing', 'unconfirmed', 'foreign']) {
    const f = fixture(); await f.model.execute({ action: 'groups' })
    if (kind === 'recalled') f.messages[0].recalled_at = 'now'
    if (kind === 'edited') f.messages[0].revision = 3
    if (kind === 'missing') f.messages = []
    if (kind === 'unconfirmed') f.start.confirmed = false
    if (kind === 'foreign') f.start.group_id = 'other'
    await assert.rejects(f.model.execute(f.start))
    assert.equal(f.task, null)
  }
})
test('completed requires real group message and matching source record', async () => {
  const f = fixture(); await f.model.execute({ action: 'groups' }); await f.model.execute(f.start)
  Object.assign(f.task, { answer: 'Fixture answer', stage: 'completed', request: { id: 'req', result_message_id: 'result' }, progress: { phase: 'completed', busy: false } })
  const c = { action: 'status', owner_binding: 'binding-1', task_id: 'operation' }
  assert.equal((await f.model.execute(c)).delivery_verified, false)
  f.messages.push({ id: 'result', content: 'Fixture answer' })
  const verified = await f.model.execute(c)
  assert.equal(verified.delivery_verified, true); assert.equal(verified.source_verified, true)
  f.messages[1].recalled_at = 'now'
  assert.equal((await f.model.execute(c)).delivery_verified, false)
})
test('resume cannot initiate a fresh send and status never sends', async () => {
  const f = fixture(); await f.model.execute({ action: 'groups' }); await f.model.execute(f.start)
  const c = { owner_binding: 'binding-1', task_id: 'operation' }
  await f.model.execute({ ...c, action: 'status' })
  await assert.rejects(f.model.execute({ ...c, action: 'resume' }), /resume_requires/)
  f.task.dispatched = true
  await f.model.execute({ ...c, action: 'resume' })
  assert.equal(f.calls.filter(v => v === 'start').length, 1)
  assert.equal(f.calls.filter(v => v === 'resume').length, 1)
})
test('logout binding is invalidated even if the same account logs back in', async () => {
  const f = fixture(); await f.model.execute({ action: 'groups' }); f.model.invalidate()
  await assert.rejects(f.model.execute(f.start), /owner_binding_stale/)
})
