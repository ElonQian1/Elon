import { test } from 'node:test'
import assert from 'node:assert/strict'
import { runWorkflow } from './workflow.mjs'
import { parseArgs } from './run.mjs'

const target = '0.3.70+' + 'a'.repeat(40), old = '0.3.69+' + 'b'.repeat(40)
function fixture({ current = target, desktop = current, state = {}, read = { content_complete: true } } = {}) {
  const calls = [], snapshots = []
  const control = {
    connect: async (...args) => { calls.push(['connect', ...args]); return { base: 'http://127.0.0.1:7799', status: await control.status() } },
    submit: async kind => { calls.push(['submit', kind]); return { kind, action_id: 'win_act_' + 'c'.repeat(32), status: 'queued' } },
    waitAction: async action => { calls.push(['receipt', action.kind]); return { ...action, status: 'succeeded', route: action.kind === 'navigate' ? '/ai' : undefined } },
    waitRelease: async value => { calls.push(['activate', value]); current = value; desktop = value },
    status: async () => ({ release_identity: current, desktop_release_identity: desktop, tauri_available: true, frontend_available: true }),
  }
  const input = { state: { run_id: 'run', target_release: target, ...state }, control, delay: async () => {},
    save: async value => snapshots.push(structuredClone(value)), read: async progress => { calls.push(['read']); await progress({ pages: 1 }); return read } }
  return { input, control, calls, snapshots }
}
test('current release is reused; page actions, actual read and final identity are required', async () => {
  const f = fixture(), value = await runWorkflow(f.input)
  assert.equal(value.status, 'passed'); assert.equal(value.update.mode, 'already_current')
  assert.deepEqual(f.calls.filter(call => call[0] === 'submit').map(call => call[1]), ['reload_page', 'navigate', 'capture_state'])
  assert.equal(value.final_release, target); assert.equal(value.stage, 'finished')
  assert.equal(value.workflow_complete, true)
})

test('update-only repairs a stale desktop without reading, navigating or reloading chats', async () => {
  const f = fixture({ desktop: old, state: { update_only: true } })
  const outcome = await runWorkflow(f.input)
  assert.equal(outcome.status, 'passed')
  assert.equal(outcome.desktop_release, target)
  assert.deepEqual(f.calls.filter(c => c[0] === 'submit').map(c => c[1]), ['update_and_restart'])
  assert.ok(!f.calls.some(c => c[0] === 'read'))
  const args = ['--project-root', '.', '--target-release', target, '--update-only']
  assert.equal(parseArgs(args).reference, null)
  assert.equal(parseArgs(args).updateOnly, true)
  assert.throws(() => parseArgs([...args, '--reference', '00000000-0000-4000-8000-000000000001']))
})
test('update intent is durable before submitting and scheduling is followed by exact activation', async () => {
  const f = fixture({ current: old })
  const submit = f.control.submit
  f.control.submit = async (...args) => {
    if (args[0] === 'update_and_restart') assert.equal(f.snapshots.at(-1).update.phase, 'intent')
    return submit(...args)
  }
  const value = await runWorkflow(f.input)
  assert.equal(value.status, 'passed')
  assert.ok(f.calls.findIndex(c => c[0] === 'activate') < f.calls.findIndex(c => c[0] === 'read'))
})
test('scheduled update without matching release does not open the feature or pass', async () => {
  const f = fixture({ current: old })
  f.control.waitRelease = async () => { throw Error('win_release_timeout') }
  const value = await runWorkflow(f.input)
  assert.equal(value.status, 'failed'); assert.equal(value.error, 'win_release_timeout')
  assert.equal(value.workflow_complete, false)
  assert.ok(!f.calls.some(c => c[0] === 'read'))
  assert.equal(value.update.phase, 'scheduled')
})
test('resume after lost update reply never submits another update or launches over the guard', async () => {
  const f = fixture({ current: old, state: { base: 'http://127.0.0.1:7799', update: { phase: 'intent', mode: 'update_requested' } } })
  assert.equal((await runWorkflow(f.input)).status, 'passed')
  assert.equal(f.calls[0][2].waitOnly, true)
  assert.ok(!f.calls.some(c => c[1] === 'update_and_restart'))
  assert.ok(f.calls.some(c => c[0] === 'activate'))
})
test('ambiguous submission reconciles once; rejection and expired actions remain failures', async () => {
  const f = fixture({ current: old }), original = f.control.submit
  let attempts = 0
  f.control.submit = async (...args) => {
    if (args[0] === 'update_and_restart') { attempts++; throw Error('transport_http_503') }
    return original(...args)
  }
  assert.equal((await runWorkflow(f.input)).status, 'passed'); assert.equal(attempts, 1)
  const g = fixture({ current: old })
  g.control.waitAction = async () => { throw Error('win_action_rejected') }
  assert.equal((await runWorkflow(g.input)).status, 'failed')
  assert.ok(!g.calls.some(c => c[0] === 'activate'))
})
test('login and partial content are explicit, and old success evidence is removed on resume', async () => {
  const f = fixture({ read: { content_complete: false, remaining_gaps: ['unsupported_message_content'] } })
  assert.equal((await runWorkflow(f.input)).status, 'partial')
  f.input.read = async () => { throw Error('login_required') }
  const value = await runWorkflow(f.input)
  assert.equal(value.status, 'user_action_required'); assert.equal(value.read, undefined); assert.equal(value.final_release, undefined)
})
test('wrong final artifact and unverified navigation fail closed', async () => {
  for (const reason of ['artifact', 'route']) {
    const f = fixture()
    if (reason === 'artifact') f.control.status = async () => ({ release_identity: old })
    else f.control.waitAction = async action => ({ ...action, status: 'succeeded', route: '/' })
    assert.equal((await runWorkflow(f.input)).status, 'failed')
  }
})
test('only an exact artifact and authorized conversation input can start a run', () => {
  const args = ['--project-root', '.', '--reference', '00000000-0000-4000-8000-000000000001', '--target-release', target]
  assert.equal(parseArgs(args).targetRelease, target)
  for (const invalid of [args.slice(0, -2), [...args, '--exe', 'anything'], [...args.slice(0, -1), 'latest'], [...args, '--resume', '../escape']]) {
    assert.throws(() => parseArgs(invalid))
  }
})
